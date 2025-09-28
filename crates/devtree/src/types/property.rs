use core::{
    fmt,
    iter::FusedIterator,
    ops::Range,
    ptr::{self},
    slice,
};

use platform_cast::CastFrom as _;

use crate::{polyfill, types::ByteStr};

macro_rules! forward_fmt_impls {
    ($ty:path, $($traits:path),* $(,)?) => {
        $(impl $traits for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Debug::fmt(&self.value(), f)
            }
        })*
    }
}

macro_rules! forward_integral_fmt_impls {
    ($ty:path) => {
        forward_fmt_impls!(
            $ty,
            fmt::Display,
            fmt::Binary,
            fmt::Octal,
            fmt::LowerHex,
            fmt::UpperHex
        );
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PropertyName<'blob> {
    value: &'blob ByteStr,
}

impl<'blob> PropertyName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.value
    }
}

#[repr(transparent)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct U32Array {
    value: [[u8; 4]],
}

impl U32Array {
    #[must_use]
    pub fn new(value: &[[u8; 4]]) -> &Self {
        // SAFETY: U32Array is #[repr(transparent)] over [[u8; 4]]
        #[expect(clippy::missing_panics_doc)]
        unsafe {
            (ptr::from_ref(value) as *const Self).as_ref().unwrap()
        }
    }

    #[must_use]
    pub fn iter(&self) -> U32ArrayIter<'_> {
        self.into_iter()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.value.len()
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<u32> {
        self.value.get(index).copied().map(u32::from_be_bytes)
    }
}

impl<'blob> IntoIterator for &'blob U32Array {
    type Item = u32;
    type IntoIter = U32ArrayIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        U32ArrayIter {
            iter: self.value.iter(),
        }
    }
}

impl fmt::Debug for U32Array {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[derive(Debug)]
pub struct U32ArrayIter<'blob> {
    iter: slice::Iter<'blob, [u8; 4]>,
}

impl Iterator for U32ArrayIter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let bytes = self.iter.next()?;
        Some(u32::from_be_bytes(*bytes))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for U32ArrayIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let bytes = self.iter.next_back()?;
        Some(u32::from_be_bytes(*bytes))
    }
}

impl FusedIterator for U32ArrayIter<'_> {}
impl ExactSizeIterator for U32ArrayIter<'_> {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrList<'blob> {
    value: &'blob str,
}

impl<'blob> StrList<'blob> {
    #[must_use]
    pub fn new(value: &'blob str) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn iter(&self) -> StrListIter<'blob> {
        self.into_iter()
    }
}

impl fmt::Debug for StrList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'blob> IntoIterator for &StrList<'blob> {
    type Item = &'blob str;
    type IntoIter = StrListIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        StrListIter {
            remainder: self.value,
        }
    }
}

#[derive(Clone)]
pub struct StrListIter<'blob> {
    remainder: &'blob str,
}

impl<'blob> Iterator for StrListIter<'blob> {
    type Item = &'blob str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }

        let (s, rest) = self
            .remainder
            .split_once('\0')
            .unwrap_or((self.remainder, ""));
        self.remainder = rest;
        Some(s)
    }
}

impl DoubleEndedIterator for StrListIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }

        let (rest, s) = self
            .remainder
            .rsplit_once('\0')
            .unwrap_or(("", self.remainder));
        self.remainder = rest;
        Some(s)
    }
}

impl FusedIterator for StrListIter<'_> {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteStrList<'blob> {
    value: &'blob ByteStr,
}

impl fmt::Debug for ByteStrList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl<'blob> ByteStrList<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn iter(&self) -> ByteStrListIter<'blob> {
        self.into_iter()
    }
}

impl<'blob> IntoIterator for &ByteStrList<'blob> {
    type Item = &'blob ByteStr;
    type IntoIter = ByteStrListIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        ByteStrListIter {
            remainder: self.value,
        }
    }
}

