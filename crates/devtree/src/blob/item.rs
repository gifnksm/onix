use super::{Node, Property};

#[derive(Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::IsVariant)]
pub enum Item<'blob> {
    Property(Property<'blob>),
    Node(Node<'blob>),
}

impl<'blob> Item<'blob> {
    #[must_use]
    pub const fn into_property(self) -> Option<Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub const fn into_node(self) -> Option<Node<'blob>> {
        match self {
            Self::Property(_) => None,
            Self::Node(node) => Some(node),
        }
    }

    #[must_use]
    pub const fn as_property(&self) -> Option<&Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub const fn as_node(&self) -> Option<&Node<'blob>> {
        match self {
            Self::Property(_) => None,
            Self::Node(node) => Some(node),
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_property() {
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop.clone());
        assert_eq!(item.into_property(), Some(prop));
        let node = Node::new("node");
        let item = Item::Node(node);
        assert_eq!(item.into_property(), None);
    }

    #[test]
    fn test_into_node() {
        let node = Node::new("node");
        let item = Item::Node(node.clone());
        assert_eq!(item.into_node(), Some(node));
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop);
        assert_eq!(item.into_node(), None);
    }

    #[test]
    fn test_as_property() {
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop.clone());
        assert_eq!(item.as_property(), Some(&prop));
        let node = Node::new("node");
        let item = Item::Node(node);
        assert_eq!(item.as_property(), None);
    }

    #[test]
    fn test_as_node() {
        let node = Node::new("node");
        let item = Item::Node(node.clone());
        assert_eq!(item.as_node(), Some(&node));
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop);
        assert_eq!(item.as_node(), None);
    }

    #[test]
    fn test_is_property() {
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop);
        assert!(item.is_property());
        let node = Node::new("node");
        let item = Item::Node(node);
        assert!(!item.is_property());
    }

    #[test]
    fn test_is_node() {
        let node = Node::new("node");
        let item = Item::Node(node);
        assert!(item.is_node());
        let prop = Property::new(b"prop", b"value");
        let item = Item::Property(prop);
        assert!(!item.is_node());
    }
}
