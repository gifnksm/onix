use super::error::ReadTokenError;
use crate::blob::{Node, Property};

#[derive(Debug)]
pub enum Token<'blob> {
    BeginNode(Node<'blob>),
    EndNode,
    Property(Property<'blob>),
}

pub trait TokenCursor<'blob>: Clone {
    type NodeRef: Default + Clone;

    fn make_node_ref(&self, node: &Node<'blob>) -> Self::NodeRef;
    fn get_node(&self, node_ref: &Self::NodeRef) -> Node<'blob>;

    #[must_use]
    fn position(&self) -> usize;
    fn reset(&mut self);
    fn seek_item_start_of_node(&mut self, node_ref: &Self::NodeRef);
    fn read_token(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError>;
}
