use super::error::{DeserializeError, DeserializeNodeError};
use crate::{
    blob::{Node, Property},
    tree_cursor::TreeCursor,
};

pub trait DeserializeProperty<'blob>: Sized {
    fn deserialize_property<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized;
}

pub trait DeserializeNode<'blob>: Sized {
    fn deserialize_node<'de, D>(de: &mut D) -> Result<Self, DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized;
}

pub trait NodeCollection<'blob>: Default {
    fn insert_node<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: NodeDeserializer<'de, 'blob> + ?Sized;
}

pub trait PropertyCollection<'blob>: Default {
    fn insert_property<'de, D>(&mut self, de: &mut D) -> Result<(), DeserializeError>
    where
        D: PropertyDeserializer<'de, 'blob> + ?Sized;
}

pub enum ItemDeserializer<PD, ND> {
    Property(PD),
    Node(ND),
}

impl<PD, ND> ItemDeserializer<PD, ND> {
    pub fn as_property(&self) -> Option<&PD> {
        match self {
            Self::Property(de) => Some(de),
            Self::Node(_) => None,
        }
    }

    pub fn as_node(&self) -> Option<&ND> {
        match self {
            Self::Property(_) => None,
            Self::Node(de) => Some(de),
        }
    }

    pub fn into_property(self) -> Option<PD> {
        match self {
            Self::Property(de) => Some(de),
            Self::Node(_) => None,
        }
    }

    pub fn into_node(self) -> Option<ND> {
        match self {
            Self::Property(_) => None,
            Self::Node(de) => Some(de),
        }
    }

    pub fn is_property(&self) -> bool {
        matches!(self, Self::Property(_))
    }

    pub fn is_node(&self) -> bool {
        matches!(self, Self::Node(_))
    }
}

pub trait PropertyDeserializer<'de, 'blob> {
    type TreeCursor: TreeCursor<'blob>;

    fn property(&self) -> &Property<'blob>;
    fn tree_cursor(&self) -> &Self::TreeCursor;

    fn clone_tree_cursor(&self) -> Result<Self::TreeCursor, DeserializeError>
    where
        Self::TreeCursor: Sized,
    {
        self.tree_cursor()
            .try_clone()
            .ok_or_else(DeserializeError::clone_not_supported)
    }
}
pub trait NodeDeserializer<'de, 'blob> {
    type TreeCursor: TreeCursor<'blob>;
    type PropertyDeserializer<'sub_de>: PropertyDeserializer<'sub_de, 'blob>
    where
        Self: 'sub_de;
    type NodeDeserializer<'sub_de>: NodeDeserializer<'sub_de, 'blob>
    where
        Self: 'sub_de;

    fn node(&self) -> &Node<'blob>;
    fn tree_cursor(&self) -> &Self::TreeCursor;

    fn clone_tree_cursor(&self) -> Result<Self::TreeCursor, DeserializeError>
    where
        Self::TreeCursor: Sized,
    {
        self.tree_cursor()
            .try_clone()
            .ok_or_else(DeserializeError::clone_not_supported)
    }

    fn read_item(
        &mut self,
    ) -> Result<
        Option<ItemDeserializer<Self::PropertyDeserializer<'_>, Self::NodeDeserializer<'_>>>,
        DeserializeError,
    >;

    fn with_items<PH, NH>(
        &mut self,
        mut property_handler: PH,
        mut node_handler: NH,
    ) -> Result<(), DeserializeError>
    where
        PH: for<'sub_de> FnMut(Self::PropertyDeserializer<'sub_de>) -> Result<(), DeserializeError>,
        NH: for<'sub_de> FnMut(Self::NodeDeserializer<'sub_de>) -> Result<(), DeserializeError>,
    {
        while let Some(sub_de) = self.read_item()? {
            match sub_de {
                ItemDeserializer::Property(sub_de) => {
                    property_handler(sub_de)?;
                }
                ItemDeserializer::Node(sub_de) => {
                    node_handler(sub_de)?;
                }
            }
        }
        Ok(())
    }

    fn with_properties<PH>(&mut self, property_handler: PH) -> Result<(), DeserializeError>
    where
        PH: for<'sub_de> FnMut(Self::PropertyDeserializer<'sub_de>) -> Result<(), DeserializeError>,
    {
        self.with_items(property_handler, |_| Ok(()))
    }

    fn with_children<NH>(&mut self, node_handler: NH) -> Result<(), DeserializeError>
    where
        NH: for<'sub_de> FnMut(Self::NodeDeserializer<'sub_de>) -> Result<(), DeserializeError>,
    {
        self.with_items(|_| Ok(()), node_handler)
    }

    fn with_node_de<F, T>(&self, f: F) -> Result<T, DeserializeError>
    where
        F: FnOnce(
            Node<'blob>,
            <Self::TreeCursor as TreeCursor<'blob>>::NodeDeserializer<'_>,
        ) -> Result<T, DeserializeError>,
    {
        let mut cursor = self.clone_tree_cursor()?;
        cursor
            .seek_node_start()
            .ok_or_else(DeserializeError::missing_current_node)?;
        let node = cursor.node().unwrap();
        let de = cursor.node_deserializer()?;
        f(node, de)
    }

    fn with_parent_de<F, T>(&self, f: F) -> Result<T, DeserializeError>
    where
        F: FnOnce(
            Node<'blob>,
            <Self::TreeCursor as TreeCursor<'blob>>::NodeDeserializer<'_>,
        ) -> Result<T, DeserializeError>,
    {
        let mut cursor = self.clone_tree_cursor()?;
        cursor.seek_parent_start().ok_or_else(|| {
            if let Some(node) = cursor.node() {
                DeserializeNodeError::missing_parent_node(&node).into()
            } else {
                DeserializeError::missing_current_node()
            }
        })?;
        let node = cursor.node().unwrap();
        let de = cursor.node_deserializer()?;
        f(node, de)
    }
}
