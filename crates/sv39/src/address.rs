use core::{
    fmt,
    ops::{Add, Sub},
    ptr,
};

use platform_cast::{CastFrom as _, CastInto};

use crate::{PAGE_SHIFT, PAGE_SIZE};

macro_rules! impl_hex {
    ($ty:ty) => {
        impl fmt::LowerHex for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::LowerHex::fmt(&self.0, f)
            }
        }

        impl fmt::UpperHex for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::UpperHex::fmt(&self.0, f)
            }
        }
    };
}

macro_rules! impl_pointer {
    ($ty:ty) => {
        impl fmt::Pointer for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let ptr = &ptr::without_provenance::<u8>(self.0.cast_into());
                fmt::Pointer::fmt(ptr, f)
            }
        }
    };
}

/// Physical page number in the SV39 virtual memory system.
///
/// Represents a 44-bit physical page number used in RISC-V SV39 paging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysPageNum(u64);
impl_hex!(PhysPageNum);

impl PhysPageNum {
    /// The minimum valid physical page number (0).
    pub const MIN: Self = Self(0);
    /// The maximum valid physical page number (2^44 - 1).
    pub const MAX: Self = Self((1 << 44) - 1);

    /// Creates a new physical page number.
    ///
    /// # Panics
    ///
    /// Panics if `page_num` exceeds the maximum valid physical page number.
    #[must_use]
    pub fn new(page_num: u64) -> Self {
        assert!(
            page_num <= Self::MAX.value(),
            "Physical page number must be less than 2^44"
        );
        Self(page_num)
    }

    /// Returns the raw page number value.
    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }

    /// Checks if this page number is aligned to the specified level boundary.
    ///
    /// # Panics
    ///
    /// Panics if level is greater than 2.
    #[must_use]
    pub fn is_level_aligned(self, level: usize) -> bool {
        assert!(level <= 2, "Level must be 0, 1, or 2");
        self.0.is_multiple_of(1 << (level * 9))
    }

    /// Safely adds a number of pages to this page number.
    ///
    /// Returns `None` if the addition would overflow.
    #[must_use]
    pub fn checked_add(self, pages: usize) -> Option<Self> {
        self.0.checked_add(u64::cast_from(pages)).map(Self)
    }

    /// Safely subtracts another page number from this one.
    ///
    /// Returns `None` if the subtraction would underflow.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<usize> {
        self.0.checked_sub(rhs.0).map(CastInto::cast_into)
    }
}

impl Add<usize> for PhysPageNum {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl Sub<Self> for PhysPageNum {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

/// Physical address in the SV39 virtual memory system.
///
/// Represents a complete physical address with both page number and offset
/// components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct PhysAddr(u64);
impl_hex!(PhysAddr);
impl_pointer!(PhysAddr);

impl PhysAddr {
    const OFFSET_SHIFT: usize = 0;
    const PPN_SHIFT: usize = PAGE_SHIFT;
    const OFFSET_MASK: u64 = ((1 << PAGE_SHIFT) - 1) << Self::OFFSET_SHIFT;
    const PPN_MASK: u64 = ((1 << 27) - 1) << Self::PPN_SHIFT;

    /// Creates a physical address from a raw address value.
    ///
    /// # Panics
    ///
    /// Panics if `addr` exceeds the maximum valid physical address.
    #[must_use]
    pub fn from_addr(addr: usize) -> Self {
        assert!(addr < (1 << 56), "Physical address must be less than 2^56");
        Self(addr.cast_into())
    }

    /// Creates a physical address from a pointer.
    ///
    /// # Panics
    ///
    /// Panics if `addr` exceeds the maximum valid physical address.
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::from_addr(ptr.addr())
    }

