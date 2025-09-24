#[cfg(feature = "alloc")]
pub use self::alloc::*;
use super::{
    Glob, debug_tree,
    error::ReadTreeError,
    iter::{self, DeserializedNodes},
};
use crate::{
    blob::{Item, Node},
    de::{
        DeserializeNode, NodeDeserializer,
        error::{DeserializeError, DeserializeNodeError},
    },
    token_cursor::TokenCursor,
    types::{ByteStr, property::Phandle},
};

#[cfg(feature = "alloc")]
mod alloc;

pub trait TreeCursor<'blob> {
    type TokenCursor: TokenCursor<'blob>;

    fn try_clone(&self) -> Option<Self>
    where
        Self: Sized;

    #[must_use]
    fn token_cursor(&self) -> &Self::TokenCursor;
    #[must_use]
    fn depth(&self) -> Option<usize>;
    #[must_use]
    fn node(&self) -> Option<Node<'blob>>;

    type Parents<'cursor>: Iterator<Item = Node<'blob>>
    where
        Self: 'cursor;

    #[must_use]
    fn parents(&self) -> Self::Parents<'_>;

    fn reset(&mut self);
    fn seek_node_start(&mut self) -> Option<Node<'blob>>;
    fn seek_node_end(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError>;
    fn seek_root_start(&mut self) -> Result<Node<'blob>, ReadTreeError>;
    fn seek_parent_start(&mut self) -> Option<Node<'blob>>;
    fn seek_parent_next(&mut self) -> Result<Option<Node<'blob>>, ReadTreeError>;
    fn read_item_descend(&mut self) -> Result<Option<Item<'blob>>, ReadTreeError>;

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
    ) -> Result<Option<Node<'blob>>, ReadTreeError> {
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
        let node = self.seek_node_start().unwrap();
        Ok(Some(node))
    }

    fn read_node_by_path<'path>(
        &mut self,
        path: &'path ByteStr,
    ) -> Result<Option<Node<'blob>>, ReadTreeError> {
        let mut iter = self.read_descendant_nodes_by_glob(path);
        if let Some(node) = iter.next() {
            let node = node?;
            return Ok(Some(node));
        }
        Ok(None)
    }

    type NodeDeserializer<'de>: NodeDeserializer<'de, 'blob>
    where
        Self: 'de;

    fn node_deserializer(&mut self) -> Result<Self::NodeDeserializer<'_>, DeserializeError>;

    fn deserialize_node<T>(&mut self) -> Result<T, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        let mut de = self.node_deserializer()?;
        T::deserialize_node(&mut de)
    }

    fn deserialize_parent<T>(&mut self) -> Result<T, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        let _parent = self.seek_parent_start().ok_or_else(|| {
            if let Some(node) = self.node() {
                DeserializeNodeError::missing_parent_node(&node).into()
            } else {
                DeserializeError::missing_current_node()
            }
        })?;
        self.deserialize_node()
    }

    fn deserialize_phandle_node<T>(
        &mut self,
        phandle: Phandle,
    ) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        let Some(_node) = self.read_node_by_phandle(phandle)? else {
            return Ok(None);
        };
        self.deserialize_node().map(Some)
    }

    #[must_use]
    fn debug_tree<'this>(&mut self) -> debug_tree::DebugTree<'this, '_, Self> {
        debug_tree::DebugTree::new(self)
    }
}

pub trait TreeIterator<'blob>: Iterator {
    type TreeCursor: TreeCursor<'blob> + ?Sized;

    #[must_use]
    fn tree_cursor(&self) -> &Self::TreeCursor;

    fn deserialize_node<T>(self) -> DeserializedNodes<T, Self>
    where
        Self: Iterator<Item = Result<Node<'blob>, ReadTreeError>> + Sized,
        Self::TreeCursor: Sized,
        T: DeserializeNode<'blob>,
    {
        DeserializedNodes::new(self)
    }
}
