use core::ops::Range;

use riscv::{
    asm,
    register::satp::{self, Satp},
};
use spin::Once;

use super::{
    PAGE_SIZE,
    allocator::{self, HeapLayout},
    layout,
    page_table::sv39::{
        MapPageFlags, PageTableError, PageTableRoot,
        address::{PhysAddr, VirtAddr},
    },
};
use crate::memory::Align as _;

#[derive(Debug)]
pub struct KernelPageTable {
    pt: PageTableRoot,
}

impl KernelPageTable {
    fn new() -> Result<Self, PageTableError> {
        let pt = PageTableRoot::new()?;
        Ok(Self { pt })
    }

    pub fn identity_map_range(
        &mut self,
        addr_range: Range<usize>,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        assert!(addr_range.start.is_page_aligned());
        assert!(addr_range.end.is_page_aligned());
        let start_vpn = VirtAddr::from_addr(addr_range.start).page_num();
        let start_ppn = PhysAddr::from_addr(addr_range.start).page_num();
        let count = addr_range.len() / PAGE_SIZE;
        self.pt.map_fixed_pages(start_vpn, start_ppn, flags, count)
    }

    fn satp(&self) -> Satp {
        self.pt.satp()
    }
}

static KERNEL_PAGE_TABLE: Once<KernelPageTable> = Once::new();

pub fn init(heap_layout: &HeapLayout) -> Result<(), PageTableError> {
    let mut kpgtbl = KernelPageTable::new()?;

    kpgtbl.identity_map_range(layout::kernel_rx_range(), MapPageFlags::RX)?;
    kpgtbl.identity_map_range(layout::kernel_ro_range(), MapPageFlags::R)?;
    kpgtbl.identity_map_range(layout::kernel_rw_range(), MapPageFlags::RW)?;

    allocator::update_kernel_page_table(&mut kpgtbl, heap_layout)?;

    KERNEL_PAGE_TABLE.call_once(|| kpgtbl);

    Ok(())
}

pub fn apply() {
    let kpgtbl = KERNEL_PAGE_TABLE.get().unwrap();

    // wait for any previous writes to the page table memory to finish.
    asm::sfence_vma_all();

    let satp = kpgtbl.satp();
    unsafe {
        satp::write(satp);
    }

    asm::sfence_vma_all();

    crate::println!("{kpgtbl:#?}");
}
