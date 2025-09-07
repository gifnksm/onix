use super::Devicetree;
use crate::{types::ByteStr, utils};

#[derive(Debug, Clone)]
pub struct Property<'blob> {
    devicetree: &'blob Devicetree,
    name_offset: usize,
    value: &'blob [u8],
}

impl<'blob> Property<'blob> {
    #[must_use]
    pub fn new(devicetree: &'blob Devicetree, name_offset: usize, value: &'blob [u8]) -> Self {
        Self {
            devicetree,
            name_offset,
            value,
        }
    }

    #[must_use]
    pub fn name(&self) -> &'blob ByteStr {
        let name = self.devicetree.strings_block()[self.name_offset..].as_ref();
        let name = utils::slice_split_once(name, |&b| b == 0).map_or(name, |(s, _)| s);
        ByteStr::new(name)
    }

    #[must_use]
    pub fn value(&self) -> &'blob [u8] {
        self.value
    }
}