    /// Creates a physical address from a page number and offset.
    ///
    /// # Panics
    ///
    /// Panics if offset is greater than or equal to `PAGE_SIZE`.
    #[must_use]
    pub fn from_parts(page_num: PhysPageNum, offset: usize) -> Self {
        assert!(offset < PAGE_SIZE, "Offset must be less than 2^12");
        let addr = (page_num.value() << Self::PPN_SHIFT) | u64::cast_from(offset);
        Self::from_addr(addr.cast_into())
    }

    /// Returns the minimum address within the specified page.
    #[must_use]
    pub fn min_in_page(page_num: PhysPageNum) -> Self {
        Self::from_parts(page_num, 0)
    }

    /// Returns the maximum address within the specified page.
    #[must_use]
    pub fn max_in_page(page_num: PhysPageNum) -> Self {
        Self::from_parts(page_num, PAGE_SIZE - 1)
    }

    /// Converts this address to a const pointer.
    #[must_use]
    pub fn as_ptr<T>(self) -> *const T {
        ptr::with_exposed_provenance(self.0.cast_into())
    }

    /// Converts this address to a mutable pointer.
    #[must_use]
    pub fn as_mut_ptr<T>(self) -> *mut T {
        ptr::with_exposed_provenance_mut(self.0.cast_into())
    }

    /// Extracts the page number from this address.
    #[must_use]
    pub fn page_num(self) -> PhysPageNum {
        PhysPageNum::new((self.0 & Self::PPN_MASK) >> Self::PPN_SHIFT)
    }

    /// Extracts the offset within the page from this address.
    #[must_use]
    pub fn offset(self) -> usize {
        ((self.0 & Self::OFFSET_MASK) >> Self::OFFSET_SHIFT).cast_into()
    }

    /// Safely subtracts another address from this one.
    ///
    /// Returns `None` if the subtraction would underflow.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<usize> {
        self.0.checked_sub(rhs.0).map(CastInto::cast_into)
    }
}

/// Virtual page number in the SV39 virtual memory system.
///
/// Represents a 27-bit virtual page number used in RISC-V SV39 paging.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtPageNum(u64);
impl_hex!(VirtPageNum);

impl VirtPageNum {
    /// The minimum valid virtual page number (0).
    pub const MIN: Self = Self(0);
    /// The maximum valid virtual page number (2^27 - 1).
    pub const MAX: Self = Self((1 << 27) - 1);

    /// Creates a new virtual page number.
    ///
    /// # Panics
    ///
    /// Panics if `page_num` exceeds the maximum valid virtual page number.
    #[must_use]
    pub fn new(page_num: u64) -> Self {
        assert!(
            page_num <= Self::MAX.value(),
            "Virtual page number must be less than 2^27"
        );
        Self(page_num)
    }

    /// Returns the raw page number value.
    #[must_use]
    pub fn value(self) -> u64 {
        self.0
    }

    /// Checks if this page number is aligned to the specified level boundary.
    ///
    /// # Panics
    ///
    /// Panics if level is greater than 2.
    #[must_use]
    pub fn is_level_aligned(self, level: usize) -> bool {
        assert!(level <= 2, "Level must be 0, 1, or 2");
        self.0.is_multiple_of(1 << (level * 9))
    }

    /// Safely adds a number of pages to this page number.
    ///
    /// Returns `None` if the addition would overflow.
    #[must_use]
    pub fn checked_add(self, pages: usize) -> Option<Self> {
        self.0.checked_add(u64::cast_from(pages)).map(Self)
    }

    /// Safely subtracts another page number from this one.
    ///
    /// Returns `None` if the subtraction would underflow.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<usize> {
        self.0.checked_sub(rhs.0).map(CastInto::cast_into)
    }

    /// Adds a level index to this page number at the specified level.
    ///
    /// # Panics
    ///
    /// Panics if level is greater than 2 or index is greater than or equal to
    /// 512.
    #[must_use]
    pub fn add_level_index(self, level: usize, index: usize) -> Self {
        assert!(level <= 2, "Level must be 0, 1, or 2");
        assert!(index < (1 << 9), "Index must be less than 512");
        self.add(index << (level * 9))
    }

    /// Extracts the index for the specified page table level.
    ///
    /// # Panics
    ///
    /// Panics if level is greater than 2.
    #[must_use]
    pub fn level_index(self, level: usize) -> usize {
        assert!(level <= 2);
        ((self.0 >> (level * 9)) & 0x1ff).cast_into()
    }
}

