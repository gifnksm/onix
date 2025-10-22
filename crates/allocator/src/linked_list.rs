//! Linked list allocator implementation.
//!
//! This module provides a general-purpose memory allocator that maintains a
//! linked list of free memory blocks. The allocator supports dynamic heap
//! regions, arbitrary allocation sizes and alignments, and automatic
//! coalescing of adjacent free blocks to minimize fragmentation.
//!
//! # Algorithm
//!
//! The allocator uses a **first-fit** allocation strategy combined with an
//! **address-ordered free list**:
//!
//! - **Free List**: Maintains free memory blocks in a singly-linked list sorted
//!   by memory address
//! - **Allocation**: Searches the free list from the beginning for the first
//!   block that can satisfy the size and alignment requirements
//! - **Deallocation**: Inserts freed blocks back into the address-ordered list
//!   and automatically merges adjacent blocks
//! - **Coalescing**: Adjacent free blocks are merged immediately during
//!   deallocation to reduce fragmentation
//!
//! # Memory Layout
//!
//! Each free block contains a [`ListNode`] header at its beginning, which
//! stores the block size and a pointer to the next free block. The header
//! has 16-byte alignment to ensure compatibility with various data types.
//!
//! ```text
//! Free Block Layout:
//! ┌──────────────────────────────────┬───────────────────────┐
//! │ ListNode Header (16 bytes)       │ Available Space       │
//! │ ┌─────────────┬─────────────────┐│                       │
//! │ │ size: usize │ next: *mut Node ││                       │
//! │ └─────────────┴─────────────────┘│                       │
//! └──────────────────────────────────┴───────────────────────┘
//! ```
//!
//! # Usage Example
//!
//! ```rust
//! use core::alloc::Layout;
//!
//! use allocator::linked_list::LinkedListAllocator;
//!
//! let mut allocator = LinkedListAllocator::new();
//!
//! // Add heap memory (this would typically be done with actual heap memory)
//! let mut heap = vec![0u8; 1024];
//! unsafe {
//!     allocator.add_heap(heap.as_mut_ptr(), heap.len());
//! }
//!
//! // Allocate memory
//! let layout = Layout::from_size_align(64, 8).unwrap();
//! if let Some(ptr) = allocator.allocate(layout) {
//!     // Use the allocated memory...
//!
//!     // Free the memory
//!     unsafe {
//!         allocator.deallocate(ptr, layout);
//!     }
//! }
//! ```
//!
//! # Performance Characteristics
//!
//! - **Allocation**: O(n) where n is the number of free blocks
//! - **Deallocation**: O(n) where n is the number of free blocks
//! - **Memory Overhead**: 16 bytes per free block for the linked list header
//! - **Fragmentation**: Minimal due to automatic coalescing of adjacent blocks
//!
//! # Thread Safety
//!
//! The allocator is `Send` but not `Sync`. It can be moved between threads
//! but requires external synchronization for concurrent access.

use core::{alloc::Layout, ptr};

/// A node in the linked list of free memory blocks.
///
/// Each node represents a contiguous free memory region that can be allocated.
/// The node itself is stored at the beginning of the free memory region.
///
/// # Memory Layout
///
/// The node has 16-byte alignment to ensure proper memory alignment for
/// various data types that might be allocated from this region.
#[repr(align(16))]
#[derive(Debug)]
struct ListNode {
    /// Size of the free memory block in bytes
    size: usize,
    /// Pointer to the next free node in the linked list, or null if this is the
    /// last node
    next: *mut Self,
}
const _: () = assert!(size_of::<ListNode>() == align_of::<ListNode>());

