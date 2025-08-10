use core::ops::Range;

use riscv::register::satp::{self, Satp};
use riscv_utils::asm;
use spin::Once;

use super::{
    PAGE_SIZE,
    layout::{self, MemoryLayout},
    page_table::sv39::{
        MapPageFlags, PageTableError, PageTableRoot,
        address::{PhysAddr, VirtAddr},
    },
};
use crate::{cpu, memory::Align as _};

const STACK_SIZE: usize = 128 * 1024;

const KERNEL_ASID: u16 = 0;

#[derive(Debug)]
pub struct KernelPageTable {
    pt: PageTableRoot,
}

impl KernelPageTable {
    fn new() -> Result<Self, PageTableError> {
        let pt = PageTableRoot::new(KERNEL_ASID)?;
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
        self.pt.map_fixed_pages(start_vpn, start_ppn, count, flags)
    }

    pub fn allocate_virt_addr_range(
        &mut self,
        addr_range: Range<usize>,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        assert!(addr_range.start.is_page_aligned());
        assert!(addr_range.end.is_page_aligned());
        let start_vpn = VirtAddr::from_addr(addr_range.start).page_num();
        let count = addr_range.len() / PAGE_SIZE;
        self.pt.allocate_pages(start_vpn, count, flags)
    }

    fn satp(&self) -> Satp {
        self.pt.satp()
    }

    fn asid(&self) -> u16 {
        self.pt.asid()
    }
}

static KERNEL_PAGE_TABLE: Once<KernelPageTable> = Once::new();

pub fn init(memory_layout: &MemoryLayout) -> Result<(), PageTableError> {
    let mut kpgtbl = KernelPageTable::new()?;

    kpgtbl.identity_map_range(layout::kernel_rx_range(), MapPageFlags::RX)?;
    kpgtbl.identity_map_range(layout::kernel_ro_range(), MapPageFlags::R)?;
    kpgtbl.identity_map_range(layout::kernel_rw_range(), MapPageFlags::RW)?;

    layout::update_kernel_page_table(&mut kpgtbl, memory_layout)?;
    cpu::update_kernel_page_table(&mut kpgtbl)?;

    KERNEL_PAGE_TABLE.call_once(|| kpgtbl);

    Ok(())
}

pub fn apply() {
    let kpgtbl = KERNEL_PAGE_TABLE.get().unwrap();
    let asid = kpgtbl.asid();

    // wait for any previous writes to the page table memory to finish.
    asm::sfence_vma_asid_all(asid.into());

    let satp = kpgtbl.satp();
    unsafe {
        satp::write(satp);
    }

    asm::sfence_vma_asid_all(asid.into());
}

pub fn kernel_stack_ranges(cpu_index: usize) -> Range<usize> {
    let base = 0xffff_ffff_0000_0000 + STACK_SIZE * 2 * cpu_index;
    base..base + STACK_SIZE
}
