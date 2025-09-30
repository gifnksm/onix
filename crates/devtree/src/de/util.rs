use platform_cast::CastFrom as _;

use super::{
    DeserializeNode, DeserializeProperty, NodeDeserializer, PropertyCollection,
    PropertyDeserializer,
    error::{DeserializeError, DeserializeNodeError, DeserializePropertyError},
};
use crate::{blob::Node, tree_cursor::TreeCursor as _};

pub fn deserialize_property_as_usize_via_u32<'de, 'blob, D>(
    de: &mut D,
) -> Result<usize, DeserializeError>
where
    D: PropertyDeserializer<'de, 'blob>,
{
    let value = u32::deserialize_property(de)?;
    Ok(usize::cast_from(value))
}

pub fn deserialize_u64_or_u32_property<'de, 'blob, D>(de: &mut D) -> Result<u64, DeserializeError>
where
    D: PropertyDeserializer<'de, 'blob>,
{
    let value = de.property().value();
    let value = match value.len() {
        4 => u64::from(u32::deserialize_property(de)?),
        8 => u64::deserialize_property(de)?,
        _ => {
            return Err(
                DeserializePropertyError::custom(de.property(), "u32 or u64 expected").into(),
            );
        }
    };
    Ok(value)
}

pub fn deserialize_node_as_property_collection<'de, 'blob, D, T>(
    de: &mut D,
) -> Result<T, DeserializeError>
where
    D: NodeDeserializer<'de, 'blob>,
    T: PropertyCollection<'blob>,
{
    let mut collection = T::default();
    while let Some(sub_de) = de.read_item()? {
        if let Some(mut sub_de) = sub_de.into_property() {
            collection.insert_property(&mut sub_de)?;
        } else {
            break;
        }
    }
    Ok(collection)
}

#[derive(Debug)]
pub struct PropertyCell<'blob, T> {
    node: Node<'blob>,
    name: &'static str,
    value: Option<T>,
}

impl<'blob, T> PropertyCell<'blob, T> {
    pub fn new<'de, D>(de: &D, name: &'static str) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let node = de.tree_cursor().node();
        Ok(Self {
            node,
            name,
            value: None,
        })
    }

    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub fn set_deserialized<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        T: DeserializeProperty<'blob>,
        D: PropertyDeserializer<'de, 'blob> + ?Sized,
    {
        let value = T::deserialize_property(de)?;
        self.set(value)?;
        Ok(())
    }

    pub fn set(&mut self, value: T) -> Result<(), DeserializeError> {
        if self.value.is_some() {
            return Err(DeserializeNodeError::duplicated_property(&self.node, self.name).into());
        }
        self.value = Some(value);
        Ok(())
    }

    pub fn finish(self) -> Result<T, DeserializeError> {
        self.value
            .ok_or_else(|| DeserializeNodeError::missing_property(&self.node, self.name).into())
    }

    pub fn finish_or_default(self) -> T
    where
        T: Default,
    {
        self.value.unwrap_or_default()
    }

    pub fn finish_or_else<F>(self, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        self.value.unwrap_or_else(f)
    }
}

#[derive(Debug)]
pub struct NodeCell<'blob, T> {
    node: Node<'blob>,
    name: &'static str,
    value: Option<T>,
}

impl<'blob, T> NodeCell<'blob, T> {
    pub fn new<'de, D>(de: &D, name: &'static str) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let node = de.tree_cursor().node();
        Ok(Self {
            node,
            name,
            value: None,
        })
    }

    pub fn has_value(&self) -> bool {
        self.value.is_some()
    }

    pub fn set_deserialized<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        T: DeserializeNode<'blob>,
        D: NodeDeserializer<'de, 'blob> + ?Sized,
    {
        let value = T::deserialize_node(de)?;
        self.set(value)?;
        Ok(())
    }

    pub fn set(&mut self, value: T) -> Result<(), DeserializeError> {
        if self.value.is_some() {
            return Err(DeserializeNodeError::duplicated_child(&self.node, self.name).into());
        }
        self.value = Some(value);
        Ok(())
    }

    pub fn finish(self) -> Result<T, DeserializeError> {
        self.value
            .ok_or_else(|| DeserializeNodeError::missing_child(&self.node, self.name).into())
    }

    pub fn finish_or_default(self) -> T
    where
        T: Default,
    {
        self.value.unwrap_or_default()
    }
}
