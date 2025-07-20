use core::{iter::FusedIterator, ops::Range, ptr};

use devicetree::{
    Devicetree,
    node::{Children, GetPropertyError, Node, ParseStructError, RegIter},
};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

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
pub enum MemoryAddrRangesError {
    #[snafu(display("invalid struct: {source}"))]
    ParseStruct {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ParseStructError,
    },
    #[snafu(display("invalid property: {source}"))]
    GetProperty {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: GetPropertyError,
    },
    #[snafu(display("missing `reg` property in node"))]
    MissingRegProperty {
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn memory_addr_ranges<'fdt, 'tree>(
    dtb: &'tree Devicetree<'fdt>,
) -> Result<MemoryAddrRanges<'fdt, 'tree>, MemoryAddrRangesError> {
    let parent = dtb.root_node().context(ParseStructSnafu)?;
    let children = parent.children();
    Ok(MemoryAddrRanges {
        parent,
        children,
        reg_iter: None,
    })
}

pub struct MemoryAddrRanges<'fdt, 'tree> {
    parent: Node<'fdt, 'tree>,
    children: Children<'fdt, 'tree>,
    reg_iter: Option<RegIter<'fdt>>,
}

impl MemoryAddrRanges<'_, '_> {
    fn try_next(&mut self) -> Result<Option<Range<usize>>, MemoryAddrRangesError> {
        loop {
            if let Some(reg_iter) = self.reg_iter.as_mut() {
                if let Some(reg) = reg_iter.next() {
                    return Ok(Some(reg.range()));
                }
                self.reg_iter = None;
            }

            let Some(child) = self.children.next() else {
                return Ok(None);
            };
            let child = child.context(ParseStructSnafu)?;
            if child.name() != "memory" {
                continue;
            }

            let reg_iter = child
                .properties()
                .reg(&self.parent)
                .context(GetPropertySnafu)?
                .context(MissingRegPropertySnafu)?;
            self.reg_iter = Some(reg_iter);
        }
    }
}

impl Iterator for MemoryAddrRanges<'_, '_> {
    type Item = Result<Range<usize>, MemoryAddrRangesError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

impl FusedIterator for MemoryAddrRanges<'_, '_> {}

pub fn opensbi_reserved_range() -> Range<usize> {
    0x8000_0000..0x8020_0000
}

pub fn kernel_reserved_range() -> Range<usize> {
    let kernel_start = (&raw const KERNEL_START).addr();
    let kernel_end = (&raw const KERNEL_END).addr();
    super::expand_to_page_boundaries(kernel_start..kernel_end)
}

pub fn kernel_boot_stack_range() -> Range<usize> {
    let stack_start = (&raw const BOOT_STACK_START).addr();
    let stack_end = (&raw const BOOT_STACK_END).addr();
    super::expand_to_page_boundaries(stack_start..stack_end)
}

pub fn dtb_range(dtb: &Devicetree<'_>) -> Range<usize> {
    let header = dtb.header();
    let dtb_start = ptr::from_ref(header).addr();
    let dtb_end = dtb_start + dtb.size();
    super::expand_to_page_boundaries(dtb_start..dtb_end)
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
