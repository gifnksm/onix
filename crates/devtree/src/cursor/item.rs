use super::{NodeCursor, PropertyCursor};

#[derive(Debug, Clone)]
pub enum ItemCursor<'parent, 'blob> {
    Property(PropertyCursor<'parent, 'blob>),
    Node(NodeCursor<'parent, 'blob>),
}

impl<'parent, 'blob> ItemCursor<'parent, 'blob> {
    pub fn as_property(&self) -> Option<&PropertyCursor<'parent, 'blob>> {
        let Self::Property(cursor) = self else {
            return None;
        };
        Some(cursor)
    }

    pub fn as_node(&self) -> Option<&NodeCursor<'parent, 'blob>> {
        let Self::Node(cursor) = self else {
            return None;
        };
        Some(cursor)
    }

    pub fn into_property(self) -> Option<PropertyCursor<'parent, 'blob>> {
        let Self::Property(cursor) = self else {
            return None;
        };
        Some(cursor)
    }

    pub fn into_node(self) -> Option<NodeCursor<'parent, 'blob>> {
        let Self::Node(cursor) = self else {
            return None;
        };
        Some(cursor)
    }
}
