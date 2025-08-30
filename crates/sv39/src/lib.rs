#![feature(allocator_api)]
#![feature(error_generic_member_access)]
#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use core::{
    alloc::{AllocError, Layout},
    fmt::{self, DebugMap},
};

use bitflags::bitflags;
use platform_cast::CastInto as _;
use riscv::register::satp::{Mode, Satp};
use snafu::Snafu;
use snafu_utils::Location;

use self::{
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    entry::{PageFlags, PageTableEntry, PageTableEntryRef},
    table::{PageTable, PageTableRef},
};

pub mod address;
mod entry;
mod table;

pub const PAGE_SIZE: usize = 4096;
const PAGE_SHIFT: usize = 12;
const _: () = assert!(PAGE_SIZE == 1 << PAGE_SHIFT);

/// Errors that can occur during page table operations.
#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum PageTableError {
    #[snafu(display("failed to allocate new page table"))]
    #[snafu(provide(ref, priority, Location => location))]
    AllocPageTable {
        #[snafu(source)]
        source: AllocError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to allocate new page table entry, layout: {layout:?}"))]
    #[snafu(provide(ref, priority, Location => location))]
    AllocPage {
        layout: Layout,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("attempted to map a page to an already mapped address"))]
    #[snafu(provide(ref, priority, Location => location))]
    AlreadyMapped {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid flags for mapping page: {flags:?}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidMapFlags {
        flags: MapPageFlags,
        #[snafu(implicit)]
        location: Location,
    },
}

bitflags! {
    /// Flags for mapping pages in the page table.
    ///
    /// These flags control the permissions and properties of mapped pages.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapPageFlags: u64 {
        /// Read Bit of page table entry.
        ///
        /// If set, the CPU can read to this virtual address.
        const R = 1 << 0;

        /// Write Bit of page table entry.
        ///
        /// If set, the CPU can write to this virtual address.
        const W = 1 << 1;

        /// Executable Bit of page table entry.
        ///
        /// If set, the CPU can execute instructions on this virtual address.
        const X = 1 << 2;

        /// UserMode Bit of page table entry.
        ///
        /// If set, userspace can access this virtual address.
        const U = 1 << 3;

        const RW = Self::R.bits() | Self::W.bits();
        const RX = Self::R.bits() | Self::X.bits();
        const RWX = Self::R.bits() | Self::W.bits() | Self::X.bits();
        const UR = Self::U.bits() | Self::R.bits();
        const UW = Self::U.bits() | Self::W.bits();
        const URW = Self::U.bits() | Self::RW.bits();
        const URX = Self::U.bits() | Self::RX.bits();
        const URWX = Self::U.bits() | Self::RWX.bits();
    }
}

/// Root of an SV39 page table hierarchy.
///
/// This structure represents the top-level page table and provides methods
/// for mapping pages and managing the virtual memory space.
pub struct PageTableRoot {
    pt: Box<PageTable>,
    asid: u16,
}

impl PageTableRoot {
    /// Creates a new page table root with the specified ASID.
    pub fn new(asid: u16) -> Result<Self, PageTableError> {
        Ok(Self {
            pt: PageTable::try_allocate()?,
            asid,
        })
    }

    fn as_ref(&self) -> PageTableRef<&PageTable> {
        PageTableRef::new(&self.pt, 2, VirtPageNum::MIN)
    }

    fn as_mut(&mut self) -> PageTableRef<&mut PageTable> {
        PageTableRef::new(&mut self.pt, 2, VirtPageNum::MIN)
    }

    /// Returns the physical page number of the root page table.
    ///
    /// This is used for setting the SATP register.
    #[must_use]
    pub fn phys_page_num(&self) -> PhysPageNum {
        self.as_ref().phys_page_num()
    }

    /// Returns the SATP register value for this page table.
    ///
    /// This value can be written to the SATP register to activate this page
    /// table.
    #[must_use]
    pub fn satp(&self) -> Satp {
        let mut satp = Satp::from_bits(0);
        satp.set_mode(Mode::Sv39);
        satp.set_asid(self.asid.into());
        satp.set_ppn(self.phys_page_num().value().cast_into());
        satp
    }

    /// Returns the Address Space Identifier (ASID) for this page table.
    #[must_use]
    pub fn asid(&self) -> u16 {
        self.asid
    }

    /// Allocates and maps pages starting from the specified virtual page
    /// number.
    ///
    /// This function allocates physical memory and maps it to the virtual
    /// address space.
    ///
    /// # Arguments
    ///
    /// * `virt_page_num` - Starting virtual page number for mapping
    /// * `count` - Number of pages to allocate and map
    /// * `flags` - Permission flags for the mapped pages
    ///
    /// # Returns
    ///
    /// The actual number of pages that were successfully mapped.
    ///
    /// # Errors
    ///
    /// Returns an error if allocation fails, flags are invalid, or pages are
    /// already mapped.
    pub fn allocate_pages(
        &mut self,
        virt_page_num: VirtPageNum,
        count: usize,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        self.as_mut().allocate_pages(virt_page_num, count, flags)
    }

