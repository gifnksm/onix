use core::str::Utf8Error;

use snafu::{IntoError as _, Snafu};
use snafu_utils::Location;

use super::PropertyContext;
use crate::{cursor::ReadNodeError, de::NodeContext, types::property::Phandle};

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum DeserializeError {
    #[snafu(display("failed to read a node from DTB"))]
    #[snafu(provide(ref, priority, Location => location))]
    ReadNode {
        #[snafu(source)]
        source: ReadNodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing parent node"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingParentNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing child node: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingChildNode {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing node with phandle: {phandle}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingPhandleNode {
        phandle: Phandle,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing property: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing properties: `{names:?}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingProperties {
        names: &'static [&'static str],
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid property value length: expected {expected}, got {actual}"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidPropertyValueLength {
        expected: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing nul character in string"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingNulInString {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid property value: invalid string"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringProperty {
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("{message}"))]
    #[snafu(provide(ref, priority, Location => location))]
    Custom {
        message: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
}

impl NodeContext<'_, '_> {
    #[track_caller]
    #[must_use]
    pub fn error_read_node(&self, source: ReadNodeError) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        ReadNodeSnafu.into_error(source)
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_parent_node(&self) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingParentNodeSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_child_node(&self, name: &'static str) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingChildNodeSnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_phandle_node(&self, phandle: Phandle) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingPhandleNodeSnafu { phandle }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_property(&self, name: &'static str) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingPropertySnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_properties(&self, names: &'static [&'static str]) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingPropertiesSnafu { names }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_custom(&self, message: &'static str) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        CustomSnafu { message }.build()
    }
}

impl PropertyContext<'_, '_> {
    #[track_caller]
    #[must_use]
    pub fn error_invalid_value_length(&self, expected: usize) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        InvalidPropertyValueLengthSnafu {
            expected,
            actual: self.property().value().len(),
        }
        .build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_missing_nul_in_string(&self) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingNulInStringSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn error_invalid_string_value(&self, source: Utf8Error) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        InvalidStringPropertySnafu.into_error(source)
    }

    #[track_caller]
    #[must_use]
    pub fn error_custom(&self, message: &'static str) -> DeserializeError {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        CustomSnafu { message }.build()
    }
}

pub(crate) fn error_read_node(source: ReadNodeError) -> DeserializeError {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::deserialize_error::*;

    ReadNodeSnafu.into_error(source)
}
