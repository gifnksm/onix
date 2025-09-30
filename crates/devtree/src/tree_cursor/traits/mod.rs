#[cfg(feature = "alloc")]
pub use self::alloc::*;
use super::{Glob, debug_tree, error::ReadTreeError, iter};
use crate::{
    blob::{Item, Node, Property},
    de::{
        DeserializeNode, DeserializeProperty,
        error::DeserializeError,
        types::{DefaultNodeDeserializer, DefaultPropertyDeserializer},
    },
    token_cursor::TokenCursor,
    types::{ByteStr, property::Phandle},
};

#[cfg(feature = "alloc")]
mod alloc;

pub trait TreeCursor<'blob>: Sized {
    type TokenCursor: TokenCursor<'blob>;

    fn try_clone(&self) -> Option<Self>;

    #[must_use]
    fn token_cursor(&self) -> &Self::TokenCursor;
    #[must_use]
    fn depth(&self) -> usize;
    #[must_use]
    fn node(&self) -> Node<'blob>;

    type Parents<'tc>: Iterator<Item = Node<'blob>>
    where
        Self: 'tc;

    #[must_use]
    fn parents(&self) -> Self::Parents<'_>;

    fn reset(&mut self);
    fn seek_node_start(&mut self);
    fn seek_node_end(&mut self) -> Result<(), ReadTreeError>;
    fn seek_root_start(&mut self);
    fn seek_parent_start(&mut self) -> Option<()>;
    fn seek_parent_next(&mut self) -> Result<Option<()>, ReadTreeError>;
    fn read_item_descend(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError>;

    fn read_tree_item_ref_descend(
        &mut self,
    ) -> Result<Option<TreeItemRef<'_, 'blob, Self>>, ReadTreeError> {
        let Some(item) = self.read_item_descend()? else {
            return Ok(None);
        };
        let item_ref = match item {
            Item::Property(property) => TreePropertyRef::new(property, self).into(),
            Item::Node(node) => TreeNodeRef::new(node, self).into(),
        };
        Ok(Some(item_ref))
    }

    #[must_use]
    fn read_items(&mut self) -> iter::ReadItems<'_, 'blob, Self> {
        iter::ReadItems::new(self)
    }

    #[must_use]
    fn read_properties(&mut self) -> iter::ReadProperties<'_, 'blob, Self> {
        iter::ReadProperties::new(self)
    }

    #[must_use]
    fn read_children(&mut self) -> iter::ReadChildren<'_, 'blob, Self> {
        iter::ReadChildren::new(self)
    }

    fn read_node(&mut self) -> TreeNodeRef<'_, 'blob, Self> {
        self.seek_node_start();
        TreeNodeRef::new(self.node(), self)
    }

    fn read_parent(&mut self) -> Option<TreeNodeRef<'_, 'blob, Self>> {
        self.seek_parent_start()?;
        Some(TreeNodeRef::new(self.node(), self))
    }

    fn read_root(&mut self) -> TreeNodeRef<'_, 'blob, Self> {
        self.seek_root_start();
        TreeNodeRef::new(self.node(), self)
    }

    #[must_use]
    fn read_descendant_items(&mut self) -> iter::ReadDescendantItems<'_, 'blob, Self> {
        iter::ReadDescendantItems::new(self)
    }

    #[must_use]
    fn read_descendant_properties(&mut self) -> iter::ReadDescendantProperties<'_, 'blob, Self> {
        iter::ReadDescendantProperties::new(self)
    }

    #[must_use]
    fn read_descendant_nodes(&mut self) -> iter::ReadDescendantNodes<'_, 'blob, Self> {
        iter::ReadDescendantNodes::new(self)
    }

    #[must_use]
    fn read_descendant_nodes_by_glob<'glob, G>(
        &mut self,
        glob: &'glob G,
    ) -> iter::ReadDescendantNodesByGlob<'_, 'glob, 'blob, Self>
    where
        G: AsRef<Glob> + ?Sized + 'glob,
    {
        iter::ReadDescendantNodesByGlob::new(self, glob)
    }

    fn read_node_by_phandle(
        &mut self,
        phandle: Phandle,
    ) -> Result<Option<TreeNodeRef<'_, 'blob, Self>>, ReadTreeError> {
        let property = self
            .read_descendant_properties()
            .find(|property| {
                property.as_ref().map_or(true, |property| {
                    property.name() == "phandle"
                        && property.value() == phandle.value().to_be_bytes()
                })
            })
            .transpose()?;
        if property.is_none() {
            return Ok(None);
        }
        self.seek_node_start();
        Ok(Some(TreeNodeRef::new(self.node(), self)))
    }

    fn read_node_by_path<'path, P>(
        &mut self,
        path: &'path P,
    ) -> Result<Option<TreeNodeRef<'_, 'blob, Self>>, ReadTreeError>
    where
        P: AsRef<ByteStr> + ?Sized + 'path,
    {
        let mut iter = self.read_descendant_nodes_by_glob(path.as_ref());
        if let Some(node) = iter.next() {
            let node = node?;
            return Ok(Some(TreeNodeRef::new(node, self)));
        }
        Ok(None)
    }

    #[must_use]
    fn debug_tree<'this>(&mut self) -> debug_tree::DebugTree<'this, '_, Self> {
        debug_tree::DebugTree::new(self)
    }
}

