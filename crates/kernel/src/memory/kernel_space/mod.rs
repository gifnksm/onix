use core::ops::Range;

use riscv::register::satp::{self, Satp};
use riscv_utils::asm;
use sbi::{SbiError, rfence};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;
use spin::Once;

use self::stack::StackSlot;
use super::{
    PAGE_SIZE,
    layout::{self, MemoryLayout},
    page_table::sv39::{
        MapPageFlags, PageTableError, PageTableRoot,
        address::{PhysAddr, VirtAddr},
    },
};
use crate::{cpu, memory::Align as _, spinlock::SpinMutex};

mod stack;

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

static KERNEL_PAGE_TABLE: Once<SpinMutex<KernelPageTable>> = Once::new();

pub fn init(memory_layout: &MemoryLayout) -> Result<(), PageTableError> {
    stack::init();
    let mut kpgtbl = KernelPageTable::new()?;

    kpgtbl.identity_map_range(layout::kernel_rx_range(), MapPageFlags::RX)?;
    kpgtbl.identity_map_range(layout::kernel_ro_range(), MapPageFlags::R)?;
    kpgtbl.identity_map_range(layout::kernel_rw_range(), MapPageFlags::RW)?;

    layout::update_kernel_page_table(&mut kpgtbl, memory_layout)?;

    KERNEL_PAGE_TABLE.call_once(|| SpinMutex::new(kpgtbl));

    Ok(())
}

pub fn apply() {
    let kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let satp = kpgtbl.satp();
    let asid = kpgtbl.asid();
    kpgtbl.unlock();

    // wait for any previous writes to the page table memory to finish.
    asm::sfence_vma_asid_all(asid.into());

    unsafe {
        satp::write(satp);
    }

    asm::sfence_vma_asid_all(asid.into());
}

pub struct KernelStack {
    slot: StackSlot,
}

impl KernelStack {
    pub fn top(&self) -> usize {
        self.slot.top()
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        unimplemented!("freeing kernel stack is not implemented yet");
    }
}

#[derive(Debug, Snafu)]
pub enum KernelStackError {
    #[snafu(display("no stack slot available"))]
    NoStackSlot {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("page table error: {source}"))]
    PageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(implicit)]
        source: PageTableError,
    },
    #[snafu(display("remote sfence.vma failed: {source}"))]
    RemoteSfenceVma {
        #[snafu(implicit)]
        location: Location,
        #[snafu(implicit)]
        source: SbiError,
    },
}

pub fn allocate_kernel_stack() -> Result<KernelStack, KernelStackError> {
    let slot = StackSlot::allocate().context(NoStackSlotSnafu)?;

    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .allocate_virt_addr_range(slot.range(), MapPageFlags::RW)
        .context(PageTableSnafu)?;
    kpgtbl.unlock();

    slot.range().step_by(PAGE_SIZE).for_each(|vaddr| {
        asm::sfence_vma(vaddr, asid.into());
    });

    for cpu_mask in cpu::remote_cpu_masks() {
        rfence::remote_sfence_vma_asid(
            cpu_mask.mask,
            cpu_mask.base,
            slot.range().start,
            slot.range().len(),
            asid.into(),
        )
        .context(RemoteSfenceVmaSnafu)?;
    }

    Ok(KernelStack { slot })
}
