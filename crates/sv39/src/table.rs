use alloc::boxed::Box;
use core::{
    iter::{Enumerate, FusedIterator},
    ops::{Deref, DerefMut},
    slice,
};

use dataview::Pod;
use snafu::ResultExt as _;

use super::{
    MapPageFlags, PageTableError,
    address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum},
    entry::{PageTableEntry, PageTableEntryRef},
};

const NUM_ENTRIES: usize = 512;

#[repr(C, align(4096))]
#[derive(Pod)]
pub(super) struct PageTable([PageTableEntry; NUM_ENTRIES]);

impl PageTable {
    pub(super) fn try_allocate() -> Result<Box<Self>, PageTableError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use super::page_table_error::*;

        let pt = Box::try_new_zeroed().context(AllocPageTableSnafu)?;
        Ok(unsafe { pt.assume_init() })
    }
}

pub(super) struct PageTableRef<R> {
    pt: R,
    level: usize,
    base_vpn: VirtPageNum,
}

impl<R> PageTableRef<R> {
    pub(super) fn new(pt: R, level: usize, base_vpn: VirtPageNum) -> Self {
        assert!(level <= 2);
        Self {
            pt,
            level,
            base_vpn,
        }
    }

    pub(super) fn entry_base_vpn(&self, index: usize) -> VirtPageNum {
        assert!(index < NUM_ENTRIES);
        self.base_vpn.add_level_index(self.level, index)
    }
}

impl<R> PageTableRef<R>
where
    R: Deref<Target = PageTable>,
{
    pub(super) fn entries<'pt>(&'pt self) -> Entries<'pt>
    where
        R: 'pt,
    {
        Entries {
            iter: self.pt.0.iter().enumerate(),
            level: self.level,
            base_vpn: self.base_vpn,
        }
    }

    pub(super) fn phys_page_num(&self) -> PhysPageNum {
        let pt: &PageTable = &self.pt;
        PhysAddr::from_ptr(pt).page_num()
    }

    pub(super) fn phys_addr(&self) -> PhysAddr {
        PhysAddr::min_in_page(self.phys_page_num())
    }

    fn entry(&self, index: usize) -> PageTableEntryRef<&PageTableEntry> {
        PageTableEntryRef::new(&self.pt.0[index], self.level, self.entry_base_vpn(index))
    }

    pub(super) fn min_vpn(&self) -> VirtPageNum {
        self.entry(0).min_vpn()
    }

    pub(super) fn max_vpn(&self) -> VirtPageNum {
        self.entry(NUM_ENTRIES - 1).max_vpn()
    }

    pub(super) fn min_virt_addr(&self) -> VirtAddr {
        VirtAddr::min_in_page(self.min_vpn())
    }

    pub(super) fn max_virt_addr(&self) -> VirtAddr {
        VirtAddr::max_in_page(self.max_vpn())
    }
}

impl<R> PageTableRef<R>
where
    R: DerefMut<Target = PageTable>,
{
    fn entry_mut(&mut self, index: usize) -> PageTableEntryRef<&mut PageTableEntry> {
        let level = self.level;
        let base_vpn = self.entry_base_vpn(index);
        PageTableEntryRef::new(&mut self.pt.0[index], level, base_vpn)
    }

    pub(super) fn allocate_pages(
        &mut self,
        vpn_base: VirtPageNum,
        count: usize,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        let page_count_per_entry = 1 << (self.level * 9);

        let mut mapped_count = 0;
        for level_index in vpn_base.level_index(self.level)..NUM_ENTRIES {
            if mapped_count >= count {
                break;
            }

            let vpn = vpn_base + mapped_count;
            assert_eq!(level_index, vpn.level_index(self.level));
            assert!(self.min_vpn() <= vpn && vpn <= self.max_vpn());

            if vpn.is_level_aligned(self.level) && (count - mapped_count) >= page_count_per_entry {
                self.entry_mut(level_index).allocate_page(flags)?;
                mapped_count += page_count_per_entry;
                continue;
            }

            mapped_count += self
                .entry_mut(level_index)
                .get_or_insert_next_level_table()?
                .allocate_pages(vpn, count - mapped_count, flags)?;
        }
        assert!(mapped_count <= count);

        Ok(mapped_count)
    }

    pub(super) fn map_fixed_pages(
        &mut self,
        vpn_base: VirtPageNum,
        ppn_base: PhysPageNum,
        count: usize,
        flags: MapPageFlags,
    ) -> Result<usize, PageTableError> {
        let page_count_per_entry = 1 << (self.level * 9);

        let mut mapped_count = 0;
        for level_index in vpn_base.level_index(self.level)..NUM_ENTRIES {
            if mapped_count >= count {
                break;
            }

            let vpn = vpn_base + mapped_count;
            let ppn = ppn_base + mapped_count;
            assert_eq!(level_index, vpn.level_index(self.level));
            assert!(self.min_vpn() <= vpn && vpn <= self.max_vpn());

            if vpn.is_level_aligned(self.level)
                && ppn.is_level_aligned(self.level)
                && (count - mapped_count) >= page_count_per_entry
            {
                self.entry_mut(level_index).map_page(ppn, flags)?;
                mapped_count += page_count_per_entry;
                continue;
            }

            mapped_count += self
                .entry_mut(level_index)
                .get_or_insert_next_level_table()?
                .map_fixed_pages(vpn, ppn, count - mapped_count, flags)?;
        }
        assert!(mapped_count <= count);

        Ok(mapped_count)
    }
}

pub(super) struct Entries<'pt> {
    iter: Enumerate<slice::Iter<'pt, PageTableEntry>>,
    level: usize,
    base_vpn: VirtPageNum,
}

impl<'pt> Iterator for Entries<'pt> {
    type Item = PageTableEntryRef<&'pt PageTableEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(index, entry)| {
            PageTableEntryRef::new(
                entry,
                self.level,
                self.base_vpn.add_level_index(self.level, index),
            )
        })
    }
}

impl FusedIterator for Entries<'_> {}
