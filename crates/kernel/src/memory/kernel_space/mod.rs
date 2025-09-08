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
use crate::{
    cpu::{self, CpuMask},
    memory::Align as _,
    sync::spinlock::SpinMutex,
};

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

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum KernelSpaceInitError {
    #[snafu(display("failed to create kernel page table"))]
    #[snafu(provide(ref, priority, Location => location))]
    CreateKernelPageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PageTableError,
    },
}

pub fn init() -> Result<(), KernelSpaceInitError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::kernel_space_init_error::*;

    stack::init();
    let kpgtbl = KernelPageTable::new().context(CreateKernelPageTableSnafu)?;
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
#[snafu(module)]
pub enum ApplyPageTableChangesError {
    #[snafu(display(
        "failed to remote sfence.vma for cpus `{cpu_mask:?}` with virtual address range `{start:#x}..{end:#x}`",
        start = vaddr_range.start,
        end = vaddr_range.end
    ))]
    #[snafu(provide(ref, priority, Location => location))]
    RemoteSfenceVma {
        vaddr_range: Range<usize>,
        cpu_mask: CpuMask,
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: SbiError,
    },
}

fn apply_page_table_changes(
    asid: u16,
    vaddr_range: Range<usize>,
) -> Result<(), ApplyPageTableChangesError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::apply_page_table_changes_error::*;

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
        .with_context(|_e| RemoteSfenceVmaSnafu {
            cpu_mask,
            vaddr_range: vaddr_range.clone(),
        })?;
    }

    Ok(())
}

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum IdentityMapError {
    #[snafu(display("failed to update kernel page table"))]
    #[snafu(provide(ref, priority, Location => location))]
    UpdateKernelPageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PageTableError,
    },
    #[snafu(display("failed to apply kernel page table changes"))]
    #[snafu(provide(ref, priority, Location => location))]
    ApplyKernelPageTableChanges {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ApplyPageTableChangesError,
    },
}

pub fn identity_map_range(
    range: Range<usize>,
    flags: MapPageFlags,
) -> Result<(), IdentityMapError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::identity_map_error::*;

    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .identity_map_range(range.clone(), flags)
        .context(UpdateKernelPageTableSnafu)?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        apply_page_table_changes(asid, range).context(ApplyKernelPageTableChangesSnafu)?;
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
#[snafu(module)]
pub enum AllocateKernelStackError {
    #[snafu(display("no stack slot available"))]
    #[snafu(provide(ref, priority, Location => location))]
    NoStackSlot {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to update kernel page table"))]
    #[snafu(provide(ref, priority, Location => location))]
    UpdateKernelPageTable {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PageTableError,
    },
    #[snafu(display("failed to apply kernel page table changes"))]
    #[snafu(provide(ref, priority, Location => location))]
    ApplyKernelPageTableChanges {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ApplyPageTableChangesError,
    },
}

pub fn allocate_kernel_stack() -> Result<KernelStack, AllocateKernelStackError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::allocate_kernel_stack_error::*;

    let slot = StackSlot::allocate().context(NoStackSlotSnafu)?;

    let mut kpgtbl = KERNEL_PAGE_TABLE.get().unwrap().lock();
    let asid = kpgtbl.asid();
    kpgtbl
        .allocate_virt_addr_range(slot.range(), MapPageFlags::RW)
        .context(UpdateKernelPageTableSnafu)?;
    kpgtbl.unlock();

    if APPLIED.get().load(Ordering::Acquire) {
        apply_page_table_changes(asid, slot.range()).context(ApplyKernelPageTableChangesSnafu)?;
    }

    Ok(KernelStack { slot })
}
