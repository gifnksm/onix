#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ReadTokenErrorKind {
    #[display("unknown token: token={token:#x}, offset={position}")]
    UnknownToken { token: u32, position: usize },
    #[display("invalid BEGIN_NODE token")]
    BeginNode {
        #[error(source)]
        source: ReadBeginNodeTokenError,
    },
    #[display("invalid PROP token")]
    Prop {
        #[error(source)]
        source: ReadPropTokenError,
    },
}

define_error!(
    pub struct ReadTokenError {
        kind: ReadTokenErrorKind,
    }
);

#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ReadBeginNodeTokenErrorKind {
    #[display("unterminated node name: position={position}")]
    UnterminatedNodeName { position: usize },
    #[display("root node has non-empty name: position={position}")]
    RootNodeWithNonEmptyName { position: usize },
    #[display("non-root node has empty name: position={position}")]
    NonRootNodeWithEmptyName { position: usize },
    #[display("node name contains '/' character: position={position}")]
    SlashInName { position: usize },
}

define_error!(
    pub struct ReadBeginNodeTokenError {
        kind: ReadBeginNodeTokenErrorKind,
    }
);

#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::IsVariant)]
#[non_exhaustive]
pub enum ReadPropTokenErrorKind {
    #[display("missing property header: position={position}")]
    MissingPropertyHeader { position: usize },
    #[display(
        "property name offset exceeds DTB strings block: position={position}, \
         name_offset={name_offset}"
    )]
    PropertyNameOffsetExceedingBlock { position: usize, name_offset: usize },
    #[display("property value exceeds DTB structure block: position={position}, len={len}")]
    PropertyValueExceedingBlock { position: usize, len: usize },
}

define_error!(
    pub struct ReadPropTokenError {
        kind: ReadPropTokenErrorKind,
    }
);
