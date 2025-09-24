use super::{Node, Property};

#[derive(Debug, Clone, derive_more::From)]
pub enum Item<'blob> {
    Property(Property<'blob>),
    Node(Node<'blob>),
}

impl<'blob> Item<'blob> {
    #[must_use]
    pub fn into_property(self) -> Option<Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn into_node(self) -> Option<Node<'blob>> {
        match self {
            Self::Property(_) => None,
            Self::Node(node) => Some(node),
        }
    }

    #[must_use]
    pub fn as_property(&self) -> Option<&Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn as_node(&self) -> Option<&Node<'blob>> {
        match self {
            Self::Property(_) => None,
            Self::Node(node) => Some(node),
        }
    }

    #[must_use]
    pub fn is_property(&self) -> bool {
        matches!(self, Self::Property(_))
    }

    #[must_use]
    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }
}
