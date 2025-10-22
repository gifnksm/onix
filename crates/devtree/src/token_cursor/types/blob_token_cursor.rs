use core::iter::FusedIterator;

use dataview::{DataView, Pod};
use platform_cast::CastFrom as _;

use crate::{
    blob::{
        Node, PATH_SEPARATOR, Property,
        struct_block::{PropertyHeader, TokenType},
    },
    debug::SliceDebug as _,
    polyfill,
    token_cursor::{
        Token, TokenCursor,
        error::{
            ReadBeginNodeTokenError, ReadBeginNodeTokenErrorKind, ReadPropTokenError,
            ReadPropTokenErrorKind, ReadTokenError, ReadTokenErrorKind,
        },
    },
    types::ByteStr,
};

// Like `Node<'blob>`, but smaller in size.
// Intended to reduce the stack size required to hold a `TreeCursor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobNodeHandle {
    name_start: u32,
    name_len: u32,
}

impl Default for BlobNodeHandle {
    fn default() -> Self {
        Self {
            name_start: u32::MAX,
            name_len: u32::MAX,
        }
    }
}

impl BlobNodeHandle {
    fn from_node(node: &Node<'_>, token_cursor: &BlobTokenCursor<'_>) -> Self {
        let range =
            polyfill::slice_subslice_range(token_cursor.struct_block, node.full_name()).unwrap();
        let name_start = u32::try_from(range.start).unwrap();
        let name_len = u32::try_from(range.len()).unwrap();
        Self {
            name_start,
            name_len,
        }
    }

    fn node<'blob>(&self, token_cursor: &BlobTokenCursor<'blob>) -> Node<'blob> {
        let name_start = usize::cast_from(self.name_start);
        let name_len = usize::cast_from(self.name_len);
        let full_name = ByteStr::new(&token_cursor.struct_block[name_start..][..name_len]);
        Node::new(full_name)
    }
}

#[derive(derive_more::Debug, Clone)]
pub struct BlobTokenCursor<'blob> {
    #[debug("{:?}", struct_block.slice_debug(16))]
    struct_block: &'blob [u8],
    #[debug("{:?}", strings_block.slice_debug(16))]
    strings_block: &'blob [u8],
    position: usize,
    root_emitted: bool,
    done: bool,
}

impl<'blob> BlobTokenCursor<'blob> {
    #[must_use]
    pub fn new(struct_block: &'blob [u8], strings_block: &'blob [u8]) -> Self {
        assert!(
            struct_block
                .as_ptr()
                .addr()
                .is_multiple_of(align_of::<TokenType>())
        );
        Self {
            struct_block,
            strings_block,
            position: 0,
            root_emitted: false,
            done: false,
        }
    }
}

impl<'blob> TokenCursor<'blob> for BlobTokenCursor<'blob> {
    type NodeHandle = BlobNodeHandle;

    fn make_node_handle(&self, node: &Node<'blob>) -> Self::NodeHandle {
        BlobNodeHandle::from_node(node, self)
    }

    fn get_node(&self, node_ref: &Self::NodeHandle) -> Node<'blob> {
        node_ref.node(self)
    }

    fn position(&self) -> usize {
        self.position
    }

    fn reset(&mut self) {
        self.done = false;
        self.position = 0;
        self.root_emitted = false;
    }

    fn seek_item_start_of_node(&mut self, node_ref: &Self::NodeHandle) {
        let struct_block = DataView::from(self.struct_block);
        let name_start = usize::cast_from(node_ref.name_start);
        let name_len = usize::cast_from(node_ref.name_len);
        let name_end = name_start + name_len + 1; // nul character

        assert!(name_start >= size_of::<TokenType>());
        assert!(name_start.is_multiple_of(align_of::<TokenType>()));
        assert_eq!(
            {
                let token_pos = name_start - size_of::<TokenType>();
                let token = struct_block.get::<TokenType>(token_pos);
                token.value()
            },
            TokenType::BEGIN_NODE
        );
        assert_eq!(*struct_block.get::<u8>(name_end), 0);

        let item_start = name_end.next_multiple_of(align_of::<TokenType>());
        self.done = false;
        self.root_emitted = true;
        self.position = item_start;
    }

    fn read_token(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError> {
        if self.done {
            return Ok(None);
        }
        let res = self.read_token_inner();
        if res.is_err() || res.as_ref().is_ok_and(Option::is_none) {
            self.done = true;
        }
        res
    }
}