impl Add<usize> for VirtPageNum {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        self.checked_add(rhs).unwrap()
    }
}

impl Sub<Self> for VirtPageNum {
    type Output = usize;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs).unwrap()
    }
}

/// Virtual address in the SV39 virtual memory system.
///
/// Represents a complete virtual address with both page number and offset
/// components. The address is sign-extended according to SV39 specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VirtAddr(u64);
impl_hex!(VirtAddr);
impl_pointer!(VirtAddr);

impl VirtAddr {
    const OFFSET_SHIFT: usize = 0;
    const VPN_SHIFT: usize = PAGE_SHIFT;
    const OFFSET_MASK: u64 = ((1 << PAGE_SHIFT) - 1) << Self::OFFSET_SHIFT;
    const VPN_MASK: u64 = ((1 << 27) - 1) << Self::VPN_SHIFT;

    /// Creates a virtual address from a raw address value.
    ///
    /// The address must be properly sign-extended according to SV39
    /// specification.
    ///
    /// # Panics
    ///
    /// Panics if the address is not properly sign-extended.
    #[must_use]
    pub fn from_addr(addr: usize) -> Self {
        let addr = addr.cast_into();
        let extended_addr = Self::sign_extend(addr);
        assert_eq!(addr, extended_addr, "Address must be sign-extended");
        Self(addr)
    }

    /// Creates a virtual address from a pointer.
    #[must_use]
    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self::from_addr(ptr.addr())
    }

    /// Creates a virtual address from a page number and offset.
    ///
    /// # Panics
    ///
    /// Panics if offset is greater than or equal to `PAGE_SIZE`.
    #[must_use]
    pub fn from_parts(page_num: VirtPageNum, offset: usize) -> Self {
        assert!(offset < PAGE_SIZE, "Offset must be less than 2^12");
        let addr = (page_num.value() << Self::VPN_SHIFT) | u64::cast_from(offset);
        Self(Self::sign_extend(addr))
    }

    /// Returns the minimum address within the specified page.
    #[must_use]
    pub fn min_in_page(page_num: VirtPageNum) -> Self {
        Self::from_parts(page_num, 0)
    }

    /// Returns the maximum address within the specified page.
    #[must_use]
    pub fn max_in_page(page_num: VirtPageNum) -> Self {
        Self::from_parts(page_num, PAGE_SIZE - 1)
    }

    fn sign_extend(addr: u64) -> u64 {
        const HIGH_MASK: u64 = !((1 << 39) - 1);
        const _: () = assert!(HIGH_MASK.count_ones() == 64 - 39);

        let bit38_on = (addr & (1 << 38)) != 0;
        if bit38_on {
            addr | HIGH_MASK
        } else {
            addr & !HIGH_MASK
        }
    }

    /// Extracts the page number from this address.
    #[must_use]
    pub fn page_num(self) -> VirtPageNum {
        VirtPageNum::new((self.0 & Self::VPN_MASK) >> Self::VPN_SHIFT)
    }

    /// Extracts the offset within the page from this address.
    #[must_use]
    pub fn offset(self) -> usize {
        ((self.0 & Self::OFFSET_MASK) >> Self::OFFSET_SHIFT).cast_into()
    }

    /// Safely subtracts another address from this one.
    ///
    /// Returns `None` if the subtraction would underflow.
    #[must_use]
    pub fn checked_sub(self, rhs: Self) -> Option<usize> {
        self.0.checked_sub(rhs.0).map(CastInto::cast_into)
    }

    /// Returns the raw address value.
    #[must_use]
    pub fn value(self) -> usize {
        self.0.cast_into()
    }
}
