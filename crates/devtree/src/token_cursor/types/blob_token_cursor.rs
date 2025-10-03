use core::iter::FusedIterator;

use dataview::{DataView, Pod};
use platform_cast::CastFrom as _;

use crate::{
    blob::{Node, PATH_SEPARATOR, Property, PropertyHeader, TokenType},
    polyfill::{self, SliceDebug as _},
    token_cursor::{
        Token, TokenCursor,
        error::{ReadBeginNodeTokenError, ReadPropTokenError, ReadTokenError},
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
    pub(crate) fn from_node(node: &Node<'_>, token_cursor: &BlobTokenCursor<'_>) -> Self {
        let range =
            polyfill::slice_subslice_range(token_cursor.struct_block, node.full_name()).unwrap();
        let name_start = u32::try_from(range.start).unwrap();
        let name_len = u32::try_from(range.len()).unwrap();
        Self {
            name_start,
            name_len,
        }
    }

    pub(crate) fn node<'blob>(&self, token_cursor: &BlobTokenCursor<'blob>) -> Node<'blob> {
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
                TokenType::BEGIN_NODE => {
                    self.read_begin_node().map_err(ReadTokenError::begin_node)?
                }
                TokenType::END_NODE => Token::EndNode,
                TokenType::PROP => self.read_prop_token().map_err(ReadTokenError::prop)?,
                TokenType::NOP => continue,
                TokenType::END => return Ok(None),
                token => return Err(ReadTokenError::unknown_token(token, position)),
            };
            return Ok(Some(token));
        }
    }

    fn read_begin_node(&mut self) -> Result<Token<'blob>, ReadBeginNodeTokenError> {
        let position = self.position;
        let name = self
            .read_null_terminated_string()
            .ok_or_else(|| ReadBeginNodeTokenError::unterminated_node_name(position))?;
        let is_root = !self.root_emitted;
        if is_root && !name.is_empty() {
            return Err(ReadBeginNodeTokenError::root_node_with_non_empty_name(
                position,
            ));
        }
        if !is_root && name.is_empty() {
            return Err(ReadBeginNodeTokenError::non_root_node_with_empty_name(
                position,
            ));
        }

        if name.contains(&PATH_SEPARATOR) {
            return Err(ReadBeginNodeTokenError::slash_in_name(position));
        }

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
            .ok_or_else(|| ReadPropTokenError::missing_property_header(position))?;

        let name_offset = usize::cast_from(header.name_offset());
        let len = usize::cast_from(header.len());

        if name_offset >= strings_block.len() {
            return Err(ReadPropTokenError::property_name_offset_exceeding_block(
                position,
                name_offset,
            ));
        }

        let value = self
            .read_bytes(len)
            .ok_or_else(|| ReadPropTokenError::property_value_exceeding_block(position, len))?;

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
