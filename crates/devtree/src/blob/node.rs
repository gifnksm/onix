use super::{Devicetree, UNIT_ADDRESS_SEPARATOR};
use crate::{cursor::TokenCursor, types::ByteStr, utils};

#[derive(Debug, Clone)]
pub struct Node<'blob> {
    full_name: &'blob ByteStr,
    items_start_cursor: TokenCursor<'blob>,
}

impl<'blob> Node<'blob> {
    pub(crate) fn new(full_name: &'blob ByteStr, items_start_cursor: TokenCursor<'blob>) -> Self {
        Self {
            full_name,
            items_start_cursor,
        }
    }

    #[must_use]
    pub fn full_name(&self) -> &'blob ByteStr {
        self.full_name
    }

    #[must_use]
    pub fn split_name(&self) -> (&'blob ByteStr, Option<&'blob ByteStr>) {
        match utils::slice_split_once(self.full_name, |&b| b == UNIT_ADDRESS_SEPARATOR) {
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
    pub fn devicetree(&self) -> &'blob Devicetree {
        self.items_start_cursor.devicetree()
    }

    #[must_use]
    pub(crate) fn items_start_cursor(&self) -> &TokenCursor<'blob> {
        &self.items_start_cursor
    }
}
