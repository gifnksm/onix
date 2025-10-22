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

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use dataview::PodMethods as _;

    use super::*;

    #[test]
    fn test_token_type_new_and_value() {
        let token = TokenType::new(TokenType::BEGIN_NODE);
        assert_eq!(token.value(), TokenType::BEGIN_NODE);
        assert_eq!(token.as_bytes(), TokenType::BEGIN_NODE.to_be_bytes());

        let token = TokenType::new(TokenType::END_NODE);
        assert_eq!(token.value(), TokenType::END_NODE);
        assert_eq!(token.as_bytes(), TokenType::END_NODE.to_be_bytes());

        let token = TokenType::new(TokenType::PROP);
        assert_eq!(token.value(), TokenType::PROP);
        assert_eq!(token.as_bytes(), TokenType::PROP.to_be_bytes());

        let token = TokenType::new(TokenType::NOP);
        assert_eq!(token.value(), TokenType::NOP);
        assert_eq!(token.as_bytes(), TokenType::NOP.to_be_bytes());

        let token = TokenType::new(TokenType::END);
        assert_eq!(token.value(), TokenType::END);
        assert_eq!(token.as_bytes(), TokenType::END.to_be_bytes());
    }

    #[test]
    fn test_token_type_equality() {
        let t1 = TokenType::new(TokenType::BEGIN_NODE);
        let t2 = TokenType::new(TokenType::BEGIN_NODE);
        let t3 = TokenType::new(TokenType::END_NODE);
        assert_eq!(t1, t2);
        assert_ne!(t1, t3);
    }

    #[test]
    fn test_property_header_new_and_accessors() {
        let len = 42;
        let name_offset = 100;
        let header = PropertyHeader::new(len, name_offset);
        assert_eq!(header.len(), len);
        assert_eq!(header.name_offset(), name_offset);
        assert_eq!(header.as_bytes(), &[0, 0, 0, 42, 0, 0, 0, 100]);
    }

    #[test]
    fn test_property_header_equality() {
        let h1 = PropertyHeader::new(10, 20);
        let h2 = PropertyHeader::new(10, 20);
        let h3 = PropertyHeader::new(30, 40);
        assert_eq!(h1.len(), h2.len());
        assert_eq!(h1.name_offset(), h2.name_offset());
        assert_ne!(h1.len(), h3.len());
        assert_ne!(h1.name_offset(), h3.name_offset());
    }
}
