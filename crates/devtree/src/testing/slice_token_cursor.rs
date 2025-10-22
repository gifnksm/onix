use crate::{
    blob::Node,
    token_cursor::{Token, TokenCursor, error::ReadTokenError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceNodeHandle {
    index: usize,
}

impl Default for SliceNodeHandle {
    fn default() -> Self {
        Self { index: usize::MAX }
    }
}

#[derive(Debug, Clone)]
pub struct SliceTokenCursor<'a, 'blob> {
    position: usize,
    tokens: &'a [Token<'blob>],
}

impl<'a, 'blob> SliceTokenCursor<'a, 'blob> {
    #[must_use]
    pub fn new(tokens: &'a [Token<'blob>]) -> Self {
        Self {
            position: 0,
            tokens,
        }
    }
}

impl<'blob> TokenCursor<'blob> for SliceTokenCursor<'_, 'blob> {
    type NodeHandle = SliceNodeHandle;

    fn make_node_handle(&self, node: &Node<'blob>) -> Self::NodeHandle {
        let index = self
            .tokens
            .iter()
            .position(|token| token.as_begin_node().is_some_and(|n| n == node))
            .expect("node should exist in tokens");
        SliceNodeHandle { index }
    }

    fn get_node(&self, node_ref: &Self::NodeHandle) -> Node<'blob> {
        self.tokens[node_ref.index]
            .as_begin_node()
            .expect("token should be a begin node")
            .clone()
    }

    fn position(&self) -> usize {
        self.position
    }

    fn reset(&mut self) {
        self.position = 0;
    }

    fn seek_item_start_of_node(&mut self, node_ref: &Self::NodeHandle) {
        assert!(node_ref.index < self.tokens.len(), "index out of bounds");
        self.position = node_ref.index + 1;
    }

    fn read_token(&mut self) -> Result<Option<Token<'blob>>, ReadTokenError> {
        if self.position >= self.tokens.len() {
            return Ok(None);
        }
        let token = self.tokens[self.position].clone();
        self.position += 1;
        Ok(Some(token))
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob::Property;

    #[test]
    fn make_node_handle_and_get_node_return_expected() {
        let node1 = Node::new("node1");
        let prop1 = Property::new("prop1", b"value1");
        let node2 = Node::new("node2");

        let tokens = &[
            Token::BeginNode(node1.clone()),
            Token::Property(prop1.clone()),
            Token::BeginNode(node2.clone()),
        ];

        let cursor = SliceTokenCursor::new(tokens);

        let handle = cursor.make_node_handle(&node2);
        // index is the position of the BeginNode token for node2
        assert_eq!(handle.index, 2);
        let fetched = cursor.get_node(&handle);
        assert_eq!(fetched, node2);
    }

    #[test]
    #[should_panic = "node should exist in tokens"]
    fn make_node_handle_panics_when_node_not_present() {
        let node1 = Node::new("only");
        let missing = Node::new("missing");

        let tokens = &[Token::BeginNode(node1.clone())];

        let cursor = SliceTokenCursor::new(tokens);
        // make_node_handle uses unwrap() internally; it should panic for missing node
        let _ = cursor.make_node_handle(&missing);
    }

    #[test]
    fn new_empty_read_returns_none() {
        let tokens = &[];
        let mut cursor = SliceTokenCursor::new(tokens);
        assert_eq!(cursor.position(), 0);
        assert_eq!(cursor.read_token().unwrap(), None);
    }

    #[test]
    fn read_token_returns_tokens_in_order() {
        let node1 = Node::new("node1");
        let prop1 = Property::new("prop1", b"value1");
        let node2 = Node::new("node2");

        let tokens = &[
            Token::BeginNode(node1.clone()),
            Token::Property(prop1.clone()),
            Token::BeginNode(node2.clone()),
        ];

        let mut cursor = SliceTokenCursor::new(tokens);

        assert_eq!(
            cursor.read_token().unwrap(),
            Some(Token::BeginNode(node1.clone()))
        );
        assert_eq!(
            cursor.read_token().unwrap(),
            Some(Token::Property(prop1.clone()))
        );
        assert_eq!(
            cursor.read_token().unwrap(),
            Some(Token::BeginNode(node2.clone()))
        );
        assert_eq!(cursor.read_token().unwrap(), None);
    }

    #[test]
    fn reset_sets_position_zero() {
        let tokens = &[];
        let mut cursor = SliceTokenCursor::new(tokens);
        // mutate the private field directly (tests are in the same file/module)
        cursor.position = 3;
        assert_eq!(cursor.position(), 3);
        cursor.reset();
        assert_eq!(cursor.position(), 0);
    }

    #[test]
    #[should_panic = "index out of bounds"]
    fn seek_item_start_of_node_asserts_on_out_of_bounds() {
        let tokens = &[];
        let mut cursor = SliceTokenCursor::new(tokens);
        let handle = SliceNodeHandle::default(); // index is usize::MAX, out of bounds
        cursor.seek_item_start_of_node(&handle);
    }

    #[test]
    fn seek_item_start_of_node_moves_position_and_next_read_is_after_begin() {
        let node1 = Node::new("n1");
        let prop1 = Property::new("p1", b"v1");
        let node2 = Node::new("n2");
        let prop2 = Property::new("p2", b"v2");

        let tokens = &[
            Token::BeginNode(node1.clone()), // 0
            Token::Property(prop1.clone()),  // 1
            Token::BeginNode(node2.clone()), // 2
            Token::Property(prop2.clone()),  // 3
        ];

        let mut cursor = SliceTokenCursor::new(tokens);
        let handle = cursor.make_node_handle(&node2);
        // seek to the item-start of node2: position should become index + 1 = 3
        cursor.seek_item_start_of_node(&handle);
        assert_eq!(cursor.position(), 3);
        // next read should return the property that follows BeginNode(node2)
        assert_eq!(
            cursor.read_token().unwrap(),
            Some(Token::Property(prop2.clone()))
        );
        // then EOF
        assert_eq!(cursor.read_token().unwrap(), None);
    }
}
