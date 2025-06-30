//! Fixed-size block allocator implementation.
//!
//! This module provides a fixed-size block allocator that manages memory
//! efficiently by using predefined block sizes. It combines multiple linked
//! list allocators for different block sizes with a fallback allocator for
//! larger allocations.

use core::alloc::Layout;

use crate::linked_list::LinkedListAllocator;

/// Layout for memory arenas used by fixed-size block allocators.
///
/// Each arena is 4096 bytes aligned to 4096 bytes, which is typically
/// one page size on most systems.
const ARENA_LAYOUT: Layout = match Layout::from_size_align(4096, 4096) {
    Ok(layout) => layout,
    Err(_) => panic!("Failed to create arena layout"),
};

/// Available block sizes for the fixed-size block allocator.
///
/// These sizes are chosen to cover common allocation patterns efficiently,
/// ranging from 8 bytes to 2048 bytes. Allocations larger than 2048 bytes
/// will use the fallback allocator.
const BLOCK_SIZES: [usize; 9] = [8, 16, 32, 64, 128, 256, 512, 1024, 2048];

/// Determines the appropriate block size index for a given layout.
///
/// This function maps allocation requests to the smallest available block size
/// that can satisfy the request. Returns `None` if the requested size is larger
/// than any available block size, indicating that the fallback allocator should
/// be used.
///
/// # Arguments
///
/// * `layout` - The memory layout describing the allocation request
///
/// # Returns
///
/// * `Some(index)` - Index into `BLOCK_SIZES` array for the appropriate block
///   size
/// * `None` - If the request is too large and should use the fallback allocator
///
/// # Panics
///
/// Panics if the layout's size is smaller than its alignment requirement.
fn list_index(layout: &Layout) -> Option<usize> {
    assert!(layout.size() >= layout.align());
    let required_block_size = layout.size();
    BLOCK_SIZES.iter().position(|&s| s >= required_block_size)
}

/// Creates a layout for allocating blocks of the specified size index.
///
/// This function creates a layout where both the size and alignment are set
/// to the block size, ensuring proper alignment for the fixed-size blocks.
/// This is used internally to maintain consistent alignment requirements
/// across all block allocations.
///
/// # Arguments
///
/// * `index` - Index into the `BLOCK_SIZES` array
///
/// # Returns
///
/// A `Layout` with size and alignment both set to `BLOCK_SIZES[index]`
///
/// # Panics
///
/// Panics if `index` is out of bounds for the `BLOCK_SIZES` array, or if
/// the layout creation fails (which should never happen for valid block sizes).
fn alloc_layout(index: usize) -> Layout {
    assert!(index < BLOCK_SIZES.len());
    let size = BLOCK_SIZES[index];
    Layout::from_size_align(size, size).unwrap()
}

/// A fixed-size block allocator that manages memory in predefined block sizes.
///
/// This allocator maintains separate linked list allocators for each supported
/// block size, allowing for efficient allocation and deallocation of commonly
/// used sizes. For allocations larger than the maximum block size, it falls
/// back to a general-purpose linked list allocator.
///
/// # Memory Layout
///
/// The allocator organizes memory into arenas of 4096 bytes each. When a
/// specific block size allocator runs out of memory, it requests a new arena
/// from the fallback allocator and subdivides it into blocks of the appropriate
/// size.
///
/// # Performance Characteristics
///
/// - O(1) allocation and deallocation for supported block sizes
/// - Minimal fragmentation for common allocation patterns
/// - Efficient memory reuse through block recycling
pub struct FixedSizeBlockAllocator {
    /// Array of linked list allocators, one for each supported block size.
    list_heads: [LinkedListAllocator; BLOCK_SIZES.len()],
    /// Fallback allocator for large allocations and arena management.
    fallback_allocator: LinkedListAllocator,
}

