use dataview::Pod;
use endian::Be;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod)]
pub struct TokenType(Be<u32>);

impl TokenType {
    pub const BEGIN_NODE: u32 = 0x0000_0001;
    pub const END_NODE: u32 = 0x0000_0002;
    pub const PROP: u32 = 0x0000_0003;
    pub const NOP: u32 = 0x0000_0004;
    pub const END: u32 = 0x0000_0009;

    #[must_use]
    pub fn new(value: u32) -> Self {
        Self(Be::new(&value))
    }

    #[must_use]
    pub fn value(&self) -> u32 {
        self.0.read()
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod)]
pub struct PropertyHeader {
    len: Be<u32>,
    name_offset: Be<u32>,
}

impl PropertyHeader {
    #[must_use]
    pub fn new(len: u32, name_offset: u32) -> Self {
        Self {
            len: Be::new(&len),
            name_offset: Be::new(&name_offset),
        }
    }

    #[must_use]
    #[expect(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len.read()
    }

    #[must_use]
    pub fn name_offset(&self) -> u32 {
        self.name_offset.read()
    }
}
