use snafu::{IntoError as _, Snafu};
use snafu_utils::Location;

use crate::token_cursor::error::ReadTokenError;

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum ReadTreeError {
    #[snafu(display("failed to read DTB token"))]
    #[snafu(provide(ref, priority, Location => location))]
    ReadToken {
        #[snafu(source)]
        source: ReadTokenError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no root node found in DTB structure block"))]
    #[snafu(provide(ref, priority, Location => location))]
    NoRootNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected Property token found in DTB structure block: offset={offset}"))]
    UnexpectedPropertyToken {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected EndNode token found in DTB structure block: offset={offset}"))]
    UnexpectedEndNodeToken {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("unexpected end of tokens found in DTB structure block: offset={offset}"))]
    UnexpectedEndOfTokens {
        offset: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("devicetree is too deep"))]
    TooDeep {
        #[snafu(implicit)]
        location: Location,
    },
}

impl ReadTreeError {
    #[track_caller]
    #[must_use]
    pub fn read_token(source: ReadTokenError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        ReadTokenSnafu.into_error(source)
    }

    #[track_caller]
    #[must_use]
    pub fn no_root_node() -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        NoRootNodeSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn unexpected_property_token(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        UnexpectedPropertyTokenSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn unexpected_end_node_token(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        UnexpectedEndNodeTokenSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn unexpected_end_of_tokens(offset: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        UnexpectedEndOfTokensSnafu { offset }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn too_deep() -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::read_tree_error::*;

        TooDeepSnafu.build()
    }
}
