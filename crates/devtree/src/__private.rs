pub use core::{default::Default, marker::Sized, option::Option, result::Result};

pub use crate::{
    de::{
        DeserializeNode, DeserializeProperty, ItemDeserializer, NodeCollection, NodeDeserializer,
        PropertyCollection, PropertyDeserializer,
        error::DeserializeError,
        util::{NodeCell, PropertyCell},
    },
    tree_cursor::{TreeCursor, TreeNodeRef},
};

pub fn node_de_name<'de, 'blob, D>(de: &D) -> &'blob [u8]
where
    D: NodeDeserializer<'de, 'blob> + ?Sized,
{
    de.node().name()
}

pub fn node_de_with_items<'de, 'blob, D, PH, NH>(
    de: &mut D,
    property_handler: PH,
    node_handler: NH,
) -> Result<(), DeserializeError>
where
    D: NodeDeserializer<'de, 'blob> + ?Sized,
    PH: for<'sub_de> FnMut(D::PropertyDeserializer<'sub_de>) -> Result<(), DeserializeError>,
    NH: for<'sub_de> FnMut(D::NodeDeserializer<'sub_de>) -> Result<(), DeserializeError>,
{
    de.with_items(property_handler, node_handler)
}

pub fn prop_de_name<'de, 'blob, D>(de: &D) -> &'blob [u8]
where
    D: PropertyDeserializer<'de, 'blob> + ?Sized,
{
    de.property().name()
}
