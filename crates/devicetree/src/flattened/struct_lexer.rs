//! Devicetree structure block tokenizer and lexer.
//!
//! This module provides functionality to tokenize and parse the structure block
//! of a Flattened Devicetree (FDT). The structure block contains the actual
//! devicetree hierarchy with nodes and properties encoded as binary tokens.
//!
//! # Structure Block Format
//!
//! The structure block consists of a sequence of tokens that represent:
//!
//! - Node beginnings and endings
//! - Properties with names and values
//! - Special tokens like NOP and END
//!
//! Each token is 4-byte aligned and follows the format specified in the
//! Devicetree specification.

use core::{iter::FusedIterator, str::Utf8Error};

use dataview::Pod;
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::{
    Devicetree,
    layout::{PropHeader, StructToken},
};
use crate::common::property::Property;

/// Errors that can occur while tokenizing a Flattened Devicetree (FDT).
///
/// This enum represents all possible error conditions encountered during
/// tokenizing or interpreting a FDT blob, such as invalid alignment, magic
/// number, version incompatibility, malformed layout, or invalid strings.
#[derive(Debug, Clone, Snafu)]
#[snafu(module)]
pub enum StructLexerError {
    #[snafu(display("invalid token: {token:#x} at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidToken {
        token: u32,
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid string in structure block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringInStructBlock {
        offset: usize,
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid string in strings block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringInStringsBlock {
        offset: usize,
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing prop header at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingPropHeader {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected end of struct block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnexpectedEndOfStructBlock {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected end of strings block at offset {offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnexpectedEndOfStringsBlock {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

/// Represents an element in the structure block of the Devicetree.
///
/// Each element corresponds to a node start, node end, property, or special
/// token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructTokenWithData<'fdt> {
    /// Start of a node (includes name and optional address part)
    BeginNode {
        name: &'fdt str,
        address: Option<&'fdt str>,
    },
    /// End of a node
    EndNode,
    /// Property (name and value)
    Prop(Property<'fdt>),
    /// NOP token
    Nop,
    /// End of the structure block
    End,
}

/// Iterator over the structure elements of the devicetree.
///
/// This iterator yields [`StructTokenWithData`] and returns an error if the
/// structure is invalid.
#[derive(Debug, Clone)]
pub struct StructLexer<'fdt, 'tree> {
    devicetree: &'tree Devicetree<'fdt>,
    offset: usize,
}

impl<'fdt, 'tree> StructLexer<'fdt, 'tree> {
    #[must_use]
    pub(crate) fn new(devicetree: &'tree Devicetree<'fdt>) -> Self {
        Self {
            devicetree,
            offset: 0,
        }
    }

    /// Splits a single token from the current position and returns both the
    /// token and a new lexer positioned after it.
    ///
    /// This method is useful when you need to peek at the next token without
    /// consuming it from the main iterator, or when you need to create multiple
    /// lexer states for different parsing paths.
    ///
    /// # Returns
    ///
    /// * `Ok(Some((token, lexer)))` - The next token and updated lexer position
    /// * `Ok(None)` - If there are no more tokens
    /// * `Err(error)` - If the token is invalid or malformed
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let mut lexer = devicetree.struct_lexer();
    /// if let Ok(Some((token, next_lexer))) = lexer.split_token() {
    ///     match token {
    ///         StructTokenWithData::BeginNode { name, address } => {
    ///             println!("Found node: {}", name);
    ///         }
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn split_token(
        &self,
    ) -> Result<Option<(StructTokenWithData<'fdt>, Self)>, StructLexerError> {
        let mut this = self.clone();
        let Some(token) = this.next().transpose()? else {
            return Ok(None);
        };
        Ok(Some((token, this)))
    }
}

impl<'fdt> Iterator for StructLexer<'fdt, '_> {
    type Item = Result<StructTokenWithData<'fdt>, StructLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_token_and_data().transpose()
    }
}

impl FusedIterator for StructLexer<'_, '_> {}

impl<'fdt> StructLexer<'fdt, '_> {
    fn read_token_and_data(
        &mut self,
    ) -> Result<Option<StructTokenWithData<'fdt>>, StructLexerError> {
        #[expect(clippy::wildcard_imports)]
        use self::struct_lexer_error::*;

        assert_eq!(self.offset % align_of::<StructToken>(), 0);
        let Some(token) = self.read_token() else {
            return Ok(None);
        };

        let entry = match token.0.read() {
            StructToken::BEGIN_NODE => {
                let unit_name = self.read_null_terminated_string()?;
                self.skip_token_padding();
                let (name, address) = unit_name
                    .split_once('@')
                    .map_or((unit_name, None), |(name, address)| (name, Some(address)));
                StructTokenWithData::BeginNode { name, address }
            }
            StructToken::END_NODE => StructTokenWithData::EndNode,
            StructToken::PROP => {
                let prop_header = self.read_prop_header()?;
                let name = self.read_name(usize::cast_from(prop_header.nameoff.read()))?;
                let value = self.read_bytes(usize::cast_from(prop_header.len.read()))?;
                self.skip_token_padding();
                StructTokenWithData::Prop(Property::new(name, value))
            }
            StructToken::NOP => StructTokenWithData::Nop,
            StructToken::END => StructTokenWithData::End,
            token => {
                return InvalidTokenSnafu {
                    token,
                    offset: self.offset,
                }
                .fail();
            }
        };

        Ok(Some(entry))
    }

    fn read_ty<T>(&mut self) -> Option<T>
    where
        T: Pod + Copy,
    {
        let token = self.devicetree.struct_block.try_get::<T>(self.offset)?;
        self.offset += size_of::<T>();
        Some(*token)
    }

    fn read_token(&mut self) -> Option<StructToken> {
        self.read_ty()
    }

    fn read_prop_header(&mut self) -> Result<PropHeader, StructLexerError> {
        #[expect(clippy::wildcard_imports)]
        use self::struct_lexer_error::*;

        let offset = self.offset;
        self.read_ty().context(MissingPropHeaderSnafu { offset })
    }

    fn read_null_terminated_string(&mut self) -> Result<&'fdt str, StructLexerError> {
        #[expect(clippy::wildcard_imports)]
        use self::struct_lexer_error::*;

        let offset = self.offset;
        let len = self.devicetree.struct_block[offset..]
            .as_ref()
            .iter()
            .position(|&b| b == 0)
            .context(UnexpectedEndOfStructBlockSnafu { offset })?;
        self.offset += len + 1;
        let bytes = self.devicetree.struct_block.slice(offset, len);
        let s = str::from_utf8(bytes).context(InvalidStringInStructBlockSnafu { offset })?;
        Ok(s)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'fdt [u8], StructLexerError> {
        #[expect(clippy::wildcard_imports)]
        use self::struct_lexer_error::*;

        let offset = self.offset;
        self.offset += len;
        self.devicetree
            .struct_block
            .try_slice(offset, len)
            .context(UnexpectedEndOfStructBlockSnafu { offset })
    }

    fn read_name(&self, nameoff: usize) -> Result<&'fdt str, StructLexerError> {
        #[expect(clippy::wildcard_imports)]
        use self::struct_lexer_error::*;

        let len = self.devicetree.string_block[nameoff..]
            .as_ref()
            .iter()
            .position(|&b| b == 0)
            .context(UnexpectedEndOfStringsBlockSnafu { offset: nameoff })?;
        let bytes = self.devicetree.string_block.slice(nameoff, len);
        let s =
            str::from_utf8(bytes).context(InvalidStringInStringsBlockSnafu { offset: nameoff })?;
        Ok(s)
    }

    fn skip_token_padding(&mut self) {
        self.offset = self.offset.next_multiple_of(align_of::<StructToken>());
    }
}
