use crate::{
    blob::{Node, Property},
    de::{
        PropertyDeserializer,
        error::{DeserializeError, DeserializeResult},
    },
    tree_cursor::TreeCursor,
};

pub struct DefaultPropertyDeserializer<'de, 'blob, TC> {
    node: Node<'blob>,
    property: Property<'blob>,
    cursor: &'de TC,
}

impl<'de, 'blob, TC> DefaultPropertyDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    pub fn new<'property>(property: Property<'blob>, cursor: &'property TC) -> Self
    where
        'property: 'de,
    {
        Self {
            node: cursor.node(),
            property,
            cursor,
        }
    }
}

impl<'de, 'blob, TC> PropertyDeserializer<'de, 'blob>
    for DefaultPropertyDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    type TreeCursor = TC;

    fn node(&self) -> &Node<'blob> {
        &self.node
    }

    fn property(&self) -> &Property<'blob> {
        &self.property
    }

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.cursor
    }

    fn clone_tree_cursor(&self) -> DeserializeResult<Self::TreeCursor>
    where
        Self::TreeCursor: Sized,
    {
        self.tree_cursor()
            .try_clone()
            .ok_or_else(DeserializeError::clone_not_supported)
    }
}
