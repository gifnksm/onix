use core::ptr;

use dataview::Pod;
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use super::Devicetree;
use crate::{
    blob::{PATH_SEPARATOR, PropertyHeader, TokenType},
    types::ByteStr,
    utils,
};

#[derive(Debug)]
pub enum Token<'blob> {
    BeginNode {
        full_name: &'blob ByteStr,
    },
    EndNode,
    Property {
        name_offset: usize,
        value: &'blob [u8],
    },
}

#[derive(Debug, Clone)]
pub struct TokenCursor<'blob> {
    devicetree: &'blob Devicetree,
    position: usize,
}

impl PartialEq for TokenCursor<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.devicetree, other.devicetree) && self.position == other.position
    }
}

impl Eq for TokenCursor<'_> {}

impl<'blob> TokenCursor<'blob> {
    pub(crate) fn new(devicetree: &'blob Devicetree) -> Self {
        Self::from_parts(devicetree, 0)
    }

    pub(crate) fn from_parts(devicetree: &'blob Devicetree, position: usize) -> Self {
        assert!(position.is_multiple_of(align_of::<TokenType>()));
        assert!(position <= devicetree.struct_block().len());
        Self {
            devicetree,
            position,
        }
    }

    #[must_use]
    pub fn devicetree(&self) -> &'blob Devicetree {
        self.devicetree
    }

    #[must_use]
    pub fn position(&self) -> usize {
        self.position
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadTokenError {
    #[snafu(display("unknown token found in DTB structure block: token={token}, offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    Unknown {
        token: u32,
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to read BEGIN_NODE token"))]
    #[snafu(provide(ref, priority, Location => location))]
    BeginNode {
        #[snafu(source)]
        source: ReadBeginNodeTokenError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to read PROP token"))]
    #[snafu(provide(ref, priority, Location => location))]
    Prop {
        #[snafu(source)]
        source: ReadPropTokenError,
        #[snafu(implicit)]
        location: Location,
    },
}

impl<'blob> TokenCursor<'blob> {
    pub fn read_token(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_token_error::*;

        loop {
            let Some(raw_token) = self.read_raw_token() else {
                return Ok(None);
            };

            let token = match raw_token.0.read() {
                TokenType::BEGIN_NODE => self.read_begin_node().context(BeginNodeSnafu)?,
                TokenType::END_NODE => Token::EndNode,
                TokenType::PROP => self.read_prop_token().context(PropSnafu)?,
                TokenType::NOP => continue,
                TokenType::END => return Ok(None),
                token => {
                    return UnknownSnafu {
                        token,
                        offset: self.position,
                    }
                    .fail();
                }
            };
            return Ok(Some(token));
        }
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadBeginNodeTokenError {
    #[snafu(display("unterminated node name in DTB structure block: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    UnterminatedNodeName {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("node name contains '/' character: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    SlashInName {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

impl<'blob> TokenCursor<'blob> {
    fn read_begin_node(&mut self) -> Result<Token<'blob>, ReadBeginNodeTokenError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_begin_node_token_error::*;

        let name = self
            .read_null_terminated_string()
            .context(UnterminatedNodeNameSnafu {
                offset: self.position,
            })?;
        ensure!(
            !name.contains(&PATH_SEPARATOR),
            SlashInNameSnafu {
                offset: self.position,
            }
        );

        self.skip_token_padding();
        Ok(Token::BeginNode { full_name: name })
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadPropTokenError {
    #[snafu(display("missing property header in DTB structure block: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingPropertyHeader {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display(
        "property name offset exceeds DTB strings block: offset={offset}, \
         name_offset={name_offset}"
    ))]
    #[snafu(provide(ref, priority, Location => location))]
    PropertyNameOffsetExceedingBlock {
        offset: usize,
        name_offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("property value exceeds DTB structure block: offset={offset}, len={len}"))]
    #[snafu(provide(ref, priority, Location => location))]
    PropertyValueExceedingBlock {
        offset: usize,
        len: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

impl<'blob> TokenCursor<'blob> {
    fn read_prop_token(&mut self) -> Result<Token<'blob>, ReadPropTokenError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_prop_token_error::*;

        let header = self
            .read_prop_header()
            .context(MissingPropertyHeaderSnafu {
                offset: self.position,
            })?;

        let name_offset = usize::cast_from(header.name_offset.read());
        let len = usize::cast_from(header.len.read());

        if name_offset >= self.devicetree.strings_block().len() {
            return PropertyNameOffsetExceedingBlockSnafu {
                offset: self.position,
                name_offset,
            }
            .fail();
        }

        let value = self
            .read_bytes(len)
            .context(PropertyValueExceedingBlockSnafu {
                offset: self.position,
                len,
            })?;

        self.skip_token_padding();
        Ok(Token::Property { name_offset, value })
    }
}

impl<'blob> TokenCursor<'blob> {
    fn skip_token_padding(&mut self) {
        self.position = self.position.next_multiple_of(align_of::<TokenType>());
    }

    fn read_pod<T>(&mut self) -> Option<T>
    where
        T: Pod + Copy,
    {
        assert!(self.position.is_multiple_of(align_of::<T>()));
        let value = *self.devicetree.struct_block().try_get::<T>(self.position)?;
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
        let (bytes, _) = utils::slice_split_once(
            self.devicetree.struct_block()[self.position..].as_ref(),
            |&b| b == 0,
        )?;
        self.position += bytes.len() + 1;
        Some(ByteStr::new(bytes))
    }

    fn read_bytes(&mut self, len: usize) -> Option<&'blob [u8]> {
        let bytes = self
            .devicetree
            .struct_block()
            .try_slice(self.position, len)?;
        self.position += len;
        Some(bytes)
    }
}
