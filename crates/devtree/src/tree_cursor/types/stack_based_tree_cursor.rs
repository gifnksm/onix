use core::{iter, marker::PhantomData, slice};

use crate::{
    blob::{Item, Node},
    node_stack::{NodeStack, types::ArrayNodeStack},
    token_cursor::{Token, TokenCursor},
    tree_cursor::{
        TreeCursor,
        error::{ReadTreeError, ReadTreeErrorKind},
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadState {
    Init,
    Property,
    Child,
    Done,
}

#[derive(Debug, Clone)]
pub struct StackBasedTreeCursor<
    'blob,
    TC,
    S = ArrayNodeStack<<TC as TokenCursor<'blob>>::NodeHandle, 8>,
> where
    TC: TokenCursor<'blob>,
{
    node_stack: S,
    state: ReadState,
    token_cursor: TC,
    _phantom: PhantomData<&'blob ()>,
}

impl<'blob, TC> StackBasedTreeCursor<'blob, TC>
where
    TC: TokenCursor<'blob>,
{
    pub fn new(token_cursor: TC) -> Result<Self, ReadTreeError> {
        Self::with_node_stack(token_cursor, ArrayNodeStack::default())
    }
}

impl<'blob, TC, const STACK_SIZE: usize>
    StackBasedTreeCursor<
        'blob,
        TC,
        ArrayNodeStack<<TC as TokenCursor<'blob>>::NodeHandle, STACK_SIZE>,
    >
where
    TC: TokenCursor<'blob>,
{
    pub fn with_stack_size(token_cursor: TC) -> Result<Self, ReadTreeError> {
        StackBasedTreeCursor::with_node_stack(token_cursor, ArrayNodeStack::default())
    }
}

impl<'blob, TC, S> StackBasedTreeCursor<'blob, TC, S>
where
    TC: TokenCursor<'blob>,
    S: NodeStack<TC::NodeHandle>,
{
    pub fn with_node_stack(token_cursor: TC, node_stack: S) -> Result<Self, ReadTreeError> {
        let mut this = Self {
            node_stack,
            state: ReadState::Init,
            token_cursor,
            _phantom: PhantomData,
        };
        this.read_item_descend()?;
        Ok(this)
    }

    pub fn clone_with_node_stack<U>(
        &self,
        mut node_stack: U,
    ) -> Result<StackBasedTreeCursor<'blob, TC, U>, U>
    where
        U: NodeStack<TC::NodeHandle>,
    {
        let res = node_stack.clone_from_stack(&self.node_stack);
        ensure!(res.is_ok(), node_stack);
        Ok(StackBasedTreeCursor {
            node_stack,
            state: self.state,
            token_cursor: self.token_cursor.clone(),
            _phantom: PhantomData,
        })
    }
}

impl<'blob, TC, S> TreeCursor<'blob> for StackBasedTreeCursor<'blob, TC, S>
where
    TC: TokenCursor<'blob>,
    S: NodeStack<TC::NodeHandle>,
{
    type TokenCursor = TC;

    fn try_clone(&self) -> Option<Self> {
        let node_stack = self.node_stack.try_clone()?;
        Some(Self {
            node_stack,
            state: self.state,
            token_cursor: self.token_cursor.clone(),
            _phantom: PhantomData,
        })
    }

    fn token_cursor(&self) -> &TC {
        &self.token_cursor
    }

    fn depth(&self) -> usize {
        self.node_stack.len().checked_sub(1).unwrap()
    }

    fn node(&self) -> Node<'blob> {
        let node_ref = self.node_stack.current().unwrap();
        self.token_cursor.get_node(node_ref)
    }

    type Parents<'tc>
        = StackBasedParents<'tc, 'blob, TC>
    where
        Self: 'tc;

    fn parents(&self) -> Self::Parents<'_> {
        Self::Parents::new(self)
    }

    fn reset(&mut self) {
        while self.node_stack.len() > 1 {
            self.node_stack.pop().unwrap();
        }
        self.seek_node_start();
    }

    fn seek_node_start(&mut self) {
        let node_ref = self.node_stack.current().unwrap();
        self.state = ReadState::Property;
        self.token_cursor.seek_item_start_of_node(node_ref);
    }

    fn seek_node_end(&mut self) -> Result<(), ReadTreeError> {
        while let Some(item) = self.read_item_descend()? {
            match item {
                Item::Property(_) => {}
                Item::Node(_) => {
                    self.seek_parent_next()?.unwrap();
                }
            }
        }
        Ok(())
    }

    fn seek_root_start(&mut self) {
        while self.node_stack.len() > 1 {
            self.node_stack.pop().unwrap();
        }
        self.seek_node_start();
    }

    fn seek_parent_start(&mut self) -> Option<()> {
        if self.node_stack.len() <= 1 {
            return None;
        }
        self.node_stack.pop().unwrap();
        self.seek_node_start();
        Some(())
    }

    fn seek_parent_next(&mut self) -> Result<Option<()>, ReadTreeError> {
        if self.node_stack.len() <= 1 {
            return Ok(None);
        }
        self.seek_node_end()?;
        self.node_stack.pop().unwrap();
        self.state = ReadState::Child;
        Ok(Some(()))
    }

    fn read_item_descend(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        let res = self.read_item_descend_inner();
        if res.is_err() {
            self.state = ReadState::Done;
        }
        res
    }
}

