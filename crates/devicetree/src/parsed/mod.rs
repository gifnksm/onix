use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use core::fmt;

use self::node::{Node, NodeInner};
use crate::{common::Phandle, flattened::layout::ReserveEntry};

pub mod node;

pub struct Devicetree {
    inner: Arc<DevicetreeInner>,
}

pub(crate) struct DevicetreeInner {
    root_node: Arc<NodeInner>,
    string_block: Arc<[u8]>,
    phandle_map: Arc<BTreeMap<Phandle, Arc<NodeInner>>>,
    mem_rsvmap: Vec<ReserveEntry>,
}

impl Devicetree {
    pub(crate) fn new(
        root_node: Arc<NodeInner>,
        string_block: Arc<[u8]>,
        phandle_map: Arc<BTreeMap<Phandle, Arc<NodeInner>>>,
        mem_rsvmap: Vec<ReserveEntry>,
    ) -> Self {
        Self {
            inner: Arc::new(DevicetreeInner {
                root_node,
                string_block,
                phandle_map,
                mem_rsvmap,
            }),
        }
    }

    #[must_use]
    pub fn root_node(&self) -> Node {
        Node {
            inner: Arc::clone(&self.inner.root_node),
            tree: Arc::clone(&self.inner),
        }
    }

    #[must_use]
    pub fn mem_rsvmap(&self) -> &[ReserveEntry] {
        &self.inner.mem_rsvmap
    }

    #[must_use]
    pub fn get_node_by_phandle(&self, phandle: Phandle) -> Option<Node> {
        self.inner.phandle_map.get(&phandle).map(|inner| Node {
            inner: Arc::clone(inner),
            tree: Arc::clone(&self.inner),
        })
    }

    #[must_use]
    pub fn find_node_by_path(&self, path: &str) -> Option<Node> {
        let mut current = self.root_node();
        for component in path.split('/').skip(1) {
            if component.is_empty() {
                continue;
            }
            if component == ".." {
                current = current.parent()?;
                continue;
            }
            if component == "." {
                continue;
            }
            let (name, unit_address) = match component.split_once('@') {
                Some((name, address)) => (name, Some(address)),
                None => (component, None),
            };
            current = current
                .children()
                .find(|n| n.name() == name && n.address() == unit_address)?;
        }
        Some(current)
    }
}

impl fmt::Debug for Devicetree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Devicetree")
            .field("root", &self.root_node())
            .finish()
    }
}