#[derive(Clone)]
pub struct ByteStrListIter<'blob> {
    remainder: &'blob ByteStr,
}

impl<'blob> Iterator for ByteStrListIter<'blob> {
    type Item = &'blob ByteStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }
        let (s, rest) = polyfill::slice_split_once(self.remainder, |&b| b == 0)
            .unwrap_or((self.remainder, &[]));
        self.remainder = ByteStr::new(rest);
        Some(ByteStr::new(s))
    }
}

impl DoubleEndedIterator for ByteStrListIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.remainder.is_empty() {
            return None;
        }
        let (rest, s) = polyfill::slice_rsplit_once(self.remainder, |&b| b == 0)
            .unwrap_or((&[], self.remainder));
        self.remainder = ByteStr::new(rest);
        Some(ByteStr::new(s))
    }
}

impl FusedIterator for ByteStrListIter<'_> {}

#[derive(Clone, Copy)]
pub struct Compatible<'blob> {
    value: ByteStrList<'blob>,
}

impl fmt::Debug for Compatible<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}

impl<'blob> Compatible<'blob> {
    #[must_use]
    pub fn new(value: ByteStrList<'blob>) -> Self {
        Self { value }
    }

    pub fn is_compatible_to<B>(&self, model: B) -> bool
    where
        B: AsRef<ByteStr>,
    {
        let model = model.as_ref();
        self.value.iter().any(|c| c == model)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Model<'blob> {
    value: &'blob ByteStr,
}

impl fmt::Debug for Model<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.value, f)
    }
}
impl<'blob> Model<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self { value }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Phandle(u32);

impl Phandle {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> u32 {
        self.0
    }
}

forward_integral_fmt_impls!(Phandle);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    #[default]
    Okay,
    Disabled,
    Reserved,
    Fail,
    FailSss,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AddressCells(u32);

impl AddressCells {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> usize {
        usize::cast_from(self.0)
    }
}

forward_integral_fmt_impls!(AddressCells);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SizeCells(u32);

impl SizeCells {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> usize {
        usize::cast_from(self.0)
    }
}

forward_integral_fmt_impls!(SizeCells);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterruptCells(u32);

impl InterruptCells {
    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(self) -> usize {
        usize::cast_from(self.0)
    }
}

forward_integral_fmt_impls!(InterruptCells);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Reg<'blob> {
    address_cells: AddressCells,
    size_cells: SizeCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> IntoIterator for Reg<'blob> {
    type Item = RegValue<'blob>;
    type IntoIter = RegIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        RegIter {
            address_cells: self.address_cells,
            size_cells: self.size_cells,
            value: self.value,
        }
    }
}

impl fmt::Debug for Reg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl<'blob> Reg<'blob> {
    /// # Panics
    ///
    /// Panics if `value.len()` is not a multiple of `address_cells +
    /// size_cells`.
    #[must_use]
    pub fn new(
        address_cells: AddressCells,
        size_cells: SizeCells,
        value: &'blob [[u8; 4]],
    ) -> Self {
        assert!(
            value
                .len()
                .is_multiple_of(address_cells.value() + size_cells.value())
        );
        Self {
            address_cells,
            size_cells,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegValue<'blob> {
    pub address: &'blob U32Array,
    pub size: &'blob U32Array,
}

impl RegValue<'_> {
    #[must_use]
    pub fn range(&self) -> Range<usize> {
        let address = self
            .address
            .iter()
            .fold(0, |acc, x| (acc << 32) | usize::cast_from(x));
        let size = self
            .size
            .iter()
            .fold(0, |acc, x| (acc << 32) | usize::cast_from(x));
        address..address.saturating_add(size)
    }
}

#[derive(Debug, Clone)]
pub struct RegIter<'blob> {
    address_cells: AddressCells,
    size_cells: SizeCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> Iterator for RegIter<'blob> {
    type Item = RegValue<'blob>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let value = self.value;
        let (address, value) = value.split_at(self.address_cells.value());
        let (size, value) = value.split_at(self.size_cells.value());
        self.value = value;
        let address = U32Array::new(address);
        let size = U32Array::new(size);

