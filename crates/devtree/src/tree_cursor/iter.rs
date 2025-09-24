use core::{iter::FusedIterator, marker::PhantomData};

use super::{Glob, GlobCursor, TreeCursor, TreeIterator, error::ReadTreeError};
use crate::{
    blob::{Item, Node, Property},
    de::{DeserializeNode, error::DeserializeError},
    tree_cursor::GlobComponent,
};

#[derive(Debug)]
pub struct ReadItems<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    done: bool,
    last_found_node: bool,
    tree_cursor: &'tc mut TC,
    _phantom: PhantomData<&'blob ()>,
}

impl<'tc, 'blob, TC> ReadItems<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    #[must_use]
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            tree_cursor,
            done: false,
            last_found_node: false,
            _phantom: PhantomData,
        }
    }

    fn try_next(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        let res = self.try_next_inner();
        if res.is_err() {
            self.done = true;
        }
        res
    }

    fn try_next_inner(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        if self.last_found_node {
            self.last_found_node = false;
            self.tree_cursor.seek_parent_next()?;
        }

        let Some(item) = self.tree_cursor.read_item_descend()? else {
            self.done = true;
            return Ok(None);
        };

        if item.is_node() {
            self.last_found_node = true;
        }
        Ok(Some(item))
    }
}

impl<'blob, TC> Iterator for ReadItems<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Item<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

impl<'blob, TC> FusedIterator for ReadItems<'_, 'blob, TC> where TC: TreeCursor<'blob> + ?Sized {}

impl<'blob, TC> TreeIterator<'blob> for ReadItems<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.tree_cursor
    }
}

#[derive(Debug)]
pub struct ReadProperties<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    iter: ReadItems<'tc, 'blob, TC>,
}

impl<'tc, 'blob, TC> ReadProperties<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            iter: ReadItems::new(tree_cursor),
        }
    }
}

impl<'blob, TC> Iterator for ReadProperties<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Property<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(res) = self.iter.next()?.map(Item::into_property).transpose() {
                break Some(res);
            }
        }
    }
}

impl<'blob, TC> FusedIterator for ReadProperties<'_, 'blob, TC> where TC: TreeCursor<'blob> + ?Sized {}

impl<'blob, TC> TreeIterator<'blob> for ReadProperties<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.iter.tree_cursor()
    }
}

#[derive(Debug)]
pub struct ReadChildren<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    iter: ReadItems<'tc, 'blob, TC>,
}

impl<'tc, 'blob, TC> ReadChildren<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            iter: ReadItems::new(tree_cursor),
        }
    }
}

impl<'blob, TC> Iterator for ReadChildren<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Node<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(res) = self.iter.next()?.map(Item::into_node).transpose() {
                break Some(res);
            }
        }
    }
}

impl<'blob, TC> FusedIterator for ReadChildren<'_, 'blob, TC> where TC: TreeCursor<'blob> + ?Sized {}

impl<'blob, TC> TreeIterator<'blob> for ReadChildren<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.iter.tree_cursor()
    }
}

#[derive(Debug)]
pub struct ReadDescendantItems<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    min_depth: Option<usize>,
    done: bool,
    tree_cursor: &'tc mut TC,
    _phantom: PhantomData<&'blob ()>,
}

impl<'tc, 'blob, TC> ReadDescendantItems<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            min_depth: tree_cursor.depth(),
            done: false,
            tree_cursor,
            _phantom: PhantomData,
        }
    }

    fn try_next(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        let res = self.try_next_inner();
        if res.is_err() {
            self.done = true;
        }
        res
    }

    fn try_next_inner(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError> {
        if self.done {
            return Ok(None);
        }

        loop {
            if let Some(item) = self.tree_cursor.read_item_descend()? {
                return Ok(Some(item));
            }

            assert!(self.tree_cursor.depth() >= self.min_depth);
            if self.tree_cursor.depth() == self.min_depth {
                self.done = true;
                return Ok(None);
            }

            if self.tree_cursor.seek_parent_next()?.is_none() {
                self.done = true;
                return Ok(None);
            }
        }
    }
}

impl<'blob, TC> Iterator for ReadDescendantItems<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Item<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

impl<'blob, TC> FusedIterator for ReadDescendantItems<'_, 'blob, TC> where
    TC: TreeCursor<'blob> + ?Sized
{
}

impl<'blob, TC> TreeIterator<'blob> for ReadDescendantItems<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.tree_cursor
    }
}

#[derive(Debug)]
pub struct ReadDescendantProperties<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    iter: ReadDescendantItems<'tc, 'blob, TC>,
}

impl<'tc, 'blob, TC> ReadDescendantProperties<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            iter: ReadDescendantItems::new(tree_cursor),
        }
    }
}

impl<'blob, TC> Iterator for ReadDescendantProperties<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Property<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(res) = self.iter.next()?.map(Item::into_property).transpose() {
                break Some(res);
            }
        }
    }
}

impl<'blob, TC> FusedIterator for ReadDescendantProperties<'_, 'blob, TC> where
    TC: TreeCursor<'blob> + ?Sized
{
}

impl<'blob, TC> TreeIterator<'blob> for ReadDescendantProperties<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.iter.tree_cursor()
    }
}

#[derive(Debug)]
pub struct ReadDescendantNodes<'tc, 'blob, TC>
where
    TC: ?Sized,
{
    iter: ReadDescendantItems<'tc, 'blob, TC>,
}