impl<'blob, TC, S> StackBasedTreeCursor<'blob, TC, S>
where
    TC: TokenCursor<'blob>,
    S: NodeStack<TC::NodeHandle>,
{
    fn read_item_descend_inner(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        let (in_prop, in_node) = match self.state {
            ReadState::Init => (false, false),
            ReadState::Property => (true, true),
            ReadState::Child => (false, true),
            ReadState::Done => return Ok(None),
        };

        let position = self.token_cursor.position();
        let token = self
            .token_cursor
            .read_token()
            .map_err(|source| ReadTreeErrorKind::ReadToken { source })?;
        match token {
            Some(Token::Property(property)) => {
                ensure!(
                    in_prop,
                    ReadTreeErrorKind::UnexpectedPropertyToken { position }
                );
                Ok(Some(Item::Property(property)))
            }
            Some(Token::BeginNode(node)) => {
                let item = self.token_cursor.make_node_handle(&node);
                ensure!(
                    self.node_stack.push(item).is_ok(),
                    ReadTreeErrorKind::TooDeep { position }
                );
                self.state = ReadState::Property;
                Ok(Some(Item::Node(node)))
            }
            Some(Token::EndNode) => {
                ensure!(
                    in_node,
                    ReadTreeErrorKind::UnexpectedEndNodeToken { position }
                );
                self.state = ReadState::Done;
                Ok(None)
            }
            None => {
                self.state = ReadState::Done;
                bail!(ReadTreeErrorKind::UnexpectedEndOfTokens { position })
            }
        }
    }
}

pub struct StackBasedParents<'tc, 'blob, TC>
where
    TC: TokenCursor<'blob>,
{
    token_cursor: &'tc TC,
    iter: iter::Rev<slice::Iter<'tc, TC::NodeHandle>>,
}

impl<'tc, 'blob, TC> StackBasedParents<'tc, 'blob, TC>
where
    TC: TokenCursor<'blob>,
{
    pub fn new<S>(cursor: &'tc StackBasedTreeCursor<'blob, TC, S>) -> Self
    where
        TC: TokenCursor<'blob>,
        S: NodeStack<TC::NodeHandle>,
    {
        let token_cursor = cursor.token_cursor();
        let iter = cursor.node_stack.as_slice().iter().rev();
        StackBasedParents { token_cursor, iter }
    }
}