impl<'blob> BlobTokenCursor<'blob> {
    fn read_token_inner(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError> {
        loop {
            let Some(raw_token) = self.read_raw_token() else {
                return Ok(None);
            };

            let position = self.position;
            let token = match raw_token.value() {
                TokenType::BEGIN_NODE => self
                    .read_begin_node()
                    .map_err(|source| ReadTokenErrorKind::BeginNode { source })?,
                TokenType::END_NODE => Token::EndNode,
                TokenType::PROP => self
                    .read_prop_token()
                    .map_err(|source| ReadTokenErrorKind::Prop { source })?,
                TokenType::NOP => continue,
                TokenType::END => return Ok(None),
                token => bail!(ReadTokenErrorKind::UnknownToken { token, position }),
            };
            return Ok(Some(token));
        }
    }

    fn read_begin_node(&mut self) -> Result<Token<'blob>, ReadBeginNodeTokenError> {
        let position = self.position;
        let name = self
            .read_null_terminated_string()
            .ok_or(ReadBeginNodeTokenErrorKind::UnterminatedNodeName { position })?;
        let is_root = !self.root_emitted;
        if is_root {
            ensure!(
                name.is_empty(),
                ReadBeginNodeTokenErrorKind::RootNodeWithNonEmptyName { position },
            );
        } else {
            ensure!(
                !name.is_empty(),
                ReadBeginNodeTokenErrorKind::NonRootNodeWithEmptyName { position },
            );
        }

        ensure!(
            !name.contains(&PATH_SEPARATOR),
            ReadBeginNodeTokenErrorKind::SlashInName { position },
        );

        self.skip_token_padding();
        let node = Node::new(name);
        self.root_emitted = true;
        Ok(Token::BeginNode(node))
    }

    fn read_prop_token(&mut self) -> Result<Token<'blob>, ReadPropTokenError> {
        let strings_block = self.strings_block;

        let position = self.position;
        let header = self
            .read_prop_header()
            .ok_or(ReadPropTokenErrorKind::MissingPropertyHeader { position })?;

        let name_offset = usize::cast_from(header.name_offset());
        let len = usize::cast_from(header.len());

        ensure!(
            name_offset < strings_block.len(),
            ReadPropTokenErrorKind::PropertyNameOffsetExceedingBlock {
                position,
                name_offset
            },
        );

        let value = self
            .read_bytes(len)
            .ok_or(ReadPropTokenErrorKind::PropertyValueExceedingBlock { position, len })?;

        self.skip_token_padding();
        let name_bytes = &strings_block[name_offset..];
        Ok(Token::Property(Property::new(name_bytes, value)))
    }

    fn skip_token_padding(&mut self) {
        self.position = self.position.next_multiple_of(align_of::<TokenType>());
    }

    fn read_pod<T>(&mut self) -> Option<T>
    where
        T: Pod + Copy,
    {
        let struct_block = DataView::from(self.struct_block);
        assert!(self.position.is_multiple_of(align_of::<T>()));
        let value = *struct_block.try_get::<T>(self.position)?;
        self.position += size_of::<T>();
        Some(value)
    }

    fn read_raw_token(&mut self) -> Option<TokenType> {
        self.read_pod()
    }

    fn read_prop_header(&mut self) -> Option<PropertyHeader> {
        self.read_pod()
    }

    fn read_null_terminated_string(&mut self) -> Option<&'blob ByteStr> {
        let bytes = self.struct_block.get(self.position..)?;
        let (bytes, _) = polyfill::slice_split_once(bytes, |&b| b == 0)?;
        self.position += bytes.len() + 1;
        Some(ByteStr::new(bytes))
    }

    fn read_bytes(&mut self, len: usize) -> Option<&'blob [u8]> {
        let bytes = &self.struct_block.get(self.position..)?.get(..len)?;
        self.position += len;
        Some(bytes)
    }
}

