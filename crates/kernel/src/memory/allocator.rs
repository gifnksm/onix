use core::{
    alloc::{GlobalAlloc, Layout},
    ops::Range,
    ptr,
};

use allocator::fixed_size_block::FixedSizeBlockAllocator;
use arrayvec::ArrayVec;
use devicetree::flattened::layout::ReserveEntry;
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
        source: devicetree::flattened::CreateError,
    },
    #[snafu(display("failed to parse devicetree: {source}"))]
    DtbParse {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: devicetree::flattened::node::ParseStructError,
    },
}

pub unsafe fn init(
    dtb_pa: usize,
) -> Result<(devicetree::parsed::Devicetree, HeapLayout), AllocatorInitError> {
    let dtb =
        unsafe { devicetree::flattened::Devicetree::from_addr(dtb_pa) }.context(DtbCreateSnafu)?;

    let heap_layout = HeapLayout::new(&dtb)?;
    let heap_ranges = heap_layout.compute_heap_range(true, true);

    for range in &heap_ranges {
        let mut allocator = ALLOCATOR.allocator.lock();
        unsafe {
            allocator.add_heap(
                ptr::with_exposed_provenance_mut(range.start),
                range.end - range.start,
            );
        }
    }

    let dtree = dtb.parse().context(DtbParseSnafu)?;

    {
        let mut allocator = ALLOCATOR.allocator.lock();
        let dtb_range = &heap_layout.dtb_range;
        unsafe {
            allocator.add_heap(
                ptr::with_exposed_provenance_mut(dtb_range.start),
                dtb_range.end - dtb_range.start,
            );
        }
    }

    Ok((dtree, heap_layout))
}

pub fn update_kernel_page_table(
    kpgtbl: &mut KernelPageTable,
    heap_layout: &HeapLayout,
) -> Result<(), PageTableError> {
    let rw_ranges = heap_layout.compute_heap_range(false, false);
    for range in rw_ranges {
        kpgtbl.identity_map_range(range, MapPageFlags::RW)?;
    }
    Ok(())
}

pub struct HeapLayout {
    memory_ranges: ArrayVec<Range<usize>, 128>,
    reserved_ranges: ArrayVec<Range<usize>, 128>,
    dtb_range: Range<usize>,
}

impl HeapLayout {
    fn new(dtb: &devicetree::flattened::Devicetree) -> Result<Self, AllocatorInitError> {
        let mut memory_ranges = ArrayVec::<Range<usize>, 128>::new();
        for range in layout::memory_addr_ranges(dtb).context(MemoryAddrRangesSnafu)? {
            let range = range.context(MemoryAddrRangesSnafu)?;
            memory_ranges.push(range);
        }
        let mut reserved_ranges = dtb
            .mem_rsvmap()
            .iter()
            .map(ReserveEntry::range)
            .collect::<ArrayVec<_, _>>();
        reserved_ranges.push(layout::opensbi_reserved_range());
        reserved_ranges.push(layout::kernel_reserved_range());

        let dtb_range = layout::dtb_range(dtb);
        Ok(Self {
            memory_ranges,
            reserved_ranges,
            dtb_range,
        })
    }

    fn compute_heap_range(
        &self,
        exclude_boot_stack: bool,
        exclude_dtb: bool,
    ) -> ArrayVec<Range<usize>, 128> {
        let mut heap_ranges = self.memory_ranges.clone();

        for reserved in &self.reserved_ranges {
            exclude_reserved_range(&mut heap_ranges, reserved.clone());
        }

        if exclude_boot_stack {
            exclude_reserved_range(&mut heap_ranges, layout::kernel_boot_stack_range());
        }
        if exclude_dtb {
            exclude_reserved_range(&mut heap_ranges, self.dtb_range.clone());
        }

        heap_ranges
    }
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
