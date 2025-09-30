use core::{iter, marker::PhantomData, slice};

use crate::{
    blob::{Item, Node},
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
            .map_err(ReadTreeError::read_token)?;
        match token {
            Some(Token::Property(property)) => {
                if !in_prop {
                    return Err(ReadTreeError::unexpected_property_token(position));
                }
                Ok(Some(Item::Property(property)))
            }
            Some(Token::BeginNode(node)) => {
                let item = self.token_cursor.make_node_handle(&node);
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