impl<'tc, 'blob, TC> ReadDescendantNodes<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new(tree_cursor: &'tc mut TC) -> Self {
        Self {
            iter: ReadDescendantItems::new(tree_cursor),
        }
    }
}

impl<'blob, TC> Iterator for ReadDescendantNodes<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Node<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(res) = self.iter.next()?.map(Item::into_node).transpose() {
                break Some(res);
            }
        }
    }
}

impl<'blob, TC> FusedIterator for ReadDescendantNodes<'_, 'blob, TC> where
    TC: TreeCursor<'blob> + ?Sized
{
}

impl<'blob, TC> TreeIterator<'blob> for ReadDescendantNodes<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.iter.tree_cursor()
    }
}

#[derive(Debug)]
pub struct ReadDescendantNodesByGlob<'tc, 'glob, 'blob, TC>
where
    TC: ?Sized,
{
    min_depth: usize,
    done: bool,
    last_matched: bool,
    glob_cursor: GlobCursor<'glob>,
    tree_cursor: &'tc mut TC,
    _phantom: PhantomData<&'blob ()>,
}

impl<'tc, 'glob, 'blob, TC> ReadDescendantNodesByGlob<'tc, 'glob, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    #[must_use]
    pub fn new<Q>(tree_cursor: &'tc mut TC, glob: &'glob Q) -> Self
    where
        Q: AsRef<Glob> + ?Sized + 'glob,
    {
        let glob_cursor = glob.as_ref().cursor();
        Self {
            min_depth: tree_cursor.depth().unwrap_or(0),
            done: false,
            last_matched: false,
            glob_cursor,
            tree_cursor,
            _phantom: PhantomData,
        }
    }

    fn try_next(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError> {
        let res = self.try_next_inner();
        if res.is_err() {
            self.done = true;
        }
        res
    }

    fn rewind_cursors(&mut self) -> Result<Option<()>, ReadTreeError> {
        let depth = self.tree_cursor.depth().unwrap();
        assert!(depth >= self.min_depth);
        if depth == self.min_depth {
            return Ok(None);
        }
        let is_root = self.tree_cursor.seek_parent_next()?.is_none();
        if is_root {
            return Ok(None);
        }
        self.glob_cursor.seek_ascend().unwrap();
        Ok(Some(()))
    }

    fn try_next_inner(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError> {
        if self.done {
            return Ok(None);
        }

        if self.last_matched {
            self.last_matched = false;
            if self.rewind_cursors()?.is_none() {
                self.done = true;
                return Ok(None);
            }
        }

        // seek to root if no node have been read
        if self.tree_cursor.node().is_none() {
            let _root = self.tree_cursor.seek_root_start()?;
            assert!(self.tree_cursor.node().is_some());
        }

        loop {
            let depth = self.tree_cursor.depth().unwrap();

            let Some(component) = self.glob_cursor.current_component() else {
                self.last_matched = true;
                let node = self.tree_cursor.node().unwrap();
                return Ok(Some(node));
            };

            if component == GlobComponent::RootNode && depth == 0 {
                let _root = self.tree_cursor.seek_root_start()?;
                self.glob_cursor.seek_descend();
                continue;
            }

            let matched = self
                .tree_cursor
                .read_children()
                .find(|res| {
                    res.as_ref()
                        .map_or(true, |child| component.match_node(child))
                })
                .transpose()?
                .is_some();

            if !matched {
                assert_eq!(self.tree_cursor.depth(), Some(depth));
                if self.rewind_cursors()?.is_none() {
                    self.done = true;
                    return Ok(None);
                }
                continue;
            }

            assert_eq!(self.tree_cursor.depth(), Some(depth + 1));
            self.glob_cursor.seek_descend();
        }
    }
}

impl<'blob, TC> Iterator for ReadDescendantNodesByGlob<'_, '_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type Item = Result<Node<'blob>, ReadTreeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

impl<'blob, TC> FusedIterator for ReadDescendantNodesByGlob<'_, '_, 'blob, TC> where
    TC: TreeCursor<'blob> + ?Sized
{
}

impl<'blob, TC> TreeIterator<'blob> for ReadDescendantNodesByGlob<'_, '_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    type TreeCursor = TC;

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.tree_cursor
    }
}

#[derive(Debug)]
pub struct DeserializedNodes<T, I> {
    iter: I,
    _phantom: PhantomData<T>,
}

impl<'blob, T, I> DeserializedNodes<T, I>
where
    T: DeserializeNode<'blob>,
    I: Iterator<Item = Result<Node<'blob>, ReadTreeError>> + TreeIterator<'blob>,
    <I as TreeIterator<'blob>>::TreeCursor: Sized,
{
    #[must_use]
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            _phantom: PhantomData,
        }
    }

    fn try_next(&mut self) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        let Some(_node) = self.iter.next().transpose()? else {
            return Ok(None);
        };
        let mut cursor = self
            .iter
            .tree_cursor()
            .try_clone()
            .ok_or_else(DeserializeError::clone_not_supported)?;
        cursor.deserialize_node().map(Some)
    }
}

impl<'blob, T, I> Iterator for DeserializedNodes<T, I>
where
    T: DeserializeNode<'blob>,
    I: Iterator<Item = Result<Node<'blob>, ReadTreeError>> + TreeIterator<'blob>,
    <I as TreeIterator<'blob>>::TreeCursor: Sized,
{
    type Item = Result<T, DeserializeError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}
