use super::DefaultPropertyDeserializer;
use crate::{
    blob::{Item, Node},
    de::{ItemDeserializer, NodeDeserializer, error::DeserializeError},
    tree_cursor::TreeCursor,
};

pub struct DefaultNodeDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    node: Node<'blob>,
    cursor: &'de mut TC,
}

impl<'de, 'blob, TC> DefaultNodeDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new<'cursor>(node: Node<'blob>, cursor: &'cursor mut TC) -> Self
    where
        'cursor: 'de,
    {
        Self { node, cursor }
    }
}

impl<'de, 'blob, TC> NodeDeserializer<'de, 'blob> for DefaultNodeDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    type TreeCursor = TC;
    type PropertyDeserializer<'sub_de>
        = DefaultPropertyDeserializer<'sub_de, 'blob, TC>
    where
        Self: 'sub_de;
    type NodeDeserializer<'sub_de>
        = DefaultNodeDeserializer<'sub_de, 'blob, TC>
    where
        Self: 'sub_de;

    fn node(&self) -> &Node<'blob> {
        &self.node
    }

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.cursor
    }

    fn read_item(
        &mut self,
    ) -> Result<
        Option<ItemDeserializer<Self::PropertyDeserializer<'_>, Self::NodeDeserializer<'_>>>,
        DeserializeError,
    > {
        if self.cursor.depth().is_none() {
            let _root = self.cursor.seek_root_start()?;
        }
        let Some(item) = self.cursor.read_item_descend()? else {
            return Ok(None);
        };
        let sub_de = match item {
            Item::Property(property) => {
                ItemDeserializer::Property(DefaultPropertyDeserializer::new(property, self.cursor))
            }
            Item::Node(child) => {
                ItemDeserializer::Node(DefaultNodeDeserializer::new(child, self.cursor))
            }
        };
        Ok(Some(sub_de))
    }
}

impl<'blob, TC> Drop for DefaultNodeDeserializer<'_, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    fn drop(&mut self) {
        let _ = self.cursor.seek_parent_next();
    }
}
