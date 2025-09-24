use core::{iter, marker::PhantomData, slice};

use crate::{
    blob::{Item, Node},
    de::{error::DeserializeError, types::DefaultNodeDeserializer},
    node_stack::{NodeStack, error::StackOverflowError, types::ArrayNodeStack},
    token_cursor::{Token, TokenCursor},
    tree_cursor::{TreeCursor, error::ReadTreeError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadState {
    Init,
    Property,
    Child,
    Done,
}

#[derive(Debug, Clone)]
pub struct StackBasedTreeCursor<'blob, C, S = ArrayNodeStack<<C as TokenCursor<'blob>>::NodeRef, 8>>
where
    C: TokenCursor<'blob>,
{
    node_stack: S,
    state: ReadState,
    token_cursor: C,
    _phantom: PhantomData<&'blob ()>,
}

impl<'blob, C> StackBasedTreeCursor<'blob, C>
where
    C: TokenCursor<'blob>,
{
    #[must_use]
    pub fn new(token_cursor: C) -> Self {
        Self::with_node_stack(token_cursor, ArrayNodeStack::default())
    }
}

impl<'blob, C, const STACK_SIZE: usize>
    StackBasedTreeCursor<'blob, C, ArrayNodeStack<<C as TokenCursor<'blob>>::NodeRef, STACK_SIZE>>
where
    C: TokenCursor<'blob>,
{
    #[must_use]
    pub fn with_stack_size(token_cursor: C) -> Self {
        StackBasedTreeCursor::with_node_stack(token_cursor, ArrayNodeStack::default())
    }
}

impl<'blob, C, S> StackBasedTreeCursor<'blob, C, S>
where
    C: TokenCursor<'blob>,
    S: NodeStack<C::NodeRef>,
{
    #[must_use]
    pub fn with_node_stack(token_cursor: C, node_stack: S) -> Self {
        Self {
            node_stack,
            state: ReadState::Init,
            token_cursor,
            _phantom: PhantomData,
        }
    }

    pub fn clone_with_node_stack<U>(
        &self,
        mut node_stack: U,
    ) -> Result<StackBasedTreeCursor<'blob, C, U>, U>
    where
        U: NodeStack<C::NodeRef>,
    {
        let res = node_stack.clone_from_stack(&self.node_stack);
        if matches!(res, Err(StackOverflowError)) {
            return Err(node_stack);
        }
        Ok(StackBasedTreeCursor {
            node_stack,
            state: self.state,
            token_cursor: self.token_cursor.clone(),
            _phantom: PhantomData,
        })
    }
}

impl<'blob, C, S> TreeCursor<'blob> for StackBasedTreeCursor<'blob, C, S>
where
    C: TokenCursor<'blob>,
    S: NodeStack<C::NodeRef>,
{
    type TokenCursor = C;

    fn try_clone(&self) -> Option<Self> {
        let node_stack = self.node_stack.try_clone()?;
        Some(Self {
            node_stack,
            state: self.state,
            token_cursor: self.token_cursor.clone(),
            _phantom: PhantomData,
        })
    }

    fn token_cursor(&self) -> &C {
        &self.token_cursor
    }

    fn depth(&self) -> Option<usize> {
        self.node_stack.len().checked_sub(1)
    }

    fn node(&self) -> Option<Node<'blob>> {
        let node_ref = self.node_stack.current()?;
        Some(self.token_cursor.get_node(node_ref))
    }

    type Parents<'cursor>
        = StackBasedParents<'cursor, 'blob, C>
    where
        Self: 'cursor;

    fn parents(&self) -> Self::Parents<'_> {
        Self::Parents::new(self)
    }

    fn reset(&mut self) {
        self.node_stack.clear();
        self.state = ReadState::Init;
        self.token_cursor.reset();
    }

    fn seek_node_start(&mut self) -> Option<Node<'blob>> {
        let node_ref = self.node_stack.current()?;
        self.state = ReadState::Property;
        self.token_cursor.seek_item_start_of_node(node_ref);
        let node = self.token_cursor.get_node(node_ref);
        Some(node)
    }

    fn seek_node_end(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError> {
        let Some(node_ref) = self.node_stack.current().cloned() else {
            return Ok(None);
        };
        while let Some(item) = self.read_item_descend()? {
            match item {
                Item::Property(_) => {}
                Item::Node(_) => {
                    let _node = self.seek_parent_next()?.unwrap();
                }
            }
        }
        let node = self.token_cursor.get_node(&node_ref);
        Ok(Some(node))
    }

    fn seek_root_start(&mut self) -> Result<Node<'blob>, ReadTreeError> {
        if self.node_stack.is_empty() {
            self.reset();
            let Some(item) = self.read_item_descend()? else {
                return Err(ReadTreeError::no_root_node());
            };
            let node = item.into_node().unwrap();
            return Ok(node);
        }

        while self.node_stack.len() > 1 {
            self.node_stack.pop().unwrap();
        }
        let node = self.seek_node_start().unwrap();
        Ok(node)
    }

    fn seek_parent_start(&mut self) -> Option<Node<'blob>> {
        if self.node_stack.len() <= 1 {
            return None;
        }
        self.node_stack.pop().unwrap();
        let parent = self.seek_node_start().unwrap();
        Some(parent)
    }

    fn seek_parent_next(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError> {
        if self.node_stack.len() <= 1 {
            return Ok(None);
        }
        let _node = self.seek_node_end()?.unwrap();
        self.node_stack.pop().unwrap();
        let parent_ref = self.node_stack.current().unwrap();
        let parent = self.token_cursor.get_node(parent_ref);
        self.state = ReadState::Child;
        Ok(Some(parent))
    }

    fn read_item_descend(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        let res = self.read_item_descend_inner();
        if res.is_err() {
            self.state = ReadState::Done;
        }
        res
    }

    type NodeDeserializer<'de>
        = DefaultNodeDeserializer<'de, 'blob, Self>
    where
        Self: 'de;

    fn node_deserializer(&mut self) -> Result<Self::NodeDeserializer<'_>, DeserializeError> {
        if self.node_stack.is_empty() {
            let _root = self.seek_root_start()?;
        }
        let node = self
            .seek_node_start()
            .ok_or_else(DeserializeError::missing_current_node)?;
        Ok(DefaultNodeDeserializer::new(node, self))
    }
}

