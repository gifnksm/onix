use dataview::Pod;
use endian::Be;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod)]
pub struct TokenType(pub Be<u32>);

impl TokenType {
    pub const BEGIN_NODE: u32 = 0x0000_0001;
    pub const END_NODE: u32 = 0x0000_0002;
    pub const PROP: u32 = 0x0000_0003;
    pub const NOP: u32 = 0x0000_0004;
    pub const END: u32 = 0x0000_0009;
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod)]
pub struct PropertyHeader {
    pub len: Be<u32>,
    pub name_offset: Be<u32>,
}
