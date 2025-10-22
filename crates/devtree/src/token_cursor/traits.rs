use super::error::ReadTokenError;
use crate::blob::{Node, Property};

#[derive(Debug, Clone, PartialEq, Eq, derive_more::IsVariant)]
pub enum Token<'blob> {
    BeginNode(Node<'blob>),
    EndNode,
    Property(Property<'blob>),
}

impl<'blob> Token<'blob> {
    #[must_use]
    pub fn as_begin_node(&self) -> Option<&Node<'blob>> {
        match self {
            Self::BeginNode(node) => Some(node),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_property(&self) -> Option<&Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            _ => None,
        }
    }

    #[must_use]
    pub fn into_begin_node(self) -> Option<Node<'blob>> {
        match self {
            Self::BeginNode(node) => Some(node),
            _ => None,
        }
    }

    #[must_use]
    pub fn into_property(self) -> Option<Property<'blob>> {
        match self {
            Self::Property(prop) => Some(prop),
            _ => None,
        }
    }
}

pub trait TokenCursor<'blob>: Clone {
    type NodeHandle: Default + Clone;

    fn make_node_handle(&self, node: &Node<'blob>) -> Self::NodeHandle;
    fn get_node(&self, node_ref: &Self::NodeHandle) -> Node<'blob>;

    #[must_use]
    fn position(&self) -> usize;
    fn reset(&mut self);
    fn seek_item_start_of_node(&mut self, node_ref: &Self::NodeHandle);
    fn read_token(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError>;
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_as_begin_node_and_property() {
        let node = Node::new("node");
        let prop = Property::new("prop", b"value");

        let begin_node_token = Token::BeginNode(node.clone());
        let property_token = Token::Property(prop.clone());
        let end_node_token = Token::EndNode;

        assert_eq!(begin_node_token.as_begin_node(), Some(&node));
        assert_eq!(property_token.as_begin_node(), None);
        assert_eq!(end_node_token.as_begin_node(), None);

        assert_eq!(property_token.as_property(), Some(&prop));
        assert_eq!(begin_node_token.as_property(), None);
        assert_eq!(end_node_token.as_property(), None);
    }

    #[test]
    fn token_into_begin_node_and_property() {
        let node = Node::new("node");
        let prop = Property::new("prop", b"value");

        let begin_node_token = Token::BeginNode(node.clone());
        let property_token = Token::Property(prop.clone());
        let end_node_token = Token::EndNode;

        assert_eq!(begin_node_token.clone().into_begin_node(), Some(node));
        assert_eq!(property_token.clone().into_begin_node(), None);
        assert_eq!(end_node_token.clone().into_begin_node(), None);

        assert_eq!(property_token.into_property(), Some(prop));
        assert_eq!(begin_node_token.into_property(), None);
        assert_eq!(end_node_token.into_property(), None);
    }
}
