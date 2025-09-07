use core::ops::ControlFlow;

pub use self::query::*;
use crate::{
    cursor::{ItemCursor, NodeCursor, ReadNodeError},
    de::{self, DeserializeError, NodeContext},
    types::property::Phandle,
};

mod query;

pub trait IntoControlFlow<B, C = ()> {
    fn into_control_flow(self) -> ControlFlow<B, C>;
}

impl<B, C> IntoControlFlow<B, C> for ControlFlow<B, C> {
    fn into_control_flow(self) -> Self {
        self
    }
}

impl<T, E> IntoControlFlow<E, T> for Result<T, E> {
    fn into_control_flow(self) -> ControlFlow<E, T> {
        match self {
            Ok(value) => ControlFlow::Continue(value),
            Err(err) => ControlFlow::Break(err),
        }
    }
}

impl<'blob> NodeCursor<'_, 'blob> {
    pub fn visit_node_by_query<Q, F, T>(&self, query: Q, f: F) -> Result<Option<T>, ReadNodeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnOnce(NodeCursor<'_, 'blob>) -> T,
    {
        let query = query.as_ref();

        if query.is_absolute() && !self.is_root() {
            return self.root().visit_node_by_query(query, f);
        }

        visit_node_by_query(self.clone(), query, f)
    }

    pub fn visit_node_by_phandle<F, T>(
        &self,
        phandle: Phandle,
        f: F,
    ) -> Result<Option<T>, ReadNodeError>
    where
        F: FnOnce(NodeCursor<'_, 'blob>) -> T,
    {
        visit_node_by_phandle(self.clone(), phandle, &mut Some(f))
    }

    pub fn visit_all_nodes_by_query<Q, F>(&self, query: Q, mut f: F) -> Result<usize, ReadNodeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnMut(NodeCursor<'_, 'blob>),
    {
        let query = query.as_ref();
        if query.is_absolute() && !self.is_root() {
            return self.root().visit_all_nodes_by_query(query, f);
        }

        visit_all_nodes_by_query(self.clone(), query, &mut f)
    }

    pub fn try_visit_all_nodes_by_query<Q, F, R, T>(
        &self,
        query: Q,
        mut f: F,
    ) -> Result<Option<T>, ReadNodeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnMut(NodeCursor<'_, 'blob>) -> R,
        R: IntoControlFlow<T>,
    {
        let query = query.as_ref();
        if query.is_absolute() && !self.is_root() {
            return self.root().try_visit_all_nodes_by_query(query, f);
        }

        try_visit_all_nodes_by_query(self.clone(), query, &mut f)
    }
}

impl<'blob> NodeContext<'_, 'blob> {
    pub fn visit_node_by_query<Q, F, T>(
        &self,
        query: Q,
        f: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnOnce(NodeContext<'_, 'blob>) -> T,
    {
        self.cursor()
            .visit_node_by_query(query, |cursor| f(NodeContext::new(cursor)))
            .map_err(de::error::error_read_node)
    }

    pub fn visit_node_by_phandle<F, T>(
        &self,
        phandle: Phandle,
        f: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        F: FnOnce(NodeContext<'_, 'blob>) -> T,
    {
        self.cursor()
            .visit_node_by_phandle(phandle, |cursor| f(NodeContext::new(cursor)))
            .map_err(de::error::error_read_node)
    }

    pub fn visit_all_nodes_by_query<Q, F>(
        &self,
        query: Q,
        mut f: F,
    ) -> Result<usize, DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnMut(NodeContext<'_, 'blob>),
    {
        self.cursor()
            .visit_all_nodes_by_query(query, |cursor| f(NodeContext::new(cursor)))
            .map_err(de::error::error_read_node)
    }

    pub fn try_visit_all_nodes_by_query<Q, F, R, T>(
        &self,
        query: Q,
        mut f: F,
    ) -> Result<Option<T>, DeserializeError>
    where
        Q: AsRef<NodeQuery>,
        F: FnMut(NodeContext<'_, 'blob>) -> R,
        R: IntoControlFlow<T>,
    {
        self.cursor()
            .try_visit_all_nodes_by_query(query, |cursor| f(NodeContext::new(cursor)))
            .map_err(de::error::error_read_node)
    }
}

fn visit_node_by_query<'parent, 'blob, F, T>(
    mut node: NodeCursor<'parent, 'blob>,
    query: &NodeQuery,
    f: F,
) -> Result<Option<T>, ReadNodeError>
where
    F: FnOnce(NodeCursor<'_, 'blob>) -> T,
{
    let Some((component, query)) = query.split_first_component() else {
        return Ok(Some(f(node)));
    };

    while let Some(item) = node.read_item()? {
        if let Some(child) = item.into_node()
            && component.match_node(child.node())
        {
            return visit_node_by_query(child, query, f);
        }
    }

    Ok(None)
}

fn visit_node_by_phandle<'parent, 'blob, F, T>(
    mut node: NodeCursor<'parent, 'blob>,
    phandle: Phandle,
    f: &mut Option<F>,
) -> Result<Option<T>, ReadNodeError>
where
    F: FnOnce(NodeCursor<'_, 'blob>) -> T,
{
    while let Some(item) = node.read_item()? {
        match item {
            ItemCursor::Node(cursor) => {
                if let Some(value) = visit_node_by_phandle(cursor, phandle, f)? {
                    return Ok(Some(value));
                }
            }
            ItemCursor::Property(cursor) => {
                let property = cursor.property();
                if property.name() == "phandle" && property.value() == phandle.value().to_be_bytes()
                {
                    let f = f.take().unwrap();
                    return Ok(Some(f(cursor.node().clone())));
                }
            }
        }
    }

    Ok(None)
}

fn visit_all_nodes_by_query<'blob, F>(
    mut node: NodeCursor<'_, 'blob>,
    query: &NodeQuery,
    f: &mut F,
) -> Result<usize, ReadNodeError>
where
    F: FnMut(NodeCursor<'_, 'blob>),
{
    let Some((component, query)) = query.split_first_component() else {
        f(node);
        return Ok(1);
    };

    let mut visited = 0;
    while let Some(item) = node.read_item()? {
        if let Some(child) = item.into_node()
            && component.match_node(child.node())
        {
            visited += visit_all_nodes_by_query(child, query, f)?;
        }
    }

    Ok(visited)
}

fn try_visit_all_nodes_by_query<'blob, F, R, T>(
    mut node: NodeCursor<'_, 'blob>,
    query: &NodeQuery,
    f: &mut F,
) -> Result<Option<T>, ReadNodeError>
where
    F: FnMut(NodeCursor<'_, 'blob>) -> R,
    R: IntoControlFlow<T>,
{
    let Some((component, query)) = query.split_first_component() else {
        return Ok(f(node).into_control_flow().break_value());
    };

    while let Some(item) = node.read_item()? {
        if let Some(child) = item.into_node()
            && component.match_node(child.node())
            && let Some(value) = try_visit_all_nodes_by_query(child, query, f)?
        {
            return Ok(Some(value));
        }
    }

    Ok(None)
}
