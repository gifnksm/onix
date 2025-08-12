use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Range,
    ptr,
};

use allocator::fixed_size_block::FixedSizeBlockAllocator;
use range_set::RangeSet;

use crate::sync::spinlock::SpinMutex;

#[global_allocator]
static ALLOCATOR: LockedKernelAllocator = LockedKernelAllocator::new();

struct KernelAllocator {
    allocator: FixedSizeBlockAllocator,
    heap_ranges: RangeSet<128>,
}

impl KernelAllocator {
    const fn new() -> Self {
        Self {
            allocator: FixedSizeBlockAllocator::new(),
            heap_ranges: RangeSet::new(),
        }
    }

    unsafe fn add_heap(&mut self, range: Range<usize>) {
        unsafe {
            self.allocator.add_heap(
                ptr::with_exposed_provenance_mut(range.start),
                range.end - range.start,
            );
            self.heap_ranges.insert(range);
        }
    }

    fn allocate(&mut self, layout: Layout) -> Option<*mut u8> {
        self.allocator.allocate(layout)
    }

    unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        unsafe {
            self.allocator.deallocate(ptr, layout);
        }
    }
}

struct LockedKernelAllocator(SpinMutex<KernelAllocator>);

impl LockedKernelAllocator {
    const fn new() -> Self {
        Self(SpinMutex::new(KernelAllocator::new()))
    }
}

unsafe impl GlobalAlloc for LockedKernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.0.lock().allocate(layout).unwrap_or_default()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.0.lock().deallocate(ptr, layout) }
    }
}

pub unsafe fn add_heap_ranges<I>(ranges: I)
where
    I: IntoIterator<Item = Range<usize>>,
{
    let mut allocator = ALLOCATOR.0.lock();
    for range in ranges {
        unsafe {
            allocator.add_heap(range);
        }
    }
}
