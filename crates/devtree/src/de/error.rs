use core::str::Utf8Error;

use snafu::{IntoError as _, Snafu};
use snafu_utils::Location;

use crate::{
    blob::{Node, Property},
    tree_cursor::error::ReadTreeError,
    types::property::Phandle,
};

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum DeserializeError {
    #[snafu(display("failed to read devicetree"))]
    #[snafu(provide(ref, priority, Location => location))]
    ReadTree {
        #[snafu(source)]
        source: ReadTreeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to deserialize devicetree property"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeProperty {
        #[snafu(source)]
        source: DeserializePropertyError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to deserialize devicetree node"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeNode {
        #[snafu(source)]
        source: DeserializeNodeError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("deserializer does not support cloning"))]
    #[snafu(provide(ref, priority, Location => location))]
    CloneNotSupported {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no current node"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingCurrentNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no node with phandle={phandle}"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingPhandleNode {
        phandle: Phandle,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("message"))]
    #[snafu(provide(ref, priority, Location => location))]
    Custom {
        message: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
}

impl From<ReadTreeError> for DeserializeError {
    #[track_caller]
    fn from(source: ReadTreeError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        ReadTreeSnafu.into_error(source)
    }
}

impl From<DeserializePropertyError> for DeserializeError {
    #[track_caller]
    fn from(source: DeserializePropertyError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        DeserializePropertySnafu.into_error(source)
    }
}

impl From<DeserializeNodeError> for DeserializeError {
    #[track_caller]
    fn from(source: DeserializeNodeError) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        DeserializeNodeSnafu.into_error(source)
    }
}

impl DeserializeError {
    #[track_caller]
    #[must_use]
    pub fn clone_not_supported() -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        CloneNotSupportedSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_current_node() -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingCurrentNodeSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_phandle_node(phandle: Phandle) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        MissingPhandleNodeSnafu { phandle }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn custom(_node: &Node<'_>, message: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_error::*;

        CustomSnafu { message }.build()
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum DeserializePropertyError {
    #[snafu(display("expected value length is {expected}, got {actual}"))]
    #[snafu(provide(ref, priority, Location => location))]
    ValueLengthMismatch {
        expected: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("value length is not multiple of {expected_unit}, got {actual}"))]
    #[snafu(provide(ref, priority, Location => location))]
    ValueLengthIsNotMultipleOf {
        expected_unit: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing nul character in string value"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingNulInStringValue {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid string value"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidStringValue {
        #[snafu(source)]
        source: Utf8Error,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("message"))]
    #[snafu(provide(ref, priority, Location => location))]
    Custom {
        message: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
}

impl DeserializePropertyError {
    #[track_caller]
    #[must_use]
    pub fn value_length_mismatch(property: &Property<'_>, expected: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_property_error::*;

        let actual = property.value().len();
        ValueLengthMismatchSnafu { expected, actual }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn value_length_is_not_multiple_of(property: &Property<'_>, expected_unit: usize) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_property_error::*;

        let actual = property.value().len();
        ValueLengthIsNotMultipleOfSnafu {
            expected_unit,
            actual,
        }
        .build()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_nul_in_string_value(_property: &Property<'_>) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_property_error::*;

        MissingNulInStringValueSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn invalid_string_value(_property: &Property<'_>, source: Utf8Error) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_property_error::*;

        InvalidStringValueSnafu.into_error(source)
    }

    #[track_caller]
    #[must_use]
    pub fn custom(_property: &Property<'_>, message: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_property_error::*;

        CustomSnafu { message }.build()
    }
}

#[derive(Debug, Snafu)]
#[snafu(module)]
#[non_exhaustive]
pub enum DeserializeNodeError {
    #[snafu(display("missing property: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("duplicated property: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    DuplicatedProperty {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("missing child: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingChild {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("duplicated child: `{name}`"))]
    #[snafu(provide(ref, priority, Location => location))]
    DuplicatedChild {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("no parent node"))]
    #[snafu(provide(ref, priority, Location => location))]
    MissingParentNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("message"))]
    #[snafu(provide(ref, priority, Location => location))]
    Custom {
        message: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
}

impl DeserializeNodeError {
    #[track_caller]
    #[must_use]
    pub fn missing_property(_node: &Node<'_>, name: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        MissingPropertySnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn duplicated_property(_node: &Node<'_>, name: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        DuplicatedPropertySnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_child(_node: &Node<'_>, name: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        MissingChildSnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn duplicated_child(_node: &Node<'_>, name: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        DuplicatedChildSnafu { name }.build()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_parent_node(_node: &Node<'_>) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        MissingParentNodeSnafu.build()
    }

    #[track_caller]
    #[must_use]
    pub fn custom(_node: &Node<'_>, message: &'static str) -> Self {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_node_error::*;

        CustomSnafu { message }.build()
    }
}
