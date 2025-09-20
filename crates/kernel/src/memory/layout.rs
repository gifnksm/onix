use alloc::format;
use core::ops::Range;

use devtree::{DeserializeNode, Devicetree, types::property::Reg};
use range_set::RangeSet;
use snafu::ResultExt as _;
use sv39::MapPageFlags;

use super::kernel_space;
use crate::error::GenericError;

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

#[derive(Debug)]
pub struct HeapLayout {
    available_ranges: RangeSet<128>,
}

#[derive(Debug, DeserializeNode)]
struct Memory<'blob> {
    #[devtree(property)]
    reg: Reg<'blob>,
}

#[derive(Debug, DeserializeNode)]
pub struct ReservedMemoryRegion<'blob> {
    #[devtree(property)]
    reg: Reg<'blob>,
}

impl HeapLayout {
    pub fn new(dt: &Devicetree) -> Result<Self, GenericError> {
        let mut available_ranges = RangeSet::<128>::new();

        let root_node = dt
            .read_root_node()
            .whatever_context("failed to read devicetree root node")?;
        root_node
            .try_visit_all_nodes_by_query("/memory", |node| -> Result<(), GenericError> {
                let memory = node
                    .deserialize_node::<Memory>()
                    .whatever_context("failed to deserialize devicetree memory node")?;
                for reg in memory.reg {
                    available_ranges.insert(reg.range());
                }
                Ok(())
            })
            .whatever_context("failed to read devicetree node")?
            .map_or(Ok(()), Err)?;

        for rsv in dt.memory_reservation_map() {
            available_ranges.remove(rsv.address_range());
        }

        root_node
            .try_visit_all_nodes_by_query(
                "/reserved-memory/*",
                |node| -> Result<(), GenericError> {
                    let region = node
                        .deserialize_node::<ReservedMemoryRegion>()
                        .whatever_context("failed to deserialize reserved-memory node")?;
                    for reg in region.reg {
                        available_ranges.remove(reg.range());
                    }
                    Ok(())
                },
            )
            .whatever_context("failed to read devicetree node")?
            .map_or(Ok(()), Err)?;

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

pub fn update_kernel_page_table(layout: &HeapLayout) -> Result<(), GenericError> {
    let fixed_pairs = [
        (kernel_rx_range(), MapPageFlags::RX),
        (kernel_ro_range(), MapPageFlags::R),
        (kernel_rw_range(), MapPageFlags::RW),
    ];
    let heap_pairs = layout
        .available_ranges
        .iter()
        .map(|range| (range.clone(), MapPageFlags::RW));

    for (range, flags) in fixed_pairs.into_iter().chain(heap_pairs) {
        kernel_space::identity_map_range(range.clone(), flags).with_whatever_context(
            move |_| {
                format!(
                    "failed to identity map kernel page table, range={range:#x?}, flags={flags:?}"
                )
            },
        )?;
    }

    Ok(())
}
