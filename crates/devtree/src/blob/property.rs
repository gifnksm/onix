use core::fmt;

use crate::{polyfill, types::ByteStr};

#[derive(Clone)]
pub struct Property<'blob> {
    name_bytes: &'blob [u8],
    value: &'blob [u8],
}

impl<'blob> Property<'blob> {
    #[must_use]
    pub fn new(name_bytes: &'blob [u8], value: &'blob [u8]) -> Self {
        Self { name_bytes, value }
    }

    #[must_use]
    pub fn name(&self) -> &'blob ByteStr {
        let name = polyfill::slice_split_once(self.name_bytes, |&b| b == 0)
            .map_or(self.name_bytes, |(s, _)| s);
        ByteStr::new(name)
    }

    #[must_use]
    pub fn value(&self) -> &'blob [u8] {
        self.value
    }
}

impl fmt::Debug for Property<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Property")
            .field("name", &self.name())
            .field("value", &ByteStr::new(self.value))
            .finish()
    }
}
