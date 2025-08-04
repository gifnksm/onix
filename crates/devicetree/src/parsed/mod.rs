use alloc::sync::Arc;
use core::fmt;

use self::node::{Node, NodeInner};

pub mod node;

pub struct Devicetree {
    root_node: Arc<NodeInner>,
    string_block: Arc<[u8]>,
}

impl Devicetree {
    pub(crate) fn new(root: Arc<NodeInner>, string_block: Arc<[u8]>) -> Self {
        Self {
            root_node: root,
            string_block,
        }
    }

    #[must_use]
    pub fn root_node(&self) -> Node {
        Node {
            inner: Arc::clone(&self.root_node),
            string_block: Arc::clone(&self.string_block),
        }
    }
}

impl fmt::Debug for Devicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Devicetree")
            .field("root", &self.root_node())
            .finish()
    }
}
