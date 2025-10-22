//! Memory allocator implementations for the Onix operating system.
//!
//! This crate provides efficient memory allocators designed for kernel-space
//! usage, supporting both general-purpose and specialized allocation patterns.
//! The allocators are `no_std` compatible and designed to work in bare-metal
//! environments.
//!
//! # Available Allocators
//!
//! ## [`LinkedListAllocator`](linked_list::LinkedListAllocator)
//!
//! A general-purpose allocator that maintains a linked list of free memory
//! blocks. Best suited for:
//!
//! - Variable-sized allocations
//! - Low memory overhead requirements
//! - Situations where allocation patterns are unpredictable
//!
//! **Performance**: O(n) allocation and deallocation where n is the number of
//! free blocks.
//!
//! ## [`FixedSizeBlockAllocator`](fixed_size_block::FixedSizeBlockAllocator)
//!
//! A specialized allocator optimized for common allocation sizes. Best suited
//! for:
//!
//! - Frequently allocated small objects (≤ 2048 bytes)
//! - Performance-critical code paths
//! - Reducing memory fragmentation for common sizes
//!
//! **Performance**: O(1) allocation and deallocation for supported block sizes,
//! falls back to linked list allocation for larger sizes.
//!
//! # Usage Examples
//!
//! ## Basic `LinkedListAllocator` Usage
//!
//! ```rust
//! use core::alloc::Layout;
//!
//! use allocator::linked_list::LinkedListAllocator;
//!
//! // Create allocator and add heap memory
//! let mut allocator = LinkedListAllocator::new();
//! let mut heap = vec![0u8; 4096]; // In kernel, this would be actual heap memory
//! unsafe {
//!     allocator.add_heap(heap.as_mut_ptr(), heap.len());
//! }
//!
//! // Allocate memory
//! let layout = Layout::from_size_align(64, 8).unwrap();
//! if let Some(ptr) = allocator.allocate(layout) {
//!     // Use the allocated memory...
//!
//!     // Free the memory when done
//!     unsafe {
//!         allocator.deallocate(ptr, layout);
//!     }
//! }
//! ```
//!
//! ## Basic `FixedSizeBlockAllocator` Usage
//!
//! ```rust
//! use core::alloc::Layout;
//!
//! use allocator::fixed_size_block::FixedSizeBlockAllocator;
//!
//! // Create allocator and add heap memory
//! let mut allocator = FixedSizeBlockAllocator::new();
//! let mut heap = vec![0u8; 8192];
//! unsafe {
//!     allocator.add_heap(heap.as_mut_ptr(), heap.len());
//! }
//!
//! // Small allocations use fixed-size blocks (very fast)
//! let small_layout = Layout::from_size_align(64, 8).unwrap();
//! if let Some(ptr) = allocator.allocate(small_layout) {
//!     unsafe {
//!         allocator.deallocate(ptr, small_layout);
//!     }
//! }
//!
//! // Large allocations use fallback linked list allocator
//! let large_layout = Layout::from_size_align(4096, 8).unwrap();
//! if let Some(ptr) = allocator.allocate(large_layout) {
//!     unsafe {
//!         allocator.deallocate(ptr, large_layout);
//!     }
//! }
//! ```
//!
//! # Design Considerations
//!
//! ## Memory Safety
//!
//! All allocators in this crate require `unsafe` code for heap management and
//! deallocation operations. Users must ensure:
//!
//! - Heap regions are valid and exclusive to the allocator
//! - Deallocation uses the exact same layout as allocation
//! - No use-after-free or double-free bugs
//!
//! ## Thread Safety
//!
//! The allocators are `Send` but not `Sync`. They can be moved between threads
//! but require external synchronization (e.g., mutexes) for concurrent access.
//!
//! ## Performance Characteristics
//!
//! | Allocator | Allocation | Deallocation | Memory Overhead | Best Use Case |
//! |-----------|------------|--------------|-----------------|---------------|
//! | `LinkedListAllocator` | O(n) | O(n) | 16 bytes/block | General purpose |
//! | `FixedSizeBlockAllocator` | O(1)* | O(1)* | Variable | Small objects |
//!
//! *For supported block sizes (8-2048 bytes)
//!
//! ## Integration with Global Allocator
//!
//! These allocators can be wrapped to implement Rust's `GlobalAlloc` trait
//! for use as the system allocator:
//!
//! ```rust,ignore
//! use core::alloc::{GlobalAlloc, Layout};
//! use allocator::fixed_size_block::FixedSizeBlockAllocator;
//!
//! struct KernelAllocator {
//!     inner: spin::Mutex<FixedSizeBlockAllocator>,
//! }
//!
//! unsafe impl GlobalAlloc for KernelAllocator {
//!     unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
//!         self.inner.lock()
//!             .allocate(layout)
//!             .unwrap_or(core::ptr::null_mut())
//!     }
//!
//!     unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
//!         self.inner.lock().deallocate(ptr, layout);
//!     }
//! }
//! ```

#![no_std]
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

pub mod fixed_size_block;
pub mod linked_list;
