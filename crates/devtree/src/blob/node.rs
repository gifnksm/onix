use super::UNIT_ADDRESS_SEPARATOR;
use crate::{polyfill, types::ByteStr};

#[derive(Debug, Clone)]
pub struct Node<'blob> {
    full_name: &'blob ByteStr,
}

impl<'blob> Node<'blob> {
    pub(crate) fn new(full_name: &'blob ByteStr) -> Self {
        Self { full_name }
    }

    #[must_use]
    pub fn full_name(&self) -> &'blob ByteStr {
        self.full_name
    }

    #[must_use]
    pub fn split_name(&self) -> (&'blob ByteStr, Option<&'blob ByteStr>) {
        match polyfill::slice_split_once(self.full_name, |&b| b == UNIT_ADDRESS_SEPARATOR) {
            Some((name, unit_address)) => (ByteStr::new(name), Some(ByteStr::new(unit_address))),
            None => (ByteStr::new(self.full_name), None),
        }
    }

    #[must_use]
    pub fn name(&self) -> &'blob ByteStr {
        self.split_name().0
    }

    #[must_use]
    pub fn unit_address(&self) -> Option<&'blob ByteStr> {
        self.split_name().1
    }

    #[must_use]
    pub fn is_root(&self) -> bool {
        self.full_name().is_empty()
    }
}
