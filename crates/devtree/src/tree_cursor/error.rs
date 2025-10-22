use crate::token_cursor::error::ReadTokenError;

#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ReadTreeErrorKind {
    #[display("failed to read DTB token")]
    ReadToken {
        #[error(source)]
        source: ReadTokenError,
    },
    #[display("unexpected Property token found in DTB structure block: position={position}")]
    UnexpectedPropertyToken { position: usize },
    #[display("unexpected EndNode token found in DTB structure block: position={position}")]
    UnexpectedEndNodeToken { position: usize },
    #[display("unexpected end of tokens found in DTB structure block: position={position}")]
    UnexpectedEndOfTokens { position: usize },
    #[display("devicetree is too deep: position={position}")]
    TooDeep { position: usize },
}

define_error!(
    /// An error that can occur when reading a [`TreeCursor`].
    pub struct ReadTreeError {
        kind: ReadTreeErrorKind,
    }
);
