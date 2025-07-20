use alloc::boxed::Box;
use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Range,
    ptr,
};

use allocator::fixed_size_block::FixedSizeBlockAllocator;
use arrayvec::ArrayVec;
use devicetree::Devicetree;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;

use super::{
    kernel_space::KernelPageTable,
    layout::{self, MemoryAddrRangesError},
    page_table::sv39::{MapPageFlags, PageTableError},
};
use crate::spinlock::SpinMutex;

#[global_allocator]
static ALLOCATOR: KernelAllocator = KernelAllocator::new();

struct KernelAllocator {
    allocator: SpinMutex<FixedSizeBlockAllocator>,
}

impl KernelAllocator {
    const fn new() -> Self {
        Self {
            allocator: SpinMutex::new(FixedSizeBlockAllocator::new()),
        }
    }
}

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.allocator.lock().allocate(layout).unwrap_or_default()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { self.allocator.lock().deallocate(ptr, layout) }
    }
}

#[derive(Debug, Snafu)]
pub enum AllocatorInitError {
    #[snafu(display("invalid memory address range: {source}"))]
    MemoryAddrRanges {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: MemoryAddrRangesError,
    },
    #[snafu(display("failed to create devicetree: {source}"))]
    DtbCreate {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: devicetree::CreateError,
    },
}

pub unsafe fn init(dtb_pa: usize) -> Result<Box<[u8]>, AllocatorInitError> {
    let dtb = unsafe { Devicetree::from_addr(dtb_pa) }.context(DtbCreateSnafu)?;

    let heap_ranges = compute_heap_range(&dtb, true, true)?;

    for range in &heap_ranges {
        let mut allocator = ALLOCATOR.allocator.lock();
        unsafe {
            allocator.add_heap(
                ptr::with_exposed_provenance_mut(range.start),
                range.end - range.start,
            );
        }
    }

    let dtb_bytes = Box::<[u8]>::from(dtb.as_bytes());
    {
        let mut allocator = ALLOCATOR.allocator.lock();
        let dtb_range = layout::dtb_range(&dtb);
        unsafe {
            allocator.add_heap(
                ptr::with_exposed_provenance_mut(dtb_range.start),
                dtb_range.end - dtb_range.start,
            );
        }
    }

    Ok(dtb_bytes)
}

pub fn update_kernel_page_table(
    kpgtbl: &mut KernelPageTable,
    dtb: &Devicetree<'_>,
) -> Result<(), PageTableError> {
    let rw_ranges = compute_heap_range(dtb, false, false).unwrap();
    for range in rw_ranges {
        kpgtbl.identity_map_range(range, MapPageFlags::RW)?;
    }
    Ok(())
}

fn exclude_reserved_range<const N: usize>(
    ranges: &mut ArrayVec<Range<usize>, N>,
    reserved: Range<usize>,
) {
    let mut out = ArrayVec::<Range<usize>, N>::new();
    for range in ranges.iter() {
        if range.start < reserved.end && reserved.start < range.end {
            if range.start < reserved.start {
                out.push(range.start..reserved.start);
            }
            if reserved.end < range.end {
                out.push(reserved.end..range.end);
            }
        } else {
            out.push(range.clone());
        }
    }
    *ranges = out;
}

fn compute_heap_range(
    dtb: &Devicetree<'_>,
    exclude_boot_stack: bool,
    exclude_dtb: bool,
) -> Result<ArrayVec<Range<usize>, 128>, AllocatorInitError> {
    let mut heap_ranges = ArrayVec::<Range<usize>, 128>::new();
    for range in layout::memory_addr_ranges(dtb).context(MemoryAddrRangesSnafu)? {
        let range = range.context(MemoryAddrRangesSnafu)?;
        heap_ranges.push(range);
    }

    for entry in dtb.mem_rsvmap() {
        exclude_reserved_range(&mut heap_ranges, entry.range());
    }
    exclude_reserved_range(&mut heap_ranges, layout::opensbi_reserved_range());
    exclude_reserved_range(&mut heap_ranges, layout::kernel_reserved_range());
    if exclude_boot_stack {
        exclude_reserved_range(&mut heap_ranges, layout::kernel_boot_stack_range());
    }
    if exclude_dtb {
        exclude_reserved_range(&mut heap_ranges, layout::dtb_range(dtb));
    }

    Ok(heap_ranges)
}
