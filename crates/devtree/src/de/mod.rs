pub use self::{context::*, error::*};
use crate::{
    cursor::{NodeCursor, PropertyCursor},
    search::{IntoControlFlow, NodeQuery},
    types::property::Phandle,
};

mod collections;
mod context;
pub(crate) mod error;
mod impls;
pub mod util;

pub trait DeserializeNode<'blob>: Sized {
    fn deserialize_node(nctx: &mut NodeContext<'_, 'blob>) -> Result<Self, DeserializeError>;
}

pub trait DeserializeProperty<'blob>: Sized {
    fn deserialize_property(
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<Self, DeserializeError>;
}

pub trait NodeCollection<'blob>: Default {
    fn insert_node(&mut self, nctx: &mut NodeContext<'_, 'blob>) -> Result<(), DeserializeError>;
}

pub trait PropertyCollection<'blob>: Default {
    fn insert_property(
        &mut self,
        pctx: &mut PropertyContext<'_, 'blob>,
    ) -> Result<(), DeserializeError>;
}

impl<'blob> NodeCursor<'_, 'blob> {
    pub fn deserialize_parent<T>(&self) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        NodeContext::new(self.clone()).deserialize_parent()
    }

    pub fn deserialize_parent_with<T, F>(
        &self,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        NodeContext::new(self.clone()).deserialize_parent_with(deserializer)
    }

    pub fn deserialize_node<T>(&self) -> Result<T, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        NodeContext::new(self.clone()).deserialize_node()
    }

    pub fn deserialize_node_with<T, F>(&self, deserializer: F) -> Result<T, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        NodeContext::new(self.clone()).deserialize_node_with(deserializer)
    }

    pub fn deserialize_node_by_query<T>(
        &self,
        query: &NodeQuery,
    ) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        NodeContext::new(self.clone()).deserialize_node_by_query(query)
    }

    pub fn deserialize_node_by_query_with<T, F>(
        &self,
        query: &NodeQuery,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        NodeContext::new(self.clone()).deserialize_node_by_query_with(query, deserializer)
    }

    pub fn deserialize_node_by_phandle<T>(
        &self,
        phandle: Phandle,
    ) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        NodeContext::new(self.clone()).deserialize_node_by_phandle(phandle)
    }

    pub fn deserialize_node_by_phandle_with<T, F>(
        &self,
        phandle: Phandle,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        NodeContext::new(self.clone()).deserialize_node_by_phandle_with(phandle, deserializer)
    }

    pub fn visit_deserialize_all_nodes_by_query<Q, T, F>(
        &self,
        query: Q,
        f: F,
    ) -> Result<(), DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        T: DeserializeNode<'blob>,
        F: FnMut(T),
    {
        NodeContext::new(self.clone()).visit_deserialize_all_nodes_by_query(query, f)
    }

    pub fn try_visit_deserialize_all_nodes_by_query<Q, T, R, F, U>(
        &self,
        query: Q,
        f: F,
    ) -> Result<Option<U>, DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        T: DeserializeNode<'blob>,
        F: FnMut(T) -> R,
        R: IntoControlFlow<U>,
    {
        NodeContext::new(self.clone()).try_visit_deserialize_all_nodes_by_query(query, f)
    }

    pub fn deserialize_property<T>(&self, name: &'static str) -> Result<T, DeserializeError>
    where
        T: DeserializeProperty<'blob>,
    {
        NodeContext::new(self.clone()).deserialize_property(name)
    }

    pub fn deserialize_property_with<T, F>(
        &self,
        name: &'static str,
        deserializer: F,
    ) -> Result<T, DeserializeError>
    where
        F: FnOnce(&mut PropertyContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        NodeContext::new(self.clone()).deserialize_property_with(name, deserializer)
    }
}

impl<'blob> PropertyCursor<'_, 'blob> {
    pub fn deserialize_property<T>(&self) -> Result<T, DeserializeError>
    where
        T: DeserializeProperty<'blob>,
    {
        self.deserialize_property_with(T::deserialize_property)
    }

    pub fn deserialize_property_with<T, F>(&self, deserializer: F) -> Result<T, DeserializeError>
    where
        F: FnOnce(&mut PropertyContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        let mut pctx = PropertyContext::new(self.clone());
        deserializer(&mut pctx)
    }
}
