use core::{
    ops::Range,
    sync::atomic::{AtomicBool, Ordering},
};

use riscv::register::satp::{self, Satp};
use riscv_utils::asm;
use sbi::{SbiError, rfence};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;
use spin::Once;
use sv39::{
    MapPageFlags, PageTableError, PageTableRoot,
    address::{PhysAddr, VirtAddr},
};

use self::stack::StackSlot;
use super::PAGE_SIZE;
use crate::{cpu, memory::Align as _, sync::spinlock::SpinMutex};

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

pub fn init() -> Result<(), PageTableError> {
    stack::init();
    let kpgtbl = KernelPageTable::new()?;
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

#[derive(Debug, Snafu)]
pub enum SfenceAddrsError {
    #[snafu(display("remote sfence.vma failed: {source}"))]
    RemoteSfenceVma {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: SbiError,
    },
}

fn sfence_addrs(asid: u16, vaddr_range: Range<usize>) -> Result<(), SfenceAddrsError> {
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
        .context(RemoteSfenceVmaSnafu)?;
    }

    Ok(())
}

#[derive(Debug, Snafu)]
#[snafu(context(suffix(IdentityMapSnafu)))]
pub enum IdentityMapError {
    #[snafu(display("page table error: {source}"))]
    PageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PageTableError,
    },
    #[snafu(display("sfence failed: {source}"))]
    SfenceAddrs {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: SfenceAddrsError,
    },
}

pub fn identity_map_range(
    range: Range<usize>,
    flags: MapPageFlags,
) -> Result<(), IdentityMapError> {
    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .identity_map_range(range.clone(), flags)
        .context(PageTableIdentityMapSnafu)?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        sfence_addrs(asid, range).context(SfenceAddrsIdentityMapSnafu)?;
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

#[derive(Debug, Snafu)]
#[snafu(context(suffix(AllocateKernelStackSnafu)))]
pub enum AllocateKernelStackError {
    #[snafu(display("no stack slot available"))]
    NoStackSlot {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("page table error: {source}"))]
    PageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PageTableError,
    },
    #[snafu(display("sfence failed: {source}"))]
    SfenceAddrs {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: SfenceAddrsError,
    },
}

pub fn allocate_kernel_stack() -> Result<KernelStack, AllocateKernelStackError> {
    let slot = StackSlot::allocate().context(NoStackSlotAllocateKernelStackSnafu)?;

    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .allocate_virt_addr_range(slot.range(), MapPageFlags::RW)
        .context(PageTableAllocateKernelStackSnafu)?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        sfence_addrs(asid, slot.range()).context(SfenceAddrsAllocateKernelStackSnafu)?;
    }

    Ok(KernelStack { slot })
}
