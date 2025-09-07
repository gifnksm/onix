use core::ops::ControlFlow;

use either::Either;

use super::{DeserializeError, DeserializeNode, DeserializeProperty, error};
use crate::{
    Devicetree,
    blob::{Node, Property},
    cursor::{ItemCursor, NodeCursor, PropertyCursor},
    search::{IntoControlFlow, NodeQuery},
    types::property::Phandle,
};

#[derive(Debug, Clone)]
pub enum ItemContext<'ctx, 'blob> {
    Node(NodeContext<'ctx, 'blob>),
    Property(PropertyContext<'ctx, 'blob>),
}

impl<'ctx, 'blob> ItemContext<'ctx, 'blob> {
    pub fn as_node(&self) -> Option<&NodeContext<'ctx, 'blob>> {
        let Self::Node(node) = self else {
            return None;
        };
        Some(node)
    }

    pub fn as_property(&self) -> Option<&PropertyContext<'ctx, 'blob>> {
        let Self::Property(property) = self else {
            return None;
        };
        Some(property)
    }

    pub fn into_node(self) -> Option<NodeContext<'ctx, 'blob>> {
        let Self::Node(node) = self else {
            return None;
        };
        Some(node)
    }

    pub fn into_property(self) -> Option<PropertyContext<'ctx, 'blob>> {
        let Self::Property(property) = self else {
            return None;
        };
        Some(property)
    }
}

#[derive(Debug)]
pub struct NodeContext<'ctx, 'blob> {
    cursor: NodeCursor<'ctx, 'blob>,
}

impl Clone for NodeContext<'_, '_> {
    fn clone(&self) -> Self {
        Self {
            cursor: self.cursor.clone(),
        }
    }
}

impl<'ctx, 'blob> NodeContext<'ctx, 'blob> {
    #[must_use]
    pub(crate) fn new(cursor: NodeCursor<'ctx, 'blob>) -> Self {
        Self { cursor }
    }

    #[must_use]
    pub(crate) fn cursor(&self) -> &NodeCursor<'ctx, 'blob> {
        &self.cursor
    }

