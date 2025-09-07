use alloc::vec::Vec;

use crate::types::{ByteStr, ByteString, property::U32Array};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodePath(pub ByteString);

impl NodePath {
    #[must_use]
    pub fn new(value: ByteString) -> Self {
        Self(value)
    }

    #[must_use]
    pub fn value(&self) -> &ByteStr {
        ByteStr::new(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Interrupt<'blob> {
    parent_path: ByteString,
    specifier: &'blob U32Array,
}

impl<'blob> Interrupt<'blob> {
    #[must_use]
    pub fn new(parent_path: ByteString, specifier: &'blob U32Array) -> Self {
        Self {
            parent_path,
            specifier,
        }
    }

    #[must_use]
    pub fn parent_path(&self) -> &ByteStr {
        self.parent_path.as_ref()
    }

    #[must_use]
    pub fn specifier(&self) -> &U32Array {
        self.specifier
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InterruptGeneratingDevice<'blob> {
    interrupts: Vec<Interrupt<'blob>>,
}

impl<'blob> InterruptGeneratingDevice<'blob> {
    #[must_use]
    pub fn new(interrupts: Vec<Interrupt<'blob>>) -> Self {
        Self { interrupts }
    }

    #[must_use]
    pub fn interrupts(&self) -> &[Interrupt<'blob>] {
        &self.interrupts
    }
}
