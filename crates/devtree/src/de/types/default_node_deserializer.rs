use super::DefaultPropertyDeserializer;
use crate::{
    blob::{Item, Node},
    de::{ItemDeserializer, NodeDeserializer, error::DeserializeError},
    tree_cursor::TreeCursor,
};

pub struct DefaultNodeDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    node: Node<'blob>,
    cursor: &'de mut TC,
}

impl<'de, 'blob, TC> DefaultNodeDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    pub fn new<'tc>(node: Node<'blob>, cursor: &'tc mut TC) -> Self
    where
        'tc: 'de,
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
    TC: TreeCursor<'blob>,
{
    fn drop(&mut self) {
        let _ = self.cursor.seek_parent_next();
    }
}
