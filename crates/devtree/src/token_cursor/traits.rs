use super::error::ReadTokenError;
use crate::blob::{Node, Property};

#[derive(Debug)]
pub enum Token<'blob> {
    BeginNode(Node<'blob>),
    EndNode,
    Property(Property<'blob>),
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