        Some(RegValue { address, size })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.value.len() / (self.address_cells.value() + self.size_cells.value());
        (len, Some(len))
    }
}

impl DoubleEndedIterator for RegIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let value = self.value;
        let (value, size) = value.split_at(value.len() - self.size_cells.value());
        let (value, address) = value.split_at(value.len() - self.address_cells.value());
        self.value = value;
        let address = U32Array::new(address);
        let size = U32Array::new(size);

        Some(RegValue { address, size })
    }
}

impl ExactSizeIterator for RegIter<'_> {}
impl FusedIterator for RegIter<'_> {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ranges<'blob> {
    child_address_cells: AddressCells,
    child_size_cells: SizeCells,
    parent_address_cells: AddressCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> IntoIterator for Ranges<'blob> {
    type Item = RangesValue<'blob>;
    type IntoIter = RangesIter<'blob>;

    fn into_iter(self) -> Self::IntoIter {
        RangesIter {
            child_address_cells: self.child_address_cells,
            child_size_cells: self.child_size_cells,
            parent_address_cells: self.parent_address_cells,
            value: self.value,
        }
    }
}

impl fmt::Debug for Ranges<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl<'blob> Ranges<'blob> {
    /// # Panics
    ///
    /// Panics if `value.len()` is not a multiple of `child_address_cells +
    /// child_size_cells + parent_address_cells`.
    #[must_use]
    pub fn new(
        child_address_cells: AddressCells,
        child_size_cells: SizeCells,
        parent_address_cells: AddressCells,
        value: &'blob [[u8; 4]],
    ) -> Self {
        assert!(value.len().is_multiple_of(
            child_address_cells.value() + child_size_cells.value() + parent_address_cells.value()
        ));
        Self {
            child_address_cells,
            child_size_cells,
            parent_address_cells,
            value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RangesValue<'blob> {
    pub child_bus_address: &'blob U32Array,
    pub parent_bus_address: &'blob U32Array,
    pub len: &'blob U32Array,
}

#[derive(Debug, Clone)]
pub struct RangesIter<'blob> {
    child_address_cells: AddressCells,
    child_size_cells: SizeCells,
    parent_address_cells: AddressCells,
    value: &'blob [[u8; 4]],
}

impl<'blob> Iterator for RangesIter<'blob> {
    type Item = RangesValue<'blob>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let value = self.value;
        let (child_bus_address, value) = value.split_at(self.child_address_cells.value());
        let (parent_bus_address, value) = value.split_at(self.parent_address_cells.value());
        let (len, value) = value.split_at(self.child_size_cells.value());
        self.value = value;
        let child_bus_address = U32Array::new(child_bus_address);
        let parent_bus_address = U32Array::new(parent_bus_address);
        let len = U32Array::new(len);

        Some(RangesValue {
            child_bus_address,
            parent_bus_address,
            len,
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.value.len()
            / (self.child_address_cells.value()
                + self.child_size_cells.value()
                + self.parent_address_cells.value());
        (len, Some(len))
    }
}

impl DoubleEndedIterator for RangesIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.value.is_empty() {
            return None;
        }

        let value = self.value;
        let (value, len) = value.split_at(value.len() - self.child_size_cells.value());
        let (value, parent_bus_address) =
            value.split_at(value.len() - self.parent_address_cells.value());
        let (value, child_bus_address) =
            value.split_at(value.len() - self.child_address_cells.value());
        self.value = value;
        let child_bus_address = U32Array::new(child_bus_address);
        let parent_bus_address = U32Array::new(parent_bus_address);
        let len = U32Array::new(len);

        Some(RangesValue {
            child_bus_address,
            parent_bus_address,
            len,
        })
    }
}

impl ExactSizeIterator for RangesIter<'_> {}
impl FusedIterator for RangesIter<'_> {}
