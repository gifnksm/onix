use core::ops::Range;

use devicetree::flattened::Devicetree;
use range_set::RangeSet;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;

use self::fdt::DevicetreeError;
use super::{
    kernel_space::KernelPageTable,
    page_table::sv39::{MapPageFlags, PageTableError},
};

mod fdt;

unsafe extern "C" {
    #[link_name = "__onix_kernel_start"]
    static mut KERNEL_START: u8;
    #[link_name = "__onix_kernel_end"]
    static mut KERNEL_END: u8;
    #[link_name = "__onix_bss_start"]
    static mut BSS_START: u8;
    #[link_name = "__onix_bss_end"]
    static mut BSS_END: u8;
    #[link_name = "__onix_boot_stack_start"]
    static mut BOOT_STACK_START: u8;
    #[link_name = "__onix_boot_stack_end"]
    static mut BOOT_STACK_END: u8;
    #[link_name = "__onix_rx_start"]
    static mut RX_START: u8;
    #[link_name = "__onix_rx_end"]
    static mut RX_END: u8;
    #[link_name = "__onix_ro_start"]
    static mut RO_START: u8;
    #[link_name = "__onix_ro_end"]
    static mut RO_END: u8;
    #[link_name = "__onix_rw_start"]
    static mut RW_START: u8;
    #[link_name = "__onix_rw_end"]
    static mut RW_END: u8;

}

pub fn bss_addr_range() -> Range<usize> {
    (&raw const BSS_START).addr()..(&raw const BSS_END).addr()
}

#[derive(Debug, Snafu)]
pub enum MemoryLayoutError {
    #[snafu(display("failed to parse devicetree: {source}"))]
    Devicetree {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: DevicetreeError,
    },
}

#[derive(Debug)]
pub struct MemoryLayout {
    available_ranges: RangeSet<128>,
    dtb_range: Range<usize>,
}

impl MemoryLayout {
    pub fn new(dtb: &Devicetree) -> Result<Self, MemoryLayoutError> {
        let mut available_ranges = RangeSet::<128>::new();
        fdt::insert_memory_ranges(dtb, &mut available_ranges).context(DevicetreeSnafu)?;
        fdt::remove_reserved_ranges(dtb, &mut available_ranges);
        available_ranges.remove(opensbi_reserved_range());
        available_ranges.remove(kernel_reserved_range());

        let dtb_range = fdt::dtb_range(dtb);
        Ok(Self {
            available_ranges,
            dtb_range,
        })
    }

    pub fn initial_heap_ranges(&self) -> RangeSet<128> {
        let mut heap_ranges = self.available_ranges.clone();
        heap_ranges.remove(kernel_boot_stack_range());
        heap_ranges.remove(self.dtb_range.clone());
        heap_ranges
    }

    pub fn dtb_range(&self) -> Range<usize> {
        self.dtb_range.clone()
    }
}

fn opensbi_reserved_range() -> Range<usize> {
    0x8000_0000..0x8020_0000
}

fn kernel_reserved_range() -> Range<usize> {
    let kernel_start = (&raw const KERNEL_START).addr();
    let kernel_end = (&raw const KERNEL_END).addr();
    super::expand_to_page_boundaries(kernel_start..kernel_end)
}

pub fn kernel_boot_stack_range() -> Range<usize> {
    let stack_start = (&raw const BOOT_STACK_START).addr();
    let stack_end = (&raw const BOOT_STACK_END).addr();
    super::expand_to_page_boundaries(stack_start..stack_end)
}

pub fn kernel_rx_range() -> Range<usize> {
    let rx_start = (&raw const RX_START).addr();
    let rx_end = (&raw const RX_END).addr();
    super::expand_to_page_boundaries(rx_start..rx_end)
}

pub fn kernel_ro_range() -> Range<usize> {
    let ro_start = (&raw const RO_START).addr();
    let ro_end = (&raw const RO_END).addr();
    super::expand_to_page_boundaries(ro_start..ro_end)
}

pub fn kernel_rw_range() -> Range<usize> {
    let rw_start = (&raw const RW_START).addr();
    let rw_end = (&raw const RW_END).addr();
    super::expand_to_page_boundaries(rw_start..rw_end)
}

pub fn update_kernel_page_table(
    kpgtbl: &mut KernelPageTable,
    layout: &MemoryLayout,
) -> Result<(), PageTableError> {
    let rw_ranges = &layout.available_ranges;
    for range in rw_ranges {
        kpgtbl.identity_map_range(range.clone(), MapPageFlags::RW)?;
    }
    Ok(())
}
