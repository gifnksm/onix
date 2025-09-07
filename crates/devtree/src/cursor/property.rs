use core::fmt;

use super::NodeCursor;
use crate::blob::Property;

#[derive(Clone)]
pub struct PropertyCursor<'node, 'blob> {
    property: Property<'blob>,
    node: &'node NodeCursor<'node, 'blob>,
}

impl<'node, 'blob> PropertyCursor<'node, 'blob> {
    pub(crate) fn new(
        node: &'node NodeCursor<'node, 'blob>,
        name_offset: usize,
        value: &'blob [u8],
    ) -> Self {
        let property = Property::new(node.devicetree(), name_offset, value);
        Self { property, node }
    }

    #[must_use]
    pub fn property(&self) -> &Property<'blob> {
        &self.property
    }

    #[must_use]
    pub fn node(&self) -> &'node NodeCursor<'node, 'blob> {
        self.node
    }
}

impl fmt::Debug for PropertyCursor<'_, '_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.property, f)
    }
}
