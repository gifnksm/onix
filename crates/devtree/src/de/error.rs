use core::str::Utf8Error;

use crate::{
    blob::{Node, Property},
    tree_cursor::error::ReadTreeError,
    types::property::Phandle,
};

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum DeserializeErrorKind {
    #[display("failed to read devicetree")]
    ReadTree {
        #[error(source)]
        source: ReadTreeError,
    },
    #[display("failed to deserialize devicetree property")]
    DeserializeProperty {
        #[error(source)]
        source: DeserializePropertyError,
    },
    #[display("failed to deserialize devicetree node")]
    DeserializeNode {
        #[error(source)]
        source: DeserializeNodeError,
    },
    #[display("deserializer does not support cloning")]
    CloneNotSupported,
    #[display("no node with phandle={phandle}")]
    MissingPhandleNode { phandle: Phandle },
    #[display("{message}")]
    Custom { message: &'static str },
}

define_error!(
    pub struct DeserializeError {
        kind: DeserializeErrorKind,
    }
);

impl From<ReadTreeError> for DeserializeError {
    #[track_caller]
    fn from(source: ReadTreeError) -> Self {
        DeserializeErrorKind::ReadTree { source }.into()
    }
}

impl From<DeserializePropertyError> for DeserializeError {
    #[track_caller]
    fn from(source: DeserializePropertyError) -> Self {
        DeserializeErrorKind::DeserializeProperty { source }.into()
    }
}

impl From<DeserializeNodeError> for DeserializeError {
    #[track_caller]
    fn from(source: DeserializeNodeError) -> Self {
        DeserializeErrorKind::DeserializeNode { source }.into()
    }
}

impl DeserializeError {
    #[track_caller]
    #[must_use]
    pub fn clone_not_supported() -> Self {
        DeserializeErrorKind::CloneNotSupported.into()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_phandle_node(phandle: Phandle) -> Self {
        DeserializeErrorKind::MissingPhandleNode { phandle }.into()
    }

    #[track_caller]
    #[must_use]
    pub fn custom(message: &'static str) -> Self {
        DeserializeErrorKind::Custom { message }.into()
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum DeserializePropertyErrorKind {
    #[display("expected value length is {expected}, got {actual}")]
    ValueLengthMismatch { expected: usize, actual: usize },
    #[display("value length is not multiple of {expected_unit}, got {actual}")]
    ValueLengthIsNotMultipleOf { expected_unit: usize, actual: usize },
    #[display("missing nul character in string value")]
    MissingNulInStringValue,
    #[display("invalid string value")]
    InvalidStringValue {
        #[error(source)]
        source: Utf8Error,
    },
    #[display("{message}")]
    Custom { message: &'static str },
}

define_error!(
    pub struct DeserializePropertyError {
        kind: DeserializePropertyErrorKind,
    }
);

impl DeserializePropertyError {
    #[track_caller]
    #[must_use]
    pub fn value_length_mismatch(property: &Property<'_>, expected: usize) -> Self {
        let actual = property.value().len();
        DeserializePropertyErrorKind::ValueLengthMismatch { expected, actual }.into()
    }

    #[track_caller]
    #[must_use]
    pub fn value_length_is_not_multiple_of(property: &Property<'_>, expected_unit: usize) -> Self {
        let actual = property.value().len();
        DeserializePropertyErrorKind::ValueLengthIsNotMultipleOf {
            expected_unit,
            actual,
        }
        .into()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_nul_in_string_value(_property: &Property<'_>) -> Self {
        DeserializePropertyErrorKind::MissingNulInStringValue.into()
    }

    #[track_caller]
    #[must_use]
    pub fn invalid_string_value(_property: &Property<'_>, source: Utf8Error) -> Self {
        DeserializePropertyErrorKind::InvalidStringValue { source }.into()
    }

    #[track_caller]
    #[must_use]
    pub fn custom(_property: &Property<'_>, message: &'static str) -> Self {
        DeserializePropertyErrorKind::Custom { message }.into()
    }
}

#[derive(Debug, derive_more::Display, derive_more::Error)]
#[non_exhaustive]
pub enum DeserializeNodeErrorKind {
    #[display("missing property: `{property_name}`")]
    MissingProperty { property_name: &'static str },
    #[display("duplicated property: `{property_name}`")]
    DuplicatedProperty { property_name: &'static str },
    #[display("missing child: `{child_name}`")]
    MissingChild { child_name: &'static str },
    #[display("duplicated child: `{child_name}`")]
    DuplicatedChild { child_name: &'static str },
    #[display("no parent node")]
    MissingParentNode,
    #[display("{message}")]
    Custom { message: &'static str },
}

define_error!(
    pub struct DeserializeNodeError {
        kind: DeserializeNodeErrorKind,
    }
);

impl DeserializeNodeError {
    #[track_caller]
    #[must_use]
    pub fn missing_property(_node: &Node<'_>, name: &'static str) -> Self {
        DeserializeNodeErrorKind::MissingProperty {
            property_name: name,
        }
        .into()
    }

    #[track_caller]
    #[must_use]
    pub fn duplicated_property(_node: &Node<'_>, name: &'static str) -> Self {
        DeserializeNodeErrorKind::DuplicatedProperty {
            property_name: name,
        }
        .into()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_child(_node: &Node<'_>, name: &'static str) -> Self {
        DeserializeNodeErrorKind::MissingChild { child_name: name }.into()
    }

    #[track_caller]
    #[must_use]
    pub fn duplicated_child(_node: &Node<'_>, name: &'static str) -> Self {
        DeserializeNodeErrorKind::DuplicatedChild { child_name: name }.into()
    }

    #[track_caller]
    #[must_use]
    pub fn missing_parent_node(_node: &Node<'_>) -> Self {
        DeserializeNodeErrorKind::MissingParentNode.into()
    }

    #[track_caller]
    #[must_use]
    pub fn custom(_node: &Node<'_>, message: &'static str) -> Self {
        DeserializeNodeErrorKind::Custom { message }.into()
    }
}