    /// Maps existing physical pages to virtual addresses.
    ///
    /// This function creates mappings from virtual pages to existing physical
    /// pages.
    ///
    /// # Arguments
    ///
    /// * `virt_page_num` - Starting virtual page number for mapping
    /// * `phys_page_num` - Starting physical page number to map
    /// * `count` - Number of pages to map
    /// * `flags` - Permission flags for the mapped pages
    ///
    /// # Returns
    ///
    /// The actual number of pages that were successfully mapped.
    ///
    /// # Errors
    ///
    /// Returns an error if flags are invalid or pages are already mapped.
    pub fn map_fixed_pages(
        &mut self,
        virt_page_num: VirtPageNum,
        phys_page_num: PhysPageNum,
        count: usize,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        self.as_mut()
            .map_fixed_pages(virt_page_num, phys_page_num, count, flags)
    }
}

impl fmt::Debug for PageTableRoot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&DebugPageTable { pt: self.as_ref() }, f)
    }
}

struct DebugPageTable<'a> {
    pt: PageTableRef<&'a PageTable>,
}

impl fmt::Debug for DebugPageTable<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pa = self.pt.phys_addr();
        write!(f, "PageTable@{pa:#p} ")?;
        let dm = f.debug_map();
        let mut dumper = DumpEntry::new(dm);
        for entry in self.pt.entries() {
            dumper.entry(&entry);
        }
        dumper.finish()?;
        Ok(())
    }
}

struct DumpState {
    va: (VirtAddr, VirtAddr),
    pa: Option<(PhysAddr, PhysAddr)>,
    flags: PageFlags,
}

impl fmt::Debug for DumpState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { va: _, pa, flags } = *self;
        let Some((min_pa, max_pa)) = pa else {
            return write!(f, "[invalid]",);
        };
        let flags = DebugFlags(flags);
        write!(f, "{min_pa:#p}..={max_pa:#p} ({flags:?})",)
    }
}

impl DumpState {
    fn new(entry: &PageTableEntryRef<&PageTableEntry>) -> Self {
        let va = (entry.min_virt_addr(), entry.max_virt_addr());
        let pa = match (entry.min_phys_addr(), entry.max_phys_addr()) {
            (Some(min_pa), Some(max_pa)) => Some((min_pa, max_pa)),
            _ => None,
        };
        let flags = entry.flags();
        Self { va, pa, flags }
    }

    fn try_join(&mut self, other: &Self) -> Result<(), ()> {
        if other.va.0.checked_sub(self.va.1) != Some(1) || self.flags != other.flags {
            return Err(());
        }

        let ppns = match (&mut self.pa, &other.pa) {
            (Some(self_pa), Some(other_pa)) if other_pa.0.checked_sub(self_pa.1) == Some(1) => {
                Some((self_pa, other_pa))
            }
            (None, None) => None,
            _ => {
                return Err(());
            }
        };

        self.va.1 = other.va.1;
        if let Some((self_ppn, other_ppn)) = ppns {
            self_ppn.1 = other_ppn.1;
        }

        Ok(())
    }
}

struct DumpEntry<'a, 'b> {
    dm: DebugMap<'a, 'b>,
    state: Option<DumpState>,
}

impl<'a, 'b> DumpEntry<'a, 'b> {
    fn new(dm: DebugMap<'a, 'b>) -> Self {
        Self { dm, state: None }
    }

    fn dump(&mut self) {
        let Some(state) = self.state.take() else {
            return;
        };
        self.dm.entry(
            &(DebugPointer(state.va.0)..=DebugPointer(state.va.1)),
            &state,
        );
    }

    fn entry(&mut self, entry: &PageTableEntryRef<&PageTableEntry>) {
        if let Some(pt) = entry.next_level_table() {
            self.dump();
            self.dm.entry(
                &(DebugPointer(pt.min_virt_addr())..=DebugPointer(pt.max_virt_addr())),
                &DebugPageTable { pt },
            );
            return;
        }

        let new_state = DumpState::new(entry);
        if let Some(state) = &mut self.state
            && state.try_join(&new_state).is_ok()
        {
            return;
        }
        self.dump();
        self.state = Some(new_state);
    }

    fn finish(&mut self) -> fmt::Result {
        self.dump();
        self.dm.finish()
    }
}

struct DebugPointer<T>(T);
impl<T> fmt::Debug for DebugPointer<T>
where
    T: fmt::Pointer,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#p}", self.0)
    }
}

struct DebugFlags(PageFlags);
impl fmt::Debug for DebugFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let all_flags = PageFlags::all() ^ PageFlags::V;
        for (name, flag) in all_flags.iter_names() {
            if self.0.contains(flag) {
                for ch in name.chars() {
                    write!(f, "{}", ch.to_ascii_lowercase())?;
                }
            } else {
                write!(f, "-")?;
            }
        }
        Ok(())
    }
}
