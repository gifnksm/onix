use crate::{blob::Property, de::PropertyDeserializer, tree_cursor::TreeCursor};

pub struct DefaultPropertyDeserializer<'de, 'blob, TC>
where
    TC: ?Sized,
{
    property: Property<'blob>,
    cursor: &'de TC,
}

impl<'de, 'blob, TC> DefaultPropertyDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob> + ?Sized,
{
    pub fn new<'property>(property: Property<'blob>, cursor: &'property TC) -> Self
    where
        'property: 'de,
    {
        Self { property, cursor }
    }
}

impl<'de, 'blob, TC> PropertyDeserializer<'de, 'blob>
    for DefaultPropertyDeserializer<'de, 'blob, TC>
where
    TC: TreeCursor<'blob>,
{
    type TreeCursor = TC;

    fn property(&self) -> &Property<'blob> {
        &self.property
    }

    fn tree_cursor(&self) -> &Self::TreeCursor {
        self.cursor
    }
}
