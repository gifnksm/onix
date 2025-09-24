use snafu::{IntoError as _, Snafu};
use snafu_utils::Location;

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadTokenError {
    #[snafu(display(
        "unknown token found in DTB structure block: token={token:#x}, offset={offset}"
    ))]
    #[snafu(provide(ref, priority, Location => location))]
    UnknownToken {
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

impl ReadTokenError {
    #[track_caller]
    #[must_use]
    pub fn unknown_token(token: u32, offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_token_error::*;

        UnknownTokenSnafu { token, offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn begin_node(source: ReadBeginNodeTokenError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_token_error::*;

        BeginNodeSnafu.into_error(source)
    }

    #[track_caller]
    #[must_use]
    pub fn prop(source: ReadPropTokenError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_token_error::*;

        PropSnafu.into_error(source)
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
    #[snafu(display("root node has non-empty name: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    RootNodeWithNonEmptyName {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("non-root node has empty name: offset={offset}"))]
    #[snafu(provide(ref, priority, Location => location))]
    NonRootNodeWithEmptyName {
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

impl ReadBeginNodeTokenError {
    #[track_caller]
    #[must_use]
    pub fn unterminated_node_name(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_begin_node_token_error::*;

        UnterminatedNodeNameSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn slash_in_name(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_begin_node_token_error::*;

        SlashInNameSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn root_node_with_non_empty_name(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_begin_node_token_error::*;

        RootNodeWithNonEmptyNameSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn non_root_node_with_empty_name(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_begin_node_token_error::*;

        NonRootNodeWithEmptyNameSnafu { offset }.build()
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

impl ReadPropTokenError {
    #[track_caller]
    #[must_use]
    pub fn missing_property_header(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_prop_token_error::*;

        MissingPropertyHeaderSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn property_name_offset_exceeding_block(offset: usize, name_offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_prop_token_error::*;

        PropertyNameOffsetExceedingBlockSnafu {
            offset,
            name_offset,
        }
        .build()
    }

    #[track_caller]
    #[must_use]
    pub fn property_value_exceeding_block(offset: usize, len: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_prop_token_error::*;

        PropertyValueExceedingBlockSnafu { offset, len }.build()
    }
}