impl ListNode {
    /// Creates a new list node at the specified memory location.
    ///
    /// # Arguments
    ///
    /// * `node_ptr` - Pointer to the memory location where the node will be
    ///   created
    /// * `node_size` - Size of the free memory block in bytes
    ///
    /// # Returns
    ///
    /// A pointer to the newly created list node.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `node_ptr` is not null and properly aligned to 16 bytes
    /// - `node_size` is at least `size_of::<ListNode>()` bytes
    /// - `node_size` is a multiple of `size_of::<ListNode>()`
    /// - The memory region `node_ptr..node_ptr + node_size` is valid and unused
    unsafe fn new(node_ptr: *mut u8, node_size: usize) -> *mut Self {
        #[expect(clippy::cast_ptr_alignment)]
        let node = node_ptr.cast::<Self>();
        // ensure that the freed region is capable of holding `ListNode`.
        assert!(!node.is_null(), "Node pointer must not be null");
        assert!(node.is_aligned(), "Node pointer must be properly aligned");
        assert!(
            node_size >= size_of::<Self>(),
            "Node size must be at least size of ListNode"
        );
        assert!(
            node_size.is_multiple_of(size_of::<Self>()),
            "Node size must be multiple of ListNode size"
        );

        unsafe {
            (*node).size = node_size;
            (*node).next = ptr::null_mut();
        }

        node
    }

    /// Returns a pointer to the start of the memory region managed by this
    /// node.
    ///
    /// # Arguments
    ///
    /// * `node` - Pointer to the list node
    ///
    /// # Returns
    ///
    /// A pointer to the beginning of the memory region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `node` is not null and points to a valid
    /// `ListNode`.
    unsafe fn start(node: *mut Self) -> *mut u8 {
        assert!(!node.is_null(), "Node must not be null");
        node.cast()
    }

    /// Returns a pointer to the end of the memory region managed by this node.
    ///
    /// # Arguments
    ///
    /// * `node` - Pointer to the list node
    ///
    /// # Returns
    ///
    /// A pointer to one byte past the end of the memory region.
    ///
    /// # Safety
    ///
    /// The caller must ensure that `node` is not null and points to a valid
    /// `ListNode`.
    unsafe fn end(node: *mut Self) -> *mut u8 {
        assert!(!node.is_null(), "Node must not be null");
        unsafe { Self::start(node).map_addr(|addr| addr + (*node).size) }
    }

    /// Concatenates two nodes, potentially merging them if they are adjacent.
    ///
    /// If the nodes represent adjacent memory regions, they will be merged into
    /// a single larger node. Otherwise, they will be linked together in the
    /// list.
    ///
    /// # Arguments
    ///
    /// * `prev_node` - The first node (can be null)
    /// * `next_node` - The second node (can be null)
    ///
    /// # Returns
    ///
    /// A pointer to the resulting node after concatenation/merging.
    ///
    /// # Safety
    ///
    /// The caller must ensure that both nodes (if not null) point to valid
    /// `ListNode`s.
    unsafe fn concat(prev_node: *mut Self, next_node: *mut Self) -> *mut Self {
        if prev_node.is_null() {
            return next_node;
        }
        if next_node.is_null() {
            return prev_node;
        }

        unsafe {
            if (*prev_node).size > 0 && ptr::eq(Self::end(prev_node), Self::start(next_node)) {
                (*prev_node).size += (*next_node).size;
                (*prev_node).next = (*next_node).next;
            } else {
                (*prev_node).next = next_node;
            }
        }

        prev_node
    }

