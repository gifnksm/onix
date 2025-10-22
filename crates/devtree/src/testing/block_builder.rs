extern crate alloc;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use core::iter;

use dataview::PodMethods as _;

use crate::{
    blob::struct_block::{PropertyHeader, TokenType},
    util::AlignedByteBuffer,
};

#[derive(Debug, Clone)]
pub struct BlockBuilder {
    struct_block: Vec<u8>,
    strings_block: Vec<u8>,
    name_offset: BTreeMap<&'static [u8], u32>,
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            struct_block: Vec::new(),
            strings_block: Vec::new(),
            name_offset: BTreeMap::new(),
        }
    }

    pub fn extend_struct_block<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.struct_block.extend(iter);
        self
    }

    pub fn extend_struct_block_from_slice(&mut self, slice: &[u8]) -> &mut Self {
        self.struct_block.extend_from_slice(slice);
        self
    }

    pub fn extend_strings_block<I>(&mut self, iter: I) -> &mut Self
    where
        I: IntoIterator<Item = u8>,
    {
        self.strings_block.extend(iter);
        self
    }

    pub fn extend_strings_block_from_slice(&mut self, slice: &[u8]) -> &mut Self {
        self.strings_block.extend_from_slice(slice);
        self
    }

    pub fn token(&mut self, token: u32) -> &mut Self {
        self.pad_to(align_of::<TokenType>())
            .extend_struct_block_from_slice(TokenType::new(token).as_bytes())
    }

    pub fn pad_to(&mut self, align: usize) -> &mut Self {
        let rem = self.struct_block.len() % align;
        if rem != 0 {
            self.struct_block.extend(iter::repeat_n(0, align - rem));
        }
        self
    }

    pub fn begin_node(&mut self, name: &[u8]) -> &mut Self {
        self.token(TokenType::BEGIN_NODE)
            .extend_struct_block_from_slice(name)
            .extend_struct_block(iter::once(0))
    }

    pub fn end_node(&mut self) -> &mut Self {
        self.token(TokenType::END_NODE)
    }

    pub fn prop_raw(&mut self, name_offset: u32, value: &[u8]) -> &mut Self {
        #[expect(clippy::missing_panics_doc)]
        let len = u32::try_from(value.len()).unwrap();
        self.token(TokenType::PROP)
            .extend_struct_block_from_slice(PropertyHeader::new(len, name_offset).as_bytes())
            .extend_struct_block_from_slice(value)
    }

    pub fn prop(&mut self, name: &'static [u8], value: &[u8]) -> &mut Self {
        let name_offset = *self.name_offset.entry(name).or_insert_with(|| {
            #[expect(clippy::missing_panics_doc)]
            let name_offset = u32::try_from(self.strings_block.len()).unwrap();
            self.strings_block.extend_from_slice(name);
            self.strings_block.push(0);
            name_offset
        });
        self.prop_raw(name_offset, value)
    }

    pub fn nop(&mut self) -> &mut Self {
        self.token(TokenType::NOP)
    }

    pub fn end(&mut self) -> &mut Self {
        self.token(TokenType::END)
    }

    #[must_use]
    pub fn build(&self) -> (AlignedByteBuffer<{ align_of::<TokenType>() }>, Vec<u8>) {
        (
            AlignedByteBuffer::from_slice(&self.struct_block),
            self.strings_block.clone(),
        )
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_and_new_are_equivalent() {
        let builder_default = BlockBuilder::default();
        let builder_new = BlockBuilder::new();
        assert_eq!(builder_default.struct_block, builder_new.struct_block);
        assert_eq!(builder_default.strings_block, builder_new.strings_block);
        assert_eq!(builder_default.name_offset, builder_new.name_offset);
    }

    #[test]
    fn test_extend_struct_block_and_extend_struct_block_from_slice() {
        let mut builder = BlockBuilder::new();
        builder.extend_struct_block([1, 2, 3]);
        builder.extend_struct_block_from_slice(&[4, 5]);
        assert_eq!(builder.struct_block, [1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_extend_strings_block_and_extend_strings_block_from_slice() {
        let mut builder = BlockBuilder::new();
        builder.extend_strings_block([10, 20]);
        builder.extend_strings_block_from_slice(&[30, 40]);
        assert_eq!(builder.strings_block, [10, 20, 30, 40]);
    }

    #[test]
    fn test_pad_to_alignment() {
        let mut builder = BlockBuilder::new();
        builder.extend_struct_block([1, 2, 3]);
        let len_before = builder.struct_block.len();
        builder.pad_to(4);
        let len_after = builder.struct_block.len();
        assert_eq!(len_after % 4, 0);
        assert!(len_after >= len_before);
    }

    #[test]
    fn test_token_adds_token_type() {
        let mut builder = BlockBuilder::new();
        builder.token(0x1234_5678);
        assert!(builder.struct_block.len() >= size_of::<TokenType>());
    }

    #[test]
    fn test_begin_and_end_node() {
        let mut builder = BlockBuilder::new();
        builder.begin_node(b"node_name");
        builder.end_node();
        let (struct_block, _) = builder.build();
        assert!(!struct_block.is_empty());
    }

    #[test]
    fn test_prop_and_prop_raw() {
        let mut builder = BlockBuilder::new();
        let name = b"property";
        let value = &[1, 2, 3, 4];
        builder.prop(name, value);
        let (struct_block, strings_block) = builder.build();
        assert!(!struct_block.is_empty());
        assert!(strings_block.ends_with(b"property\0"));
    }

    #[test]
    fn test_nop_and_end() {
        let mut builder = BlockBuilder::new();
        builder.nop();
        builder.end();
        let (struct_block, _) = builder.build();
        assert!(struct_block.len() >= 2 * size_of::<TokenType>());
    }

    #[test]
    fn test_build_returns_cloned_blocks() {
        let mut builder = BlockBuilder::new();
        builder.extend_struct_block([1, 2, 3]);
        builder.extend_strings_block([4, 5, 6]);
        let (struct_block, strings_block) = builder.build();
        assert_eq!(struct_block.as_ref(), [1, 2, 3]);
        assert_eq!(strings_block, [4, 5, 6]);
    }

    #[test]
    fn test_prop_reuses_name_offset() {
        let mut builder = BlockBuilder::new();
        let name = b"shared";
        builder.prop(name, &[1]);
        let offset1 = builder.name_offset[&name[..]];
        builder.prop(name, &[2]);
        let offset2 = builder.name_offset[&name[..]];
        assert_eq!(offset1, offset2);
    }
}