impl Default for FixedSizeBlockAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl FixedSizeBlockAllocator {
    /// Creates a new empty fixed-size block allocator.
    ///
    /// The allocator is initialized with empty linked list allocators for each
    /// block size and an empty fallback allocator. No memory is allocated until
    /// [`add_heap`](Self::add_heap) is called.
    ///
    /// # Examples
    ///
    /// ```
    /// # use allocator::fixed_size_block::FixedSizeBlockAllocator;
    /// let allocator = FixedSizeBlockAllocator::new();
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        let fallback_allocator = LinkedListAllocator::new();
        Self {
            list_heads: [const { LinkedListAllocator::new() }; _],
            fallback_allocator,
        }
    }

    /// Adds a new heap region to the allocator.
    ///
    /// This method initializes the allocator with a memory region that it can
    /// use for allocations. The memory is initially managed by the fallback
    /// allocator and will be subdivided into arenas as needed.
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
    /// - The memory region `heap_start..heap_start + heap_size` is valid
    /// - The memory region is not used by any other allocator or code
    /// - The memory region remains valid for the lifetime of the allocator
    /// - `heap_start` is properly aligned for the target architecture
    ///
    /// # Examples
    ///
    /// ```
    /// # use allocator::fixed_size_block::FixedSizeBlockAllocator;
    /// let mut allocator = FixedSizeBlockAllocator::new();
    /// let mut heap = vec![0u8; 8192];
    /// unsafe {
    ///     allocator.add_heap(heap.as_mut_ptr(), heap.len());
    /// }
    /// ```
    pub unsafe fn add_heap(&mut self, heap_start: *mut u8, heap_size: usize) {
        unsafe {
            self.fallback_allocator.add_heap(heap_start, heap_size);
        }
    }

    /// Allocates a block of memory with the specified layout.
    ///
    /// This method attempts to allocate memory using the most appropriate
    /// strategy based on the requested size:
    ///
    /// 1. For small allocations (≤ 2048 bytes), it uses the corresponding
    ///    fixed-size block allocator
    /// 2. If the block allocator is empty, it allocates a new arena from the
    ///    fallback allocator
    /// 3. For large allocations (> 2048 bytes), it uses the fallback allocator
    ///    directly
    ///
    /// # Arguments
    ///
    /// * `layout` - The layout describing the size and alignment requirements
    ///
    /// # Returns
    ///
    /// * `Some(ptr)` - Pointer to the allocated memory block
    /// * `None` - If allocation fails due to insufficient memory
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::alloc::Layout;
    /// # use allocator::fixed_size_block::FixedSizeBlockAllocator;
    /// # let mut allocator = FixedSizeBlockAllocator::new();
    /// let layout = Layout::from_size_align(64, 8).unwrap();
    /// if let Some(ptr) = allocator.allocate(layout) {
    ///     // Use the allocated memory
    ///     unsafe {
    ///         allocator.deallocate(ptr, layout);
    ///     }
    /// }
    /// ```
    pub fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
        let Some(index) = list_index(&layout) else {
            return self.fallback_allocator.allocate(layout);
        };
        let alloc_layout = alloc_layout(index);

        let list_head = &mut self.list_heads[index];
        if let Some(ptr) = list_head.allocate(alloc_layout) {
            return Some(ptr);
        }

        let heap = self.fallback_allocator.allocate(ARENA_LAYOUT)?;
        unsafe {
            list_head.add_heap(heap, ARENA_LAYOUT.size());
        }

        list_head.allocate(alloc_layout)
    }

    /// Deallocates a previously allocated block of memory.
    ///
    /// This method returns the memory block to the appropriate allocator
    /// based on its size. Small blocks are returned to their corresponding
    /// fixed-size allocator, while large blocks are returned to the fallback
    /// allocator.
    ///
    /// # Arguments
    ///
    /// * `ptr` - Pointer to the memory block to deallocate
    /// * `layout` - The layout that was used for the original allocation
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    ///
    /// - `ptr` was allocated by this allocator instance
    /// - `layout` exactly matches the layout used for the original allocation
    /// - `ptr` has not been deallocated previously
    /// - The memory pointed to by `ptr` is not accessed after deallocation
    ///
    /// # Examples
    ///
    /// ```
    /// # use core::alloc::Layout;
    /// # use allocator::fixed_size_block::FixedSizeBlockAllocator;
    /// # let mut allocator = FixedSizeBlockAllocator::new();
    /// let layout = Layout::from_size_align(64, 8).unwrap();
    /// if let Some(ptr) = allocator.allocate(layout) {
    ///     // Use the memory...
    ///     unsafe {
    ///         allocator.deallocate(ptr, layout);
    ///     }
    /// }
    /// ```
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let Some(index) = list_index(&layout) else {
            unsafe {
                self.fallback_allocator.deallocate(ptr, layout);
            }
            return;
        };
        let alloc_layout = alloc_layout(index);

        let list_head = &mut self.list_heads[index];
        unsafe {
            list_head.deallocate(ptr, alloc_layout);
        }

        if let Some(heap) = list_head.allocate(ARENA_LAYOUT) {
            unsafe {
                self.fallback_allocator.deallocate(heap, ARENA_LAYOUT);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAllocator {
        allocator: FixedSizeBlockAllocator,
    }

    impl TestAllocator {
        fn new() -> Self {
            Self {
                allocator: FixedSizeBlockAllocator::new(),
            }
        }

        unsafe fn add_heap(&mut self, heap_start: *mut u8, heap_size: usize) {
            unsafe {
                self.allocator.add_heap(heap_start, heap_size);
            }
        }

        fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
            let ptr = self.allocator.allocate(layout)?;
            unsafe {
                ptr.write_bytes(0x33, layout.size());
            }
            Some(ptr)
        }

        unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
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
            let heap_start = std::alloc::alloc(layout);
            heap_start.write_bytes(0x11, heap_size);
            test_fn(heap_start, heap_size);
            std::alloc::dealloc(heap_start, layout);
        }
    }

    fn with_test_allocator<F>(size: usize, test_fn: F)
    where
        F: FnOnce(&mut TestAllocator),
    {
        with_test_heap(size, |heap_start, heap_size| unsafe {
            let mut allocator = TestAllocator::new();
            allocator.add_heap(heap_start, heap_size);
            test_fn(&mut allocator);
        });
    }

    #[test]
    fn test_list_index() {
        assert_eq!(list_index(&Layout::from_size_align(1, 1).unwrap()), Some(0));
        assert_eq!(list_index(&Layout::from_size_align(8, 1).unwrap()), Some(0));
        assert_eq!(
            list_index(&Layout::from_size_align(16, 1).unwrap()),
            Some(1)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(32, 1).unwrap()),
            Some(2)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(64, 1).unwrap()),
            Some(3)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(128, 1).unwrap()),
            Some(4)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(256, 1).unwrap()),
            Some(5)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(512, 1).unwrap()),
            Some(6)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(1024, 1).unwrap()),
            Some(7)
        );
        assert_eq!(
            list_index(&Layout::from_size_align(2048, 1).unwrap()),
            Some(8)
        );

        // Sizes that require fallback allocator
        assert_eq!(list_index(&Layout::from_size_align(4096, 1).unwrap()), None);
        assert_eq!(list_index(&Layout::from_size_align(8192, 1).unwrap()), None);
    }

    #[test]
    fn test_basic_allocation() {
        with_test_allocator(8192, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_multiple_allocations() {
        with_test_allocator(8192, |allocator| unsafe {
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
    fn test_different_block_sizes() {
        with_test_allocator(4096 * 1024, |allocator| unsafe {
            let mut ptrs = Vec::new();

            // Test each block size
            for &size in &BLOCK_SIZES {
                let layout = Layout::from_size_align(size, 1).unwrap();
                let ptr = allocator.allocate(layout).unwrap();
                assert!(!ptr.is_null());
                ptrs.push((ptr, layout));
            }

            // Deallocate all
            for (ptr, layout) in ptrs {
                allocator.deallocate(ptr, layout);
            }
        });
    }

    #[test]
    fn test_fallback_allocator() {
        with_test_allocator(16384, |allocator| unsafe {
            // Large size that uses fallback allocator
            let layout = Layout::from_size_align(4096, 1).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_arena_allocation() {
        with_test_allocator(16384, |allocator| unsafe {
            // Allocate many blocks of the same size to trigger new arena allocation
            let layout = Layout::from_size_align(64, 1).unwrap();
            let mut ptrs = Vec::new();

            // Allocate until the first arena is exhausted
            for _ in 0..100 {
                if let Some(ptr) = allocator.allocate(layout) {
                    ptrs.push(ptr);
                } else {
                    break;
                }
            }

            // At least one allocation should succeed
            assert!(!ptrs.is_empty());

            // Deallocate all
            for ptr in ptrs {
                allocator.deallocate(ptr, layout);
            }
        });
    }

    #[test]
    fn test_alignment() {
        with_test_allocator(8192, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 64).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());
            assert_eq!(ptr.addr() % 64, 0);

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_large_alignment() {
        with_test_allocator(8192, |allocator| unsafe {
            let layout = Layout::from_size_align(256, 256).unwrap();
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());
            assert_eq!(ptr.addr() % 256, 0);

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_mixed_allocation_patterns() {
        with_test_allocator(4096 * 1024, |allocator| unsafe {
            let layout1 = Layout::from_size_align(32, 1).unwrap();
            let layout2 = Layout::from_size_align(128, 1).unwrap();
            let layout3 = Layout::from_size_align(512, 1).unwrap();
            let layout4 = Layout::from_size_align(4096, 1).unwrap(); // Fallback

            let ptr1 = allocator.allocate(layout1).unwrap();
            let ptr2 = allocator.allocate(layout2).unwrap();
            let ptr3 = allocator.allocate(layout3).unwrap();
            let ptr4 = allocator.allocate(layout4).unwrap();

            assert!(!ptr1.is_null());
            assert!(!ptr2.is_null());
            assert!(!ptr3.is_null());
            assert!(!ptr4.is_null());

            allocator.deallocate(ptr1, layout1);
            allocator.deallocate(ptr2, layout2);
            allocator.deallocate(ptr3, layout3);
            allocator.deallocate(ptr4, layout4);
        });
    }

    #[test]
    fn test_fragmentation_handling() {
        with_test_allocator(16384, |allocator| unsafe {
            let layout = Layout::from_size_align(128, 1).unwrap();
            let mut ptrs = Vec::new();

            // Allocate multiple blocks
            for _ in 0..10 {
                if let Some(ptr) = allocator.allocate(layout) {
                    ptrs.push(ptr);
                } else {
                    break;
                }
            }

            // Deallocate every other block
            for (i, ptr) in ptrs.iter().enumerate() {
                if i.is_multiple_of(2) {
                    allocator.deallocate(*ptr, layout);
                }
            }

            // Test if new allocation succeeds
            let new_ptr = allocator.allocate(layout).unwrap();
            assert!(!new_ptr.is_null());

            allocator.deallocate(new_ptr, layout);

            // Deallocate remaining blocks
            for (i, ptr) in ptrs.iter().enumerate() {
                if !i.is_multiple_of(2) {
                    allocator.deallocate(*ptr, layout);
                }
            }
        });
    }

    #[test]
    fn test_reallocation_after_full_deallocation() {
        with_test_allocator(8192, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let mut ptrs = Vec::new();

            // Allocate as many blocks as possible
            for _ in 0..100 {
                if let Some(ptr) = allocator.allocate(layout) {
                    ptrs.push(ptr);
                } else {
                    break;
                }
            }

            assert!(!ptrs.is_empty());

            // Deallocate all
            for ptr in ptrs {
                allocator.deallocate(ptr, layout);
            }

            // Verify allocation is possible again
            let ptr = allocator.allocate(layout).unwrap();
            assert!(!ptr.is_null());

            allocator.deallocate(ptr, layout);
        });
    }

    #[test]
    fn test_out_of_memory() {
        with_test_allocator(1024, |allocator| {
            // Try to allocate more than heap size
            let layout = Layout::from_size_align(8192, 1).unwrap();
            let ptr = allocator.allocate(layout);
            assert!(ptr.is_none());
        });
    }

    #[test]
    fn test_default_constructor() {
        let allocator = FixedSizeBlockAllocator::default();
        assert_eq!(allocator.list_heads.len(), BLOCK_SIZES.len());
    }

    #[test]
    fn test_empty_allocator() {
        let mut allocator = FixedSizeBlockAllocator::new();
        let layout = Layout::from_size_align(64, 1).unwrap();
        let ptr = allocator.allocate(layout);
        assert!(ptr.is_none());
    }

    #[test]
    fn test_small_heap_handling() {
        with_test_allocator(128, |allocator| unsafe {
            let layout = Layout::from_size_align(64, 1).unwrap();
            let ptr = allocator.allocate(layout);

            if let Some(ptr) = ptr {
                allocator.deallocate(ptr, layout);
            }
        });
    }

    #[test]
    fn test_boundary_sizes() {
        with_test_allocator(16384, |allocator| unsafe {
            // Test boundary sizes
            let layouts = [
                Layout::from_size_align(7, 1).unwrap(),    // 8-byte block
                Layout::from_size_align(15, 1).unwrap(),   // 16-byte block
                Layout::from_size_align(2047, 1).unwrap(), // 2048-byte block
                Layout::from_size_align(2049, 1).unwrap(), // Fallback
            ];

            let mut ptrs = Vec::new();
            for layout in layouts {
                if let Some(ptr) = allocator.allocate(layout) {
                    ptrs.push((ptr, layout));
                }
            }

            for (ptr, layout) in ptrs {
                allocator.deallocate(ptr, layout);
            }
        });
    }
}