    /// Attempts to split a node to satisfy an allocation request.
    ///
    /// This function tries to allocate memory from the current node with the
    /// specified size and alignment requirements. If successful, it may
    /// split the node into smaller pieces to minimize fragmentation.
    ///
    /// # Arguments
    ///
    /// * `prev_node` - The previous node in the list (can be null if
    ///   `current_node` is the head)
    /// * `current_node` - The node to attempt allocation from
    /// * `size` - The number of bytes to allocate
    /// * `align` - The alignment requirement in bytes
    ///
    /// # Returns
    ///
    /// If allocation is successful, returns a tuple containing:
    ///
    /// - A pointer to the allocated memory
    /// - A pointer to the new head of the remaining free list
    ///
    /// If allocation fails (insufficient space or alignment), returns `None`.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `current_node` is not null and points to a valid `ListNode`
    /// - `prev_node` is either null or points to a valid `ListNode` that
    ///   precedes `current_node`
    /// - `size` and `align` are greater than zero
    /// - `align` is a power of two
    unsafe fn try_split(
        prev_node: *mut Self,
        current_node: *mut Self,
        size: usize,
        align: usize,
    ) -> Option<(*mut u8, *mut Self)> {
        unsafe {
            assert!(size > 0, "Size must be greater than zero");
            assert!(align > 0, "Alignment must be greater than zero");
            assert!(!current_node.is_null(), "Current node must not be null");
            assert!(prev_node.is_null() || ptr::eq((*prev_node).next, current_node));

            let (alloc_start, alloc_end) = {
                let current_start = Self::start(current_node);
                let align_offset = current_start.align_offset(align);
                let alloc_start =
                    current_start.with_addr(current_start.addr().checked_add(align_offset)?);
                let alloc_end = alloc_start.with_addr(alloc_start.addr().checked_add(size)?);
                (alloc_start, alloc_end)
            };
            if alloc_end > Self::end(current_node) {
                return None;
            }

            let mut next_node = (*current_node).next;

            assert!(!ptr::eq(prev_node, current_node) && !ptr::eq(next_node, current_node));

            // Split the node if there is remaining space after allocation
            {
                let current_end = Self::end(current_node);
                if alloc_end < current_end {
                    let remaining_size = current_end.addr() - alloc_end.addr();
                    let remaining_node = Self::new(alloc_end, remaining_size);
                    let remaining_node = Self::concat(remaining_node, next_node);
                    (*current_node).next = remaining_node; // Do not use `Self::set_next` here.
                    (*current_node).size -= remaining_size;
                    next_node = remaining_node;
                }
            }
            assert_eq!(alloc_end, Self::end(current_node));

            // Split the node if there is remaining space before allocation
            if alloc_start > Self::start(current_node) {
                (*current_node).size -= size;
                assert_eq!(alloc_start, Self::end(current_node));
            }

            // Allocation starts at the beginning of the node
            let prev_node = Self::concat(prev_node, next_node);
            Some((alloc_start, prev_node))
        }
    }
}

/// A simple linked list allocator for managing free memory regions.
///
/// This allocator maintains a sorted linked list of free memory blocks ordered
/// by address. It supports adding heap regions, allocating memory with specific
/// size and alignment requirements, and deallocating memory with automatic
/// coalescing of adjacent free blocks.
///
/// # Algorithm
///
/// - **Allocation**: Uses first-fit strategy, searching the free list for the
///   first block that can satisfy the request. Splits blocks when necessary to
///   minimize waste.
/// - **Deallocation**: Inserts freed blocks back into the free list in address
///   order and automatically coalesces adjacent free blocks to reduce
///   fragmentation.
///
/// # Thread Safety
///
/// This allocator is `Send` but not `Sync`. It can be moved between threads but
/// requires external synchronization for concurrent access.
pub struct LinkedListAllocator {
    free_list_head: *mut ListNode,
}

unsafe impl Send for LinkedListAllocator {}