    #[must_use]
    pub fn node(&self) -> &Node<'blob> {
        self.cursor.node()
    }

    #[must_use]
    pub fn devicetree(&self) -> &'blob Devicetree {
        self.cursor.devicetree()
    }

    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        self.cursor.parent().cloned().map(NodeContext::new)
    }

    #[must_use]
    pub fn root<'root>(&self) -> NodeContext<'root, 'blob> {
        NodeContext {
            cursor: self.cursor.root(),
        }
    }

    pub fn deserialize_parent<T>(&self) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        self.deserialize_parent_with(T::deserialize_node)
    }

    pub fn deserialize_parent_with<T, F>(
        &self,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        let Some(nctx) = self.parent() else {
            return Ok(None);
        };
        let mut nctx = nctx;
        deserializer(&mut nctx).map(Some)
    }

    pub fn read_item(&mut self) -> Result<Option<ItemContext<'_, 'blob>>, DeserializeError> {
        let Some(item) = self.cursor.read_item().map_err(error::error_read_node)? else {
            return Ok(None);
        };
        let ictx = match item {
            ItemCursor::Property(cursor) => ItemContext::Property(PropertyContext::new(cursor)),
            ItemCursor::Node(cursor) => ItemContext::Node(NodeContext::new(cursor)),
        };
        Ok(Some(ictx))
    }

    pub fn deserialize_node<T>(&self) -> Result<T, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        self.deserialize_node_with(T::deserialize_node)
    }

    pub fn deserialize_node_with<T, F>(&self, deserializer: F) -> Result<T, DeserializeError>
    where
        F: FnOnce(&mut Self) -> Result<T, DeserializeError>,
    {
        deserializer(&mut self.clone())
    }

    pub fn deserialize_node_by_query<T>(
        &self,
        query: &NodeQuery,
    ) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        self.deserialize_node_by_query_with(query, T::deserialize_node)
    }

    pub fn deserialize_node_by_query_with<T, F>(
        &self,
        query: &NodeQuery,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        let res = self.visit_node_by_query(query, |mut nctx| deserializer(&mut nctx))?;
        res.transpose()
    }

    pub fn deserialize_node_by_phandle<T>(
        &self,
        phandle: Phandle,
    ) -> Result<Option<T>, DeserializeError>
    where
        T: DeserializeNode<'blob>,
    {
        self.deserialize_node_by_phandle_with(phandle, T::deserialize_node)
    }

    pub fn deserialize_node_by_phandle_with<T, F>(
        &self,
        phandle: Phandle,
        deserializer: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(&mut NodeContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        self.visit_node_by_phandle(phandle, |mut nctx| deserializer(&mut nctx))?
            .transpose()
    }

    pub fn visit_deserialize_all_nodes_by_query<Q, T, F>(
        &self,
        query: Q,
        mut f: F,
    ) -> Result<(), DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        T: DeserializeNode<'blob>,
        F: FnMut(T),
    {
        let query = query.as_ref();
        self.cursor
            .try_visit_all_nodes_by_query(query, |nctx| nctx.deserialize_node().map(&mut f))
            .map_err(error::error_read_node)?;
        Ok(())
    }

    pub fn try_visit_deserialize_all_nodes_by_query<Q, T, R, F, U>(
        &self,
        query: Q,
        mut f: F,
    ) -> Result<Option<U>, DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        T: DeserializeNode<'blob>,
        F: FnMut(T) -> R,
        R: IntoControlFlow<U>,
    {
        let query = query.as_ref();
        let res = self
            .cursor
            .try_visit_all_nodes_by_query(query, |nctx| {
                let value = nctx
                    .deserialize_node::<T>()
                    .into_control_flow()
                    .map_break(Either::Left)?;
                f(value).into_control_flow().map_break(Either::Right)?;
                ControlFlow::Continue(())
            })
            .map_err(error::error_read_node)?;
        match res {
            Some(Either::Left(err)) => Err(err),
            Some(Either::Right(value)) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    pub fn deserialize_property<T>(&self, name: &'static str) -> Result<T, DeserializeError>
    where
        T: DeserializeProperty<'blob>,
    {
        self.deserialize_property_with(name, T::deserialize_property)
    }

    pub fn deserialize_property_with<T, F>(
        &self,
        name: &'static str,
        deserializer: F,
    ) -> Result<T, DeserializeError>
    where
        F: FnOnce(&mut PropertyContext<'_, 'blob>) -> Result<T, DeserializeError>,
    {
        let mut nctx = self.clone();
        while let Some(ictx) = nctx.read_item()? {
            match ictx {
                ItemContext::Node(_) => break,
                ItemContext::Property(pctx) => {
                    if pctx.property().name() == name {
                        return pctx.deserialize_property_with(deserializer);
                    }
                }
            }
        }
        Err(nctx.error_missing_property(name))
    }
}

#[derive(Debug, Clone)]
pub struct PropertyContext<'ctx, 'blob> {
    cursor: PropertyCursor<'ctx, 'blob>,
}

impl<'ctx, 'blob> PropertyContext<'ctx, 'blob> {
    #[must_use]
    pub(crate) fn new(cursor: PropertyCursor<'ctx, 'blob>) -> Self {
        Self { cursor }
    }

    #[must_use]
    pub fn property(&self) -> &Property<'blob> {
        self.cursor.property()
    }

    #[must_use]
    pub fn node(&self) -> NodeContext<'ctx, 'blob> {
        NodeContext::new(self.cursor.node().clone())
    }

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
        deserializer(&mut self.clone())
    }
}

// Utility methods for derive macro
impl<'blob> NodeContext<'_, 'blob> {
    #[doc(hidden)]
    pub fn __with_parent_property<F>(&self, name: &[u8], f: F) -> Result<(), DeserializeError>
    where
        F: for<'p> FnOnce(PropertyContext<'p, 'blob>) -> Result<(), DeserializeError>,
    {
        let Some(mut parent) = self.cursor.parent().cloned() else {
            return Ok(());
        };
        while let Some(ItemCursor::Property(cursor)) =
            parent.read_item().map_err(error::error_read_node)?
        {
            let property = cursor.property();
            if property.name() == name {
                f(PropertyContext { cursor })?;
                break;
            }
        }
        Ok(())
    }

    #[doc(hidden)]
    pub fn __read_item_with<PH, NH>(
        &mut self,
        mut property_handler: PH,
        mut node_handler: NH,
    ) -> Result<(), DeserializeError>
    where
        PH: for<'p> FnMut(&'blob [u8], PropertyContext<'p, 'blob>) -> Result<(), DeserializeError>,
        NH: for<'n> FnMut(&'blob [u8], NodeContext<'n, 'blob>) -> Result<(), DeserializeError>,
    {
        while let Some(ictx) = self.read_item()? {
            match ictx {
                ItemContext::Property(pctx) => {
                    property_handler(pctx.property().name(), pctx)?;
                }
                ItemContext::Node(nctx) => {
                    node_handler(nctx.node().name(), nctx)?;
                }
            }
        }
        Ok(())
    }
}