impl<'blob> Iterator for BlobTokenCursor<'blob> {
    type Item = Result<Token<'blob>, ReadTokenError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_token().transpose()
    }
}

impl FusedIterator for BlobTokenCursor<'_> {}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec::Vec;
    use core::ptr;

    use dataview::PodMethods as _;

    use super::*;
    use crate::testing::BlockBuilder;

    #[repr(align(4))]
    struct AlignedBytes<const N: usize>([u8; N]);

    #[test]
    fn test_blob_node_handle_default() {
        let handle = BlobNodeHandle::default();
        assert_eq!(handle.name_start, u32::MAX);
        assert_eq!(handle.name_len, u32::MAX);
    }

    #[test]
    fn test_blob_token_cursor_new_and_reset() {
        let struct_block = AlignedBytes([0_u8; 16]);
        let strings_block = &[0_u8; 8];
        let mut cursor = BlobTokenCursor::new(&struct_block.0, strings_block);
        assert_eq!(cursor.position(), 0);
        cursor.position = 10;
        cursor.done = true;
        cursor.root_emitted = true;
        cursor.reset();
        assert_eq!(cursor.position(), 0);
        assert!(!cursor.done);
        assert!(!cursor.root_emitted);
    }

    #[test]
    fn test_blob_token_cursor_read_token_end() {
        let (struct_block, strings_block) = BlockBuilder::new().end().build();
        let mut cursor = BlobTokenCursor::new(&struct_block, &strings_block);
        let token = cursor.read_token().unwrap();
        assert!(token.is_none());
        let token = cursor.read_token().unwrap();
        assert!(token.is_none());
    }

    #[test]
    fn test_blob_token_cursor_read_begin_node_root() {
        let (struct_block, strings_block) = BlockBuilder::new().begin_node(b"").end().build();
        let tokens = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(tokens, &[Token::BeginNode(Node::new(""))]);
    }

    #[test]
    fn test_blob_token_cursor_read_begin_node_non_root() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .begin_node(b"child")
            .end_node()
            .end_node()
            .end()
            .build();
        let tokens = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            tokens,
            &[
                Token::BeginNode(Node::new("")),
                Token::BeginNode(Node::new("child")),
                Token::EndNode,
                Token::EndNode
            ]
        );
    }

    #[test]
    fn test_blob_token_cursor_read_prop_token() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .prop(b"foo", b"bar")
            .end_node()
            .end()
            .build();
        let tokens = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            tokens,
            &[
                Token::BeginNode(Node::new("")),
                Token::Property(Property::new("foo", "bar")),
                Token::EndNode,
            ]
        );
    }

    #[test]
    fn test_blob_token_cursor_nop_token_skipped() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .nop()
            .prop(b"foo", b"bar")
            .nop()
            .end()
            .build();
        let tokens = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            tokens,
            &[
                Token::BeginNode(Node::new("")),
                Token::Property(Property::new("foo", "bar"))
            ]
        );
    }

    #[test]
    fn test_blob_node_handle_from_node_and_node_methods() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .begin_node(b"child")
            .end()
            .build();
        let mut cursor = BlobTokenCursor::new(&struct_block, &strings_block);
        let token = cursor.read_token().unwrap().unwrap();
        assert_eq!(token, Token::BeginNode(Node::new("")));
        let token = cursor.read_token().unwrap().unwrap();
        assert_eq!(token, Token::BeginNode(Node::new("child")));
        let node = token.into_begin_node().unwrap();
        let handle = BlobNodeHandle::from_node(&node, &cursor);
        let node2 = handle.node(&cursor);
        assert_eq!(node.full_name(), node2.full_name());
        assert!(ptr::eq(node.full_name(), node2.full_name()));
    }

    #[test]
    fn test_blob_token_cursor_seek_item_start_of_node() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .prop(b"foo", b"bar")
            .begin_node(b"child")
            .prop(b"baz", b"qux")
            .end()
            .build();
        let mut cursor = BlobTokenCursor::new(&struct_block, &strings_block);
        // Seek to end
        let tokens = cursor.by_ref().collect::<Result<Vec<_>, _>>().unwrap();
        assert_eq!(
            &tokens,
            &[
                Token::BeginNode(Node::new("")),
                Token::Property(Property::new("foo", "bar")),
                Token::BeginNode(Node::new("child")),
                Token::Property(Property::new("baz", "qux")),
            ]
        );
        let handle = BlobNodeHandle::from_node(tokens[0].as_begin_node().unwrap(), &cursor);
        cursor.seek_item_start_of_node(&handle);
        // After seeking, the next token should be the property
        let token = cursor.read_token().unwrap().unwrap();
        assert_eq!(token, Token::Property(Property::new("foo", "bar")));
    }

    #[test]
    fn test_blob_token_cursor_multiple_properties() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .prop(b"foo", b"bar")
            .prop(b"baz", b"qux")
            .end_node()
            .end()
            .build();
        let tokens = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            tokens,
            &[
                Token::BeginNode(Node::new("")),
                Token::Property(Property::new("foo", "bar")),
                Token::Property(Property::new("baz", "qux")),
                Token::EndNode,
            ]
        );
    }

    #[test]
    fn test_blob_token_cursor_empty_struct_and_strings_block() {
        let struct_block = AlignedBytes([]);
        let strings_block = &[];
        let mut cursor = BlobTokenCursor::new(&struct_block.0, strings_block);
        let token = cursor.read_token().unwrap();
        assert!(token.is_none());
    }

    #[test]
    fn test_blob_token_cursor_unknown_token_type() {
        let (struct_block, strings_block) = BlockBuilder::new().token(0x1234_5678).end().build();
        let mut cursor = BlobTokenCursor::new(&struct_block, &strings_block);
        let err = cursor.read_token().unwrap_err();
        assert!(
            matches!(
                err.kind(),
                ReadTokenErrorKind::UnknownToken { token, .. } if *token == 0x1234_5678,
            ),
            "err: {err:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_missing_property_header() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .token(TokenType::PROP) // Missing property header
            .build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::Prop { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadPropTokenErrorKind::MissingPropertyHeader { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_property_name_offset_exceeding_block() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .prop_raw(9999, b"val")
            .end()
            .build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::Prop { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadPropTokenErrorKind::PropertyNameOffsetExceedingBlock { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_property_value_exceeding_block() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .token(TokenType::PROP)
            .extend_struct_block_from_slice(PropertyHeader::new(100, 0).as_bytes()) // Value length exceeds
            .extend_strings_block_from_slice(b"foo\0")
            .end()
            .build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::Prop { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadPropTokenErrorKind::PropertyValueExceedingBlock { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_begin_node_unterminated_name() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .token(TokenType::BEGIN_NODE)
            .extend_struct_block_from_slice(b"unterminated")
            .build();
        assert!(struct_block.len().is_multiple_of(align_of::<TokenType>()));
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::BeginNode { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadBeginNodeTokenErrorKind::UnterminatedNodeName { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_begin_node_with_slash_in_name() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .begin_node(b"bad/name")
            .end()
            .build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::BeginNode { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadBeginNodeTokenErrorKind::SlashInName { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_non_root_node_with_empty_name() {
        let (struct_block, strings_block) = BlockBuilder::new()
            .begin_node(b"")
            .begin_node(b"")
            .end()
            .build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::BeginNode { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadBeginNodeTokenErrorKind::NonRootNodeWithEmptyName { .. },
            ),
            "source: {source:?}"
        );
    }

    #[test]
    fn test_blob_token_cursor_root_with_non_empty_name() {
        let (struct_block, strings_block) =
            BlockBuilder::new().begin_node(b"not_empty").end().build();
        let err = BlobTokenCursor::new(&struct_block, &strings_block)
            .collect::<Result<Vec<_>, _>>()
            .unwrap_err();
        let ReadTokenErrorKind::BeginNode { source } = err.kind() else {
            panic!("err: {err:?}");
        };
        assert!(
            matches!(
                source.kind(),
                ReadBeginNodeTokenErrorKind::RootNodeWithNonEmptyName { .. },
            ),
            "source: {source:?}"
        );
    }
}