impl Default for LinkedListAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl LinkedListAllocator {
    /// Creates an empty [`LinkedListAllocator`].
    ///
    /// The allocator starts with no heap regions. Use
    /// [`add_heap`](Self::add_heap) to add memory regions before attempting
    /// allocations.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            free_list_head: ptr::null_mut(),
        }
    }

    /// Adds a new heap region to the allocator.
    ///
    /// The heap region will be aligned and sized appropriately for the
    /// allocator's internal data structures. If the region is too small
    /// after alignment, it will be ignored.
    ///
    /// # Arguments
    ///
    /// * `heap_start` - Pointer to the beginning of the heap region
    /// * `heap_size` - Size of the heap region in bytes
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - The given heap range `heap_start..heap_start + heap_size` is valid
    /// - The memory region is not currently in use by any other allocator or
    ///   code
    /// - The memory region will remain valid for the lifetime of this allocator
    /// - This method is not called concurrently with other allocator operations
    pub unsafe fn add_heap(&mut self, heap_start: *mut u8, heap_size: usize) {
        unsafe {
            const _: () = assert!(size_of::<ListNode>() == align_of::<ListNode>());

            const NODE_SIZE_ALIGN: usize = size_of::<ListNode>();

            let align_offset = heap_start.align_offset(NODE_SIZE_ALIGN);
            let aligned_heap_start = heap_start.map_addr(|addr| addr + align_offset);
            let aligned_heap_size =
                heap_size.saturating_sub(align_offset) / NODE_SIZE_ALIGN * NODE_SIZE_ALIGN;
            if aligned_heap_size == 0 {
                return; // No space to allocate
            }

            let new_free_node = ListNode::new(aligned_heap_start, aligned_heap_size);
            self.insert_free_node(new_free_node);
        }
    }

    /// Allocates a block of memory with the given layout.
    ///
    /// Uses a first-fit allocation strategy, searching the free list for the
    /// first block that can satisfy the size and alignment requirements.
    /// The allocated block may be larger than requested due to alignment
    /// and internal fragmentation.
    ///
    /// # Arguments
    ///
    /// * `layout` - The memory layout specifying size and alignment
    ///   requirements
    ///
    /// # Returns
    ///
    /// Returns a pointer to the allocated memory block, or `None` if allocation
    /// fails. The returned pointer is guaranteed to meet the alignment
    /// requirements and point to at least `layout.size()` bytes of memory.
    ///
    /// # Panics
    ///
    /// Panics if `layout.size()` is zero.
    pub fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
        if self.free_list_head.is_null() {
            return None;
        }

        let (size, align) = Self::size_align(layout);
        unsafe {
            let mut prev_node = ptr::null_mut();
            let mut current_node = self.free_list_head;
            loop {
                let Some((alloc_start, new_head)) =
                    ListNode::try_split(prev_node, current_node, size, align)
                else {
                    // No suitable node found, move to the next node
                    if (*current_node).next.is_null() {
                        // We have looped through the entire list
                        return None;
                    }
                    prev_node = current_node;
                    current_node = (*current_node).next;
                    continue;
                };

                if ptr::eq(current_node, self.free_list_head) {
                    self.free_list_head = new_head;
                }
                return Some(alloc_start);
            }
        }
    }

    /// Deallocates a block of memory with the given layout.
    ///
    /// The freed memory block is inserted back into the free list in address
    /// order and automatically coalesced with adjacent free blocks to
    /// reduce fragmentation.
    ///
    /// # Arguments
    ///
    /// * `ptr` - Pointer to the memory block to deallocate
    /// * `layout` - The layout that was used when allocating this block
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `ptr` was allocated by this allocator using the exact same `layout`
    /// - `ptr` has not been deallocated before
    /// - The memory block is not currently in use
    /// - This method is not called concurrently with other allocator operations
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let (size, _align) = Self::size_align(layout);
        unsafe {
            let free_node = ListNode::new(ptr, size);
            self.insert_free_node(free_node);
        }
    }

    /// Adjusts the layout to meet the allocator's internal requirements.
    ///
    /// Both size and alignment are rounded up to be multiples of `ListNode`
    /// size to ensure proper alignment and efficient memory management.
    ///
    /// # Arguments
    ///
    /// * `layout` - The requested memory layout
    ///
    /// # Returns
    ///
    /// A tuple containing the adjusted (size, alignment) values.
    ///
    /// # Panics
    ///
    /// Panics if `layout.size()` is zero.
    fn size_align(layout: Layout) -> (usize, usize) {
        assert!(layout.size() > 0);
        let size = layout.size().next_multiple_of(size_of::<ListNode>());
        let align = layout.align().next_multiple_of(size_of::<ListNode>());
        (size, align)
    }

    /// Inserts a free node into the sorted free list and attempts coalescing.
    ///
    /// The node is inserted in the correct position to maintain address
    /// ordering of the free list. Adjacent free blocks are automatically
    /// merged to reduce fragmentation.
    ///
    /// # Arguments
    ///
    /// * `free_node` - Pointer to the node to insert
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `free_node` is not null and points to a valid `ListNode`
    /// - The node is not already in the free list
    /// - The memory region represented by the node is actually free
    unsafe fn insert_free_node(&mut self, free_node: *mut ListNode) {
        unsafe {
            assert!(!free_node.is_null(), "Free node must not be null");
            assert!(
                (*free_node).next.is_null(),
                "Free node must not be already linked"
            );

            if self.free_list_head.is_null() {
                self.free_list_head = free_node;
                return;
            }

            if free_node < self.free_list_head {
                // Insert at the beginning of the list
                self.free_list_head = ListNode::concat(free_node, self.free_list_head);
                return;
            }

            // Find the correct position to insert the free node (keeping list sorted by
            // address)
            let mut current_node = self.free_list_head;
            loop {
                assert!(current_node < free_node);
                if free_node < (*current_node).next || (*current_node).next.is_null() {
                    break;
                }
                current_node = (*current_node).next;
            }

            // Insert the free node and attempt coalescing
            let free_node = ListNode::concat(free_node, (*current_node).next);
            ListNode::concat(current_node, free_node);
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec::Vec;

    use super::*;

    struct TestAllocator {
        allocator: LinkedListAllocator,
    }

    impl TestAllocator {
        fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
            let ptr = self.allocator.allocate(layout)?;
            unsafe {
                ptr.write_bytes(0x33, layout.size());
            }
            Some(ptr)
        }

        pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
            unsafe {
                for i in 0..layout.size() {
                    assert_eq!(ptr.add(i).read(), 0x33);
                }
                ptr.write_bytes(0x55, layout.size());
                self.allocator.deallocate(ptr, layout);
            }
        }
    }

    fn with_test_heap<F>(heap_size: usize, test_fn: F)
    where
        F: FnOnce(*mut u8, usize),
    {
        unsafe {
            let layout = Layout::from_size_align(heap_size, 16).unwrap();
            let heap_start = alloc::alloc::alloc(layout);
            heap_start.write_bytes(0x11, heap_size);
            test_fn(heap_start, heap_size);
            alloc::alloc::dealloc(heap_start, layout);
        }
    }

    fn with_test_allocator<F>(size: usize, test_fn: F)
    where
        F: FnOnce(&mut TestAllocator),
    {
        with_test_heap(size, |heap_start, heap_size| unsafe {
            let mut allocator = LinkedListAllocator::new();
            allocator.add_heap(heap_start, heap_size);
            test_fn(&mut TestAllocator { allocator });
        });
    }

    #[test]
    fn test_basic_allocation() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_multiple_allocations() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let ptr1 = allocator.allocate(layout).unwrap();
            let ptr2 = allocator.allocate(layout).unwrap();
            let ptr3 = allocator.allocate(layout).unwrap();

            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());
            assert_ne!(ptr1, ptr2);
            assert_ne!(ptr2, ptr3);
            assert_ne!(ptr1, ptr3);

            allocator.deallocate(ptr1, layout);
            allocator.deallocate(ptr2, layout);
            allocator.deallocate(ptr3, layout);
        });
    }

    #[test]
    fn test_alignment() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 64).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());
            assert_eq!(ptr.addr() % 64, 0);

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_large_alignment() {
        with_test_allocator(2048, |allocator| unsafe {
            let layout = Layout::from_size_align(32, 256).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());
            assert_eq!(ptr.addr() % 256, 0);

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_fragmentation_and_coalescing() {
        with_test_allocator(256, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();

            // Allocate three blocks
            let ptr1 = allocator.allocate(layout).unwrap();
            let ptr2 = allocator.allocate(layout).unwrap();
            let ptr3 = allocator.allocate(layout).unwrap();
            let ptr4 = allocator.allocate(layout).unwrap();
            assert!(allocator.allocate(layout).is_none());

            assert!(ptr1 < ptr2 && ptr2 < ptr3 && ptr3 < ptr4);

            // Deallocate middle block
            allocator.deallocate(ptr2, layout);

            // Deallocate first block (should coalesce with freed middle block)
            allocator.deallocate(ptr1, layout);

            // Allocate a larger block that should fit in the coalesced space
            let large_layout = Layout::from_size_align(128, 1).unwrap();
            let large_ptr = allocator.allocate(large_layout).unwrap();
            assert!(!large_ptr.is_null());

            allocator.deallocate(large_ptr, large_layout);
            allocator.deallocate(ptr3, layout);
        });
    }

    #[test]
    fn test_fragmentation_and_allocation_failure() {
        with_test_allocator(256, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();

            // Allocate three blocks
            let ptr1 = allocator.allocate(layout).unwrap();
            let ptr2 = allocator.allocate(layout).unwrap();
            let ptr3 = allocator.allocate(layout).unwrap();
            let ptr4 = allocator.allocate(layout).unwrap();
            assert!(allocator.allocate(layout).is_none());

            // Deallocate middle block
            allocator.deallocate(ptr2, layout);
            allocator.deallocate(ptr4, layout);

            // Try to allocate a larger block that should fail
            let large_layout = Layout::from_size_align(128, 1).unwrap();
            assert!(allocator.allocate(large_layout).is_none());

            // Clean up remaining allocations
            allocator.deallocate(ptr1, layout);
            allocator.deallocate(ptr3, layout);
        });
    }

    #[test]
    fn test_allocate_entire_heap() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout = Layout::from_size_align(1024, 1).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            assert!(allocator.allocate(layout).is_none());

            allocator.deallocate(ptr, layout);
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());
        });
    }

    #[test]
    fn test_out_of_memory() {
        with_test_allocator(128, |allocator| {
            let layout = Layout::from_size_align(256, 1).unwrap();
            let ptr = allocator.allocate(layout);
            assert!(ptr.is_none());
        });
    }

    #[test]
    fn test_small_heap() {
        with_test_allocator(32, |allocator| unsafe {
            let layout = Layout::from_size_align(16, 1).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_allocate_from_insufficient_heap() {
        with_test_allocator(8, |allocator| {
            let layout = Layout::from_size_align(16, 1).unwrap();
            let ptr = allocator.allocate(layout);
            assert!(ptr.is_none());
        });
    }

    #[test]
    fn test_reallocation_after_full_deallocation() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let mut ptrs = Vec::new();

            // Allocate until out of memory
            while let Some(ptr) = allocator.allocate(layout) {
                ptrs.push(ptr);
            }

            assert!(!ptrs.is_empty());

            // Deallocate all
            for ptr in ptrs {
                allocator.deallocate(ptr, layout);
            }

            // Should be able to allocate again
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_different_sized_allocations() {
        with_test_allocator(1024, |allocator| unsafe {
            let layout1 = Layout::from_size_align(32, 1).unwrap();
            let layout2 = Layout::from_size_align(64, 1).unwrap();
            let layout3 = Layout::from_size_align(128, 1).unwrap();

            let ptr1 = allocator.allocate(layout1).unwrap();
            let ptr2 = allocator.allocate(layout2).unwrap();
            let ptr3 = allocator.allocate(layout3).unwrap();

            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());

            allocator.deallocate(ptr1, layout1);
            allocator.deallocate(ptr2, layout2);
            allocator.deallocate(ptr3, layout3);
        });
    }
}