pub trait TreeIterator<'blob>: Iterator {
    type TreeCursor: TreeCursor<'blob>;

    #[must_use]
    fn tree_cursor(&self) -> &Self::TreeCursor;

    fn deserialize_property<T>(self) -> iter::DeserializedProperties<T, Self>
    where
        Self: Iterator<Item = Result<Property<'blob>, ReadTreeError>> + Sized,
        Self::TreeCursor: Sized,
        T: DeserializeProperty<'blob>,
    {
        iter::DeserializedProperties::new(self)
    }

    fn deserialize_node<T>(self) -> iter::DeserializedNodes<T, Self>
    where
        Self: Iterator<Item = Result<Node<'blob>, ReadTreeError>> + Sized,
        Self::TreeCursor: Sized,
        T: DeserializeNode<'blob>,
    {
        iter::DeserializedNodes::new(self)
    }
}

#[derive(Debug, derive_more::From)]
pub enum TreeItemRef<'tc, 'blob, TC> {
    Property(TreePropertyRef<'tc, 'blob, TC>),
    Node(TreeNodeRef<'tc, 'blob, TC>),
}

impl<'tc, 'blob, TC> TreeItemRef<'tc, 'blob, TC> {
    #[must_use]
    pub fn as_property(&self) -> Option<&TreePropertyRef<'tc, 'blob, TC>> {
        match self {
            Self::Property(r) => Some(r),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn as_node(&self) -> Option<&TreeNodeRef<'tc, 'blob, TC>> {
        match self {
            Self::Property(_) => None,
            Self::Node(r) => Some(r),
        }
    }

    #[must_use]
    pub fn into_property(self) -> Option<TreePropertyRef<'tc, 'blob, TC>> {
        match self {
            Self::Property(r) => Some(r),
            Self::Node(_) => None,
        }
    }

    #[must_use]
    pub fn into_node(self) -> Option<TreeNodeRef<'tc, 'blob, TC>> {
        match self {
            Self::Property(_) => None,
            Self::Node(r) => Some(r),
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

#[derive(Debug)]
pub struct TreePropertyRef<'tc, 'blob, TC> {
    property: Property<'blob>,
    tree_cursor: &'tc mut TC,
}

impl<'tc, 'blob, TC> TreePropertyRef<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    fn new(property: Property<'blob>, tree_cursor: &'tc mut TC) -> Self {
        Self {
            property,
            tree_cursor,
        }
    }

    #[must_use]
    pub fn property(&self) -> &Property<'blob> {
        &self.property
    }

    #[must_use]
    pub fn tree_cursor(&self) -> &TC {
        self.tree_cursor
    }

    #[must_use]
    pub fn into_tree_cursor(self) -> &'tc mut TC {
        self.tree_cursor
    }

    #[must_use]
    pub fn property_deserializer(&self) -> DefaultPropertyDeserializer<'_, 'blob, TC> {
        DefaultPropertyDeserializer::new(self.property.clone(), self.tree_cursor)
    }

    pub fn deserialize_property<T>(self) -> Result<T, DeserializeError>
    where
        T: DeserializeProperty<'blob>,
    {
        let mut de = self.property_deserializer();
        T::deserialize_property(&mut de)
    }
}

#[derive(Debug)]
pub struct TreeNodeRef<'tc, 'blob, TC> {
    node: Node<'blob>,
    tree_cursor: &'tc mut TC,
}

impl<'tc, 'blob, TC> TreeNodeRef<'tc, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    fn new(node: Node<'blob>, tree_cursor: &'tc mut TC) -> Self {
        Self { node, tree_cursor }
    }

    #[must_use]
    pub fn node(&self) -> &Node<'blob> {
        &self.node
    }

    #[must_use]
    pub fn tree_cursor(&self) -> &TC {
        self.tree_cursor
    }

    #[must_use]
    pub fn into_tree_cursor(self) -> &'tc mut TC {
        self.tree_cursor
    }

    #[must_use]
    pub fn node_deserializer(&mut self) -> DefaultNodeDeserializer<'_, 'blob, TC> {
        DefaultNodeDeserializer::new(self.node.clone(), self.tree_cursor)
    }

    pub fn deserialize_node<T>(mut self) -> Result<T, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        let mut de = self.node_deserializer();
        T::deserialize_node(&mut de)
    }
}
