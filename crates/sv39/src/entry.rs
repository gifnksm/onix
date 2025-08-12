use alloc::boxed::Box;
use core::{
    alloc::Layout,
    ops::{Deref, DerefMut},
};

use bitflags::bitflags;
use dataview::Pod;
use snafu::ensure;

use super::{
    MapPageFlags, PageTable, PageTableError,
    address::{PhysPageNum, VirtAddr, VirtPageNum},
    table::PageTableRef,
};
use crate::{PAGE_SIZE, address::PhysAddr};

bitflags! {
    /// Flags for SV39 page table entries.
    ///
    /// These flags define the properties and permissions of a page table entry.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(super) struct PageFlags: u64 {
        /// Valid Bit of page table entry.
        ///
        /// If set, an entry for this virtual address exists.
        const V = 1 << 0;

        /// Read Bit of page table entry.
        ///
        /// If set, the CPU can read to this virtual address.
        const R = 1 << 1;

        /// Write Bit of page table entry.
        ///
        /// If set, the CPU can write to this virtual address.
        const W = 1 << 2;

        /// Executable Bit of page table entry.
        ///
        /// If set, the CPU can execute instructions on this virtual address.
        const X = 1 << 3;

        /// UserMode Bit of page table entry.
        ///
        /// If set, userspace can access this virtual address.
        const U = 1 << 4;

        /// Global Mapping Bit of page table entry.
        ///
        /// If set, this virtual address exists in all address spaces.
        const G = 1 << 5;

        /// Access Bit of page table entry.
        ///
        /// If set, this virtual address has been accessed.
        const A = 1 << 6;

        /// Dirty Bit of page table entry.
        ///
        /// If set, this virtual address has been written to.
        const D = 1 << 7;

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

impl From<MapPageFlags> for PageFlags {
    fn from(form: MapPageFlags) -> Self {
        let mut flags = Self::empty();
        if form.contains(MapPageFlags::R) {
            flags |= Self::R;
        }
        if form.contains(MapPageFlags::W) {
            flags |= Self::W;
        }
        if form.contains(MapPageFlags::X) {
            flags |= Self::X;
        }
        if form.contains(MapPageFlags::U) {
            flags |= Self::U;
        }
        flags
    }
}

/// Represents a single SV39 page table entry.
///
/// This structure encapsulates the physical address and flags associated with
/// a page table entry.
#[repr(transparent)]
#[derive(Pod)]
pub(super) struct PageTableEntry(u64);

const FLAGS_MASK: u64 = (1 << 10) - 1;
const FLAGS_SHIFT: usize = 0;
const PHYS_PAGE_NUM_MASK: u64 = ((1 << 44) - 1) << 10;
const PHYS_PAGE_NUM_SHIFT: usize = 10;

const _: () = assert!(FLAGS_MASK.count_ones() == 10);
const _: () = assert!(PHYS_PAGE_NUM_MASK.count_ones() == 44);

pub(super) struct PageTableEntryRef<R> {
    pte: R,
    level: usize,
    base_vpn: VirtPageNum,
}

impl<R> PageTableEntryRef<R> {
    pub(super) fn new(pte: R, level: usize, base_vpn: VirtPageNum) -> Self {
        Self {
            pte,
            level,
            base_vpn,
        }
    }

    pub(super) fn vpn_count(&self) -> usize {
        1 << (self.level * 9)
    }

    pub(super) fn min_vpn(&self) -> VirtPageNum {
        self.base_vpn
    }

    pub(super) fn max_vpn(&self) -> VirtPageNum {
        self.base_vpn.add(self.vpn_count() - 1)
    }

    pub(super) fn min_virt_addr(&self) -> VirtAddr {
        VirtAddr::min_in_page(self.min_vpn())
    }

    pub(super) fn max_virt_addr(&self) -> VirtAddr {
        VirtAddr::max_in_page(self.max_vpn())
    }

    fn page_layout(&self) -> Layout {
        let size = self.vpn_count() * PAGE_SIZE;
        Layout::from_size_align(size, size).unwrap()
    }
}

impl<R> PageTableEntryRef<R>
where
    R: Deref<Target = PageTableEntry>,
{
    pub(super) fn flags(&self) -> PageFlags {
        PageFlags::from_bits_retain((self.pte.0 & FLAGS_MASK) >> FLAGS_SHIFT)
    }

    pub(super) fn is_valid(&self) -> bool {
        self.flags().contains(PageFlags::V)
    }

    pub(super) fn is_leaf(&self) -> bool {
        self.is_valid() && self.flags().intersects(PageFlags::RWX)
    }

    pub(super) fn is_non_leaf(&self) -> bool {
        self.is_valid() && !self.is_leaf()
    }

    pub(super) fn phys_page_num(&self) -> Option<PhysPageNum> {
        self.is_valid()
            .then(|| PhysPageNum::new((self.pte.0 & PHYS_PAGE_NUM_MASK) >> PHYS_PAGE_NUM_SHIFT))
    }

    pub(super) fn phys_addr(&self) -> Option<PhysAddr> {
        self.phys_page_num().map(PhysAddr::min_in_page)
    }

    pub(super) fn min_ppn(&self) -> Option<PhysPageNum> {
        self.phys_page_num()
    }

    pub(super) fn max_ppn(&self) -> Option<PhysPageNum> {
        Some(self.phys_page_num()?.add(self.vpn_count() - 1))
    }

    pub(super) fn min_phys_addr(&self) -> Option<PhysAddr> {
        self.min_ppn().map(PhysAddr::min_in_page)
    }

    pub(super) fn max_phys_addr(&self) -> Option<PhysAddr> {
        self.max_ppn().map(PhysAddr::max_in_page)
    }

    pub(super) fn next_level_table(&self) -> Option<PageTableRef<&PageTable>> {
        if !self.is_non_leaf() {
            return None;
        }
        let ptr = self.phys_addr()?.as_ptr::<PageTable>();
        assert!(ptr.is_aligned());
        let pt = unsafe { ptr.as_ref() }?;
        Some(PageTableRef::new(pt, self.level - 1, self.base_vpn))
    }
}

impl<R> PageTableEntryRef<R>
where
    R: DerefMut<Target = PageTableEntry>,
{
    fn update(&mut self, phys_page_num: PhysPageNum, flags: PageFlags) {
        let phys_page_num_bits = phys_page_num.value() << PHYS_PAGE_NUM_SHIFT;
        assert!(
            phys_page_num_bits & PHYS_PAGE_NUM_MASK == phys_page_num_bits,
            "Physical page number out of bounds"
        );

        let flags_bits = flags.bits() << FLAGS_SHIFT;
        assert!(flags_bits & FLAGS_MASK == flags_bits, "Flags out of bounds");

        *self.pte = PageTableEntry(phys_page_num_bits | flags_bits);
    }

    #[expect(clippy::needless_pass_by_ref_mut)]
    pub(super) fn next_level_table_mut(&mut self) -> Option<PageTableRef<&mut PageTable>> {
        if !self.is_non_leaf() {
            return None;
        }
        let ptr = self.phys_addr()?.as_mut_ptr::<PageTable>();
        assert!(ptr.is_aligned());
        let pt = unsafe { ptr.as_mut() }?;
        Some(PageTableRef::new(pt, self.level - 1, self.base_vpn))
    }

    pub(super) fn get_or_insert_next_level_table(
        &mut self,
    ) -> Result<PageTableRef<&mut PageTable>, PageTableError> {
        ensure!(!self.is_leaf(), super::AlreadyMappedSnafu);

        if !self.is_valid() {
            let next_level_pt = PageTable::try_allocate()?;
            self.set_next_level_table(next_level_pt)?;
        }

        Ok(self.next_level_table_mut().unwrap())
    }

    pub(super) fn set_next_level_table(
        &mut self,
        table: Box<PageTable>,
    ) -> Result<(), PageTableError> {
        ensure!(!self.is_valid(), super::AlreadyMappedSnafu);
        let table = Box::leak(table);
        let flags = PageFlags::V;
        let phys_page_num = PhysAddr::from_ptr(table).page_num();
        self.update(phys_page_num, flags);
        Ok(())
    }

    pub(super) fn allocate_page(&mut self, flags: MapPageFlags) -> Result<(), PageTableError> {
        ensure!(
            !flags.is_empty() && flags & MapPageFlags::URWX == flags,
            super::InvalidMapFlagsSnafu { flags }
        );
        ensure!(!self.is_valid(), super::AlreadyMappedSnafu);

        let layout = self.page_layout();
        let page = unsafe { alloc::alloc::alloc_zeroed(layout) };
        ensure!(!page.is_null(), super::AllocPageSnafu { layout });

        let page_flags = PageFlags::V | PageFlags::from(flags);
        let phys_page_num = PhysAddr::from_ptr(page).page_num();
        self.update(phys_page_num, page_flags);
        Ok(())
    }

    pub(super) fn map_page(
        &mut self,
        phys_page_num: PhysPageNum,
        flags: MapPageFlags,
    ) -> Result<(), PageTableError> {
        ensure!(
            !flags.is_empty() && flags & MapPageFlags::URWX == flags,
            super::InvalidMapFlagsSnafu { flags }
        );
        ensure!(!self.is_valid(), super::AlreadyMappedSnafu);

        let page_flags = PageFlags::V | PageFlags::from(flags);
        self.update(phys_page_num, page_flags);
        Ok(())
    }
}
