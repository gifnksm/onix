use core::ops::Range;

use devicetree::parsed::{Devicetree, node::PropertyError};
use range_set::RangeSet;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;
use sv39::MapPageFlags;

use super::kernel_space::{self, IdentityMapError};

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
#[snafu(module)]
pub enum CreateHeapLayoutError {
    #[snafu(display("failed to parse devicetree property"))]
    #[snafu(provide(ref, priority, Location => location))]
    DevicetreeProperty {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PropertyError,
    },
}

#[derive(Debug)]
pub struct HeapLayout {
    available_ranges: RangeSet<128>,
}

impl HeapLayout {
    pub fn new(dtree: &Devicetree) -> Result<Self, CreateHeapLayoutError> {
        #[expect(clippy::wildcard_imports)]
        use self::create_heap_layout_error::*;

        let mut available_ranges = RangeSet::<128>::new();

        let root = dtree.root_node();
        let memory_nodes = root.children().filter(|node| node.name() == "memory");
        for node in memory_nodes {
            for reg in node.reg().context(DevicetreePropertySnafu)? {
                available_ranges.insert(reg.range());
            }
        }

        for rsv in dtree.mem_rsvmap() {
            available_ranges.remove(rsv.range());
        }
        if let Some(reserved_memory_node) = dtree.find_node_by_path("/reserved-memory") {
            for child in reserved_memory_node.children() {
                for reg in child.reg().context(DevicetreePropertySnafu)? {
                    available_ranges.remove(reg.range());
                }
            }
        }
        available_ranges.remove(kernel_reserved_range());

        Ok(Self { available_ranges })
    }

    pub fn heap_ranges(&self) -> RangeSet<128> {
        let mut heap_ranges = self.available_ranges.clone();
        heap_ranges.remove(kernel_boot_stack_range());
        heap_ranges
    }
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

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum UpdateKernelPageTableError {
    #[snafu(display("failed to identity map kernel page table"))]
    #[snafu(provide(ref, priority, Location => location))]
    IdentityMap {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: IdentityMapError,
    },
}

pub fn update_kernel_page_table(layout: &HeapLayout) -> Result<(), UpdateKernelPageTableError> {
    #[expect(clippy::wildcard_imports)]
    use self::update_kernel_page_table_error::*;

    kernel_space::identity_map_range(kernel_rx_range(), MapPageFlags::RX)
        .context(IdentityMapSnafu)?;
    kernel_space::identity_map_range(kernel_ro_range(), MapPageFlags::R)
        .context(IdentityMapSnafu)?;
    kernel_space::identity_map_range(kernel_rw_range(), MapPageFlags::RW)
        .context(IdentityMapSnafu)?;

    let rw_ranges = &layout.available_ranges;
    for range in rw_ranges {
        kernel_space::identity_map_range(range.clone(), MapPageFlags::RW)
            .context(IdentityMapSnafu)?;
    }
    Ok(())
}
