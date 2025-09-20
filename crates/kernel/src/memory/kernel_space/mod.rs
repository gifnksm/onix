use alloc::format;
use core::{
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

use riscv::register::satp::{self, Satp};
use riscv_utils::asm;
use sbi::rfence;
use snafu::{OptionExt as _, ResultExt as _};
use spin::Once;
use sv39::{
    MapPageFlags, PageTableError, PageTableRoot,
    address::{PhysAddr, VirtAddr},
};

use self::stack::StackSlot;
use super::PAGE_SIZE;
use crate::{cpu, error::GenericError, memory::Align as _, sync::spinlock::SpinMutex};

mod stack;

const KERNEL_ASID: u16 = 0;

#[derive(Debug)]
struct KernelPageTable {
    pt: PageTableRoot,
}

impl KernelPageTable {
    fn new() -> Result<Self, PageTableError> {
        let pt = PageTableRoot::new(KERNEL_ASID)?;
        Ok(Self { pt })
    }

    fn identity_map_range(
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

    fn allocate_virt_addr_range(
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

cpu_local! {
    static APPLIED: AtomicBool = AtomicBool::new(false);
}

pub fn init() -> Result<(), GenericError> {
    stack::init();
    let kpgtbl = KernelPageTable::new().whatever_context("failed to create kernel page table")?;
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
    APPLIED.get().store(true, Ordering::Release);
}

fn apply_page_table_changes(asid: u16, vaddr_range: Range<usize>) -> Result<(), GenericError> {
    vaddr_range.clone().step_by(PAGE_SIZE).for_each(|vaddr| {
        asm::sfence_vma(vaddr, asid.into());
    });

    for cpu_mask in cpu::remote_cpu_masks() {
        rfence::remote_sfence_vma_asid(
            cpu_mask.mask,
            cpu_mask.base,
            vaddr_range.start,
            vaddr_range.len(),
            asid.into(),
        )
        .with_whatever_context(|_e| {
            format!(
                "failed to remote sfence.vma for cpus `{cpu_mask:?}` with virtual address range \
                 `{start:#x}..{end:#x}`",
                start = vaddr_range.start,
                end = vaddr_range.end,
            )
        })?;
    }

    Ok(())
}

pub fn identity_map_range(range: Range<usize>, flags: MapPageFlags) -> Result<(), GenericError> {
    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .identity_map_range(range.clone(), flags)
        .with_whatever_context(|_| {
            format!("failed to update kernel page table, range={range:#x?}, flags={flags:?}",)
        })?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        apply_page_table_changes(asid, range.clone()).with_whatever_context(|_| {
            format!("failed to apply kernel page table changes, asid={asid}, range={range:#x?}")
        })?;
    }

    Ok(())
}

#[derive(Debug)]
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

pub fn allocate_kernel_stack() -> Result<KernelStack, GenericError> {
    let slot = StackSlot::allocate().whatever_context("no stack slot available")?;

    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .allocate_virt_addr_range(slot.range(), MapPageFlags::RW)
        .whatever_context("failed to update kernel page table")?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        apply_page_table_changes(asid, slot.range())
            .whatever_context("failed to apply page table changes")?;
    }

    Ok(KernelStack { slot })
}