impl<'blob, TC> Iterator for StackBasedParents<'_, 'blob, TC>
where
    TC: TokenCursor<'blob>,
{
    type Item = Node<'blob>;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ref = self.iter.next()?;
        Some(self.token_cursor.get_node(node_ref))
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
#[cfg(test)]
mod tests {
    extern crate alloc;

    use alloc::vec::Vec;

    use super::*;
    use crate::{
        blob::Property,
        node_stack::types::SliceNodeStack,
        testing::{SliceNodeHandle, SliceTokenCursor},
    };

    fn walk_tree<'blob, TC>(
        cursor: &mut TC,
        result: &mut Vec<(Node<'blob>, usize, Item<'blob>)>,
    ) -> Result<(), ReadTreeError>
    where
        TC: TreeCursor<'blob>,
    {
        loop {
            let node = cursor.node();
            let depth = cursor.depth();
            if let Some(item) = cursor.read_item_descend()? {
                result.push((node, depth, item));
                continue;
            }
            if cursor.seek_parent_next()?.is_none() {
                break;
            }
        }
        Ok(())
    }

    #[test]
    fn test_read_root_node() {
        let tokens = &[
            Token::BeginNode(Node::new("")),
            Token::Property(Property::new("prop1", "value1")),
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();
        assert_eq!(cursor.node(), Node::new(""));
        assert_eq!(cursor.depth(), 0);
        assert_eq!(
            cursor.read_item_descend().unwrap(),
            Some(Item::Property(Property::new("prop1", "value1")))
        );
        assert!(cursor.read_item_descend().unwrap().is_none());
    }

    #[test]
    fn test_read_nested_nodes() {
        let root = || Node::new("");
        let root_p1 = || Property::new("root_prop1", "root_value1");
        let c1 = || Node::new("child1");
        let c1_p1 = || Property::new("child1_prop1", "child1_value1");
        let c2 = || Node::new("child2");
        let c2_p1 = || Property::new("child2_prop1", "child2_value1");
        let c3 = || Node::new("child3");
        let c3_p1 = || Property::new("child3_prop1", "child3_value1");
        let c4 = || Node::new("child4");
        let c5 = || Node::new("child5");
        let tokens = &[
            Token::BeginNode(root()),
            Token::Property(root_p1()),
            Token::BeginNode(c1()),
            Token::Property(c1_p1()),
            Token::EndNode,
            Token::BeginNode(c2()),
            Token::Property(c2_p1()),
            Token::BeginNode(c3()),
            Token::Property(c3_p1()),
            Token::BeginNode(c4()),
            Token::EndNode,
            Token::EndNode,
            Token::BeginNode(c5()),
            Token::EndNode,
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();
        let mut items = Vec::new();
        walk_tree(&mut cursor, &mut items).unwrap();
        assert_eq!(
            items,
            &[
                (root(), 0, root_p1().into()),
                (root(), 0, c1().into()),
                (c1(), 1, c1_p1().into()),
                (root(), 0, c2().into()),
                (c2(), 1, c2_p1().into()),
                (c2(), 1, Item::Node(c3())),
                (c3(), 2, c3_p1().into()),
                (c3(), 2, Item::Node(c4())),
                (c2(), 1, Item::Node(c5())),
            ]
        );
    }

    #[test]
    fn test_unexpected_property_token() {
        let root = || Node::new("");
        let c1 = || Node::new("child1");
        let p1 = || Property::new("prop1", "value1");

        let tokens = &[Token::Property(p1())];
        let tokens = SliceTokenCursor::new(tokens);
        let err = StackBasedTreeCursor::new(tokens).unwrap_err();
        assert!(
            matches!(
                err.kind(),
                ReadTreeErrorKind::UnexpectedPropertyToken { .. }
            ),
            "err: {err:?}",
        );

        let tokens = &[
            Token::BeginNode(root()),
            Token::BeginNode(c1()),
            Token::EndNode,
            Token::Property(p1()),
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();
        let mut items = Vec::new();
        let err = walk_tree(&mut cursor, &mut items).unwrap_err();
        assert_eq!(items, &[(root(), 0, c1().into())]);
        assert!(
            matches!(
                err.kind(),
                ReadTreeErrorKind::UnexpectedPropertyToken { .. }
            ),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unexpected_end_node_token() {
        let tokens = &[Token::EndNode];
        let tokens = SliceTokenCursor::new(tokens);
        let err = StackBasedTreeCursor::new(tokens).unwrap_err();
        assert!(
            matches!(err.kind(), ReadTreeErrorKind::UnexpectedEndNodeToken { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_unexpected_end_of_tokens() {
        let root = || Node::new("");
        let p1 = || Property::new("prop1", "value1");

        let tokens = SliceTokenCursor::new(&[]);
        let err = StackBasedTreeCursor::new(tokens).unwrap_err();
        assert!(
            matches!(err.kind(), ReadTreeErrorKind::UnexpectedEndOfTokens { .. }),
            "err: {err:?}",
        );

        let tokens = &[Token::BeginNode(root()), Token::Property(p1())];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();
        let mut items = Vec::new();
        let err = walk_tree(&mut cursor, &mut items).unwrap_err();
        assert_eq!(items, &[(root(), 0, p1().into())]);
        assert!(
            matches!(err.kind(), ReadTreeErrorKind::UnexpectedEndOfTokens { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_too_deep() {
        let root = || Node::new("");
        let n1 = || Node::new("n1");
        let n2 = || Node::new("n2");
        let n3 = || Node::new("n3");
        let n4 = || Node::new("n4");

        let tokens = &[
            Token::BeginNode(root()),
            Token::BeginNode(n1()),
            Token::BeginNode(n2()),
            Token::BeginNode(n3()),
            Token::BeginNode(n4()),
        ];

        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor =
            StackBasedTreeCursor::<_, ArrayNodeStack<_, 4>>::with_stack_size(tokens).unwrap();
        let mut items = Vec::new();
        let err = walk_tree(&mut cursor, &mut items).unwrap_err();
        assert_eq!(
            items,
            &[
                (root(), 0, n1().into()),
                (n1(), 1, n2().into()),
                (n2(), 2, n3().into()),
            ]
        );
        assert!(
            matches!(err.kind(), ReadTreeErrorKind::TooDeep { .. }),
            "err: {err:?}",
        );
    }

    #[test]
    fn test_clone_with_node_stack() {
        let tokens = &[
            Token::BeginNode(Node::new("")),
            Token::Property(Property::new("prop1", "value1")),
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let cursor = StackBasedTreeCursor::new(tokens).unwrap();
        let mut slice_stack = [SliceNodeHandle::default(); 8];
        let slice_stack = SliceNodeStack::new(&mut slice_stack);
        let cloned_cursor = cursor.clone_with_node_stack(slice_stack).unwrap();
        assert_eq!(cursor.node(), cloned_cursor.node());
        assert_eq!(cursor.depth(), cloned_cursor.depth());
    }

    #[test]
    fn test_try_clone() {
        let tokens = &[
            Token::BeginNode(Node::new("")),
            Token::Property(Property::new("prop1", "value1")),
            Token::BeginNode(Node::new("child")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let cursor = StackBasedTreeCursor::new(tokens).unwrap();

        let cloned = cursor.try_clone().unwrap();
        assert_eq!(cursor.node(), cloned.node());
        assert_eq!(cursor.depth(), cloned.depth());
        assert_eq!(cursor.state, cloned.state);
    }

    #[test]
    fn test_parents_iterator() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::BeginNode(Node::new("child1")),
            Token::BeginNode(Node::new("child2")),
            Token::BeginNode(Node::new("child3")),
            Token::EndNode,
            Token::EndNode,
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Navigate to child3
        cursor.read_item_descend().unwrap(); // child1
        cursor.read_item_descend().unwrap(); // child2
        cursor.read_item_descend().unwrap(); // child3

        let parents: Vec<_> = cursor.parents().map(|p| p.name()).collect();
        assert_eq!(&parents, &["child3", "child2", "child1", "root"]);
    }

    #[test]
    fn test_reset() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::Property(Property::new("root_prop", "value")),
            Token::BeginNode(Node::new("child")),
            Token::Property(Property::new("child_prop", "value")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Navigate to child
        cursor.read_item_descend().unwrap(); // root_prop
        cursor.read_item_descend().unwrap(); // child
        assert_eq!(cursor.depth(), 1);
        assert_eq!(cursor.node().name(), "child");

        // Reset to root
        cursor.reset();
        assert_eq!(cursor.depth(), 0);
        assert_eq!(cursor.node().name(), "root");
        assert_eq!(cursor.state, ReadState::Property);

        // Should be able to read root_prop again
        let item = cursor.read_item_descend().unwrap().unwrap();
        assert!(matches!(item, Item::Property(p) if p.name() == "root_prop"));
    }

    #[test]
    fn test_seek_operations() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::Property(Property::new("root_prop", "value")),
            Token::BeginNode(Node::new("child1")),
            Token::Property(Property::new("child1_prop", "value")),
            Token::EndNode,
            Token::BeginNode(Node::new("child2")),
            Token::Property(Property::new("child2_prop", "value")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Navigate to child1
        cursor.read_item_descend().unwrap(); // root_prop
        cursor.read_item_descend().unwrap(); // child1
        assert_eq!(cursor.node().name(), "child1");

        // Test seek_node_start
        cursor.seek_node_start();
        let item = cursor.read_item_descend().unwrap().unwrap();
        assert!(matches!(item, Item::Property(p) if p.name() == "child1_prop"));

        // Test seek_parent_start
        cursor.seek_parent_start().unwrap();
        assert_eq!(cursor.node().name(), "root");
        assert_eq!(cursor.depth(), 0);
        assert!(cursor.seek_parent_start().is_none());

        // Test seek_root_start from deeper level
        cursor.read_item_descend().unwrap(); // root_prop
        cursor.read_item_descend().unwrap(); // child1
        cursor.read_item_descend().unwrap(); // child1_prop
        cursor.seek_root_start();
        assert_eq!(cursor.node().name(), "root");
        assert_eq!(cursor.depth(), 0);
    }

    #[test]
    fn test_seek_node_end() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::Property(Property::new("prop1", "value1")),
            Token::Property(Property::new("prop2", "value2")),
            Token::BeginNode(Node::new("child")),
            Token::Property(Property::new("child_prop", "value")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Seek to end of root node
        cursor.seek_node_end().unwrap();
        assert!(cursor.read_item_descend().unwrap().is_none());
    }

    #[test]
    fn test_seek_parent_next() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::BeginNode(Node::new("child1")),
            Token::Property(Property::new("child1_prop", "value")),
            Token::EndNode,
            Token::BeginNode(Node::new("child2")),
            Token::Property(Property::new("child2_prop", "value")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Navigate to child1
        cursor.read_item_descend().unwrap(); // child1
        assert_eq!(cursor.node().name(), "child1");

        // Seek to next sibling (child2)
        cursor.seek_parent_next().unwrap().unwrap();
        let item = cursor.read_item_descend().unwrap().unwrap();
        assert!(matches!(item, Item::Node(n) if n.name() == "child2"));

        // At root level, should return None
        cursor.seek_parent_start().unwrap();
        assert!(cursor.seek_parent_next().unwrap().is_none());
    }

    #[test]
    fn test_empty_node() {
        let tokens = &[Token::BeginNode(Node::new("empty")), Token::EndNode];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        assert_eq!(cursor.node().name(), "empty");
        assert!(cursor.read_item_descend().unwrap().is_none());
    }

    #[test]
    fn test_node_with_only_properties() {
        let tokens = &[
            Token::BeginNode(Node::new("props_only")),
            Token::Property(Property::new("prop1", "value1")),
            Token::Property(Property::new("prop2", "value2")),
            Token::Property(Property::new("prop3", "value3")),
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        let mut properties = Vec::new();
        while let Some(item) = cursor.read_item_descend().unwrap() {
            if let Item::Property(prop) = item {
                properties.push(prop.name());
            }
        }

        assert_eq!(properties, alloc::vec!["prop1", "prop2", "prop3"]);
    }

    #[test]
    fn test_node_with_only_children() {
        let tokens = &[
            Token::BeginNode(Node::new("parent")),
            Token::BeginNode(Node::new("child1")),
            Token::EndNode,
            Token::BeginNode(Node::new("child2")),
            Token::EndNode,
            Token::BeginNode(Node::new("child3")),
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        let mut children = Vec::new();
        while let Some(item) = cursor.read_item_descend().unwrap() {
            if let Item::Node(node) = item {
                children.push(node.name());
                cursor.seek_parent_next().unwrap();
            }
        }

        assert_eq!(children, alloc::vec!["child1", "child2", "child3"]);
    }

    #[test]
    fn test_clone_with_insufficient_stack_space() {
        let tokens = &[
            Token::BeginNode(Node::new("root")),
            Token::BeginNode(Node::new("child1")),
            Token::BeginNode(Node::new("child2")),
            Token::EndNode,
            Token::EndNode,
            Token::EndNode,
        ];
        let tokens = SliceTokenCursor::new(tokens);
        let mut cursor = StackBasedTreeCursor::new(tokens).unwrap();

        // Navigate deep
        cursor.read_item_descend().unwrap(); // child1
        cursor.read_item_descend().unwrap(); // child2

        // Try to clone with insufficient stack space
        let mut small_stack = [SliceNodeHandle::default(); 1];
        let small_stack = SliceNodeStack::new(&mut small_stack);
        let result = cursor.clone_with_node_stack(small_stack);
        result.unwrap_err(); // Should return the failed stack
    }
}