impl<'blob, C, S> StackBasedTreeCursor<'blob, C, S>
where
    C: TokenCursor<'blob>,
    S: NodeStack<C::NodeRef>,
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
            .map_err(ReadTreeError::read_token)?;
        match token {
            Some(Token::Property(property)) => {
                if !in_prop {
                    return Err(ReadTreeError::unexpected_property_token(position));
                }
                Ok(Some(Item::Property(property)))
            }
            Some(Token::BeginNode(node)) => {
                if in_prop {
                    self.state = ReadState::Child;
                }
                let item = self.token_cursor.make_node_ref(&node);
                if matches!(self.node_stack.push(item), Err(StackOverflowError)) {
                    return Err(ReadTreeError::too_deep());
                }
                self.state = ReadState::Property;
                Ok(Some(Item::Node(node)))
            }
            Some(Token::EndNode) => {
                if !in_node {
                    return Err(ReadTreeError::unexpected_end_node_token(position));
                }
                self.state = ReadState::Done;
                Ok(None)
            }
            None => {
                self.state = ReadState::Done;
                Err(ReadTreeError::unexpected_end_of_tokens(position))
            }
        }
    }
}

pub struct StackBasedParents<'cursor, 'blob, C>
where
    C: TokenCursor<'blob>,
{
    token_cursor: &'cursor C,
    iter: iter::Rev<slice::Iter<'cursor, C::NodeRef>>,
}

impl<'cursor, 'blob, C> StackBasedParents<'cursor, 'blob, C>
where
    C: TokenCursor<'blob>,
{
    pub fn new<S>(cursor: &'cursor StackBasedTreeCursor<'blob, C, S>) -> Self
    where
        C: TokenCursor<'blob>,
        S: NodeStack<C::NodeRef>,
    {
        let token_cursor = cursor.token_cursor();
        let iter = cursor.node_stack.as_slice().iter().rev();
        StackBasedParents { token_cursor, iter }
    }
}

impl<'blob, C> Iterator for StackBasedParents<'_, 'blob, C>
where
    C: TokenCursor<'blob>,
{
    type Item = Node<'blob>;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ref = self.iter.next()?;
        Some(self.token_cursor.get_node(node_ref))
    }
}
