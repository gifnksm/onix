#[cfg(feature = "alloc")]
pub use self::alloc::*;
use crate::types::ByteStr;

#[cfg(feature = "alloc")]
mod alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeFullName<'blob>(&'blob ByteStr);

impl<'blob> NodeFullName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeName<'blob>(&'blob ByteStr);

impl<'blob> NodeName<'blob> {
    #[must_use]
    pub fn new(value: &'blob ByteStr) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &'blob ByteStr {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeUnitAddress<'blob>(Option<&'blob ByteStr>);

impl<'blob> NodeUnitAddress<'blob> {
    #[must_use]
    pub fn new(value: Option<&'blob ByteStr>) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> Option<&'blob ByteStr> {
        self.0
    }
}
