use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
    de::util,
    tree_cursor::{TreeCursor as _, TreeIterator as _},
    types::{
        ByteStr,
        node::{InterruptGeneratingDevice, NodePath},
        property::Reg,
    },
};
use snafu::{OptionExt as _, ResultExt as _};

use super::{Plic, PlicContext};
use crate::{
    cpu::Cpuid, drivers::irq::plic::PlicMmio, error::GenericError, iter::IteratorExt as _,
    sync::spinlock::SpinMutex,
};

#[derive(Debug, DeserializeNode)]
struct PlicNode<'blob> {
    #[devtree(node)]
    path: NodePath,
    #[devtree(node)]
    device: InterruptGeneratingDevice<'blob>,
    #[devtree(property(
        name = "riscv,ndev",
        deserialize_with = util::deserialize_property_as_usize_via_u32,
    ))]
    ndev: usize,
    #[devtree(property)]
    reg: Reg<'blob>,
}

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Arc<Plic>>, GenericError> {
    let mut plic_devices = Vec::new();
    let mut cursor = dt
        .tree_cursor()
        .whatever_context("failed to create tree cursor")?;
    let iter = cursor
        .read_descendant_nodes_by_glob("/soc/plic")
        .deserialize_node::<PlicNode>();
    for plic_node in iter {
        let plic_node =
            plic_node.whatever_context("failed to deserialize plic node in devicetree")?;
        let plic_device = Plic::from_node(dt, plic_node)?;
        plic_devices.push(plic_device);
    }
    Ok(plic_devices)
}

impl Plic {
    fn from_node(dt: &Devicetree, plic_node: PlicNode) -> Result<Arc<Self>, GenericError> {
        let PlicNode {
            path,
            device,
            ndev,
            reg,
        } = plic_node;
        let reg = reg
            .into_iter()
            .assume_one()
            .whatever_context("invalid 'reg' entries in plic node")?;
        let range = reg.range();
        let context_map = deserialize_context_map(dt, &device)
            .whatever_context("failed to deserialize devicetree plic node")?;
        let plic = Arc::new(Self {
            path: path.0,
            mmio: SpinMutex::new(PlicMmio {
                base_addr: range.start,
                size: range.len(),
                ndev,
            }),
            context_map,
            callbacks: SpinMutex::new(BTreeMap::new()),
        });
        Ok(plic)
    }
}

fn deserialize_context_map(
    dt: &Devicetree,
    device: &InterruptGeneratingDevice<'_>,
) -> Result<BTreeMap<Cpuid, PlicContext>, GenericError> {
    let mut map = BTreeMap::new();
    for (id, interrupt) in device.interrupts().iter().enumerate() {
        let specifier = interrupt
            .specifier()
            .into_iter()
            .assume_one()
            .whatever_context("invalid interrupt specifier length")?;
        // 9 means supervisor interrupt
        if specifier != 9 {
            continue;
        }

        let Some(cpuid) = deserialize_cpuid(dt, interrupt.parent_path())
            .whatever_context("failed to deserialize devicetree cpu node")?
        else {
            continue;
        };
        map.insert(cpuid, PlicContext { id });
    }
    Ok(map)
}

#[derive(DeserializeNode)]
struct CpuNode<'blob> {
    #[devtree(property)]
    reg: Reg<'blob>,
}

fn deserialize_cpuid(dt: &Devicetree, intc_path: &ByteStr) -> Result<Option<Cpuid>, GenericError> {
    let mut cursor = dt
        .tree_cursor()
        .whatever_context("failed to create tree cursor")?;
    let Some(_intc_node) = cursor
        .read_node_by_path(intc_path)
        .whatever_context("failed to read devicetree")?
    else {
        return Ok(None);
    };
    let Some(parent) = cursor.read_parent() else {
        return Ok(None);
    };
    if parent.node().name() != "cpu" {
        return Ok(None);
    }
    let CpuNode { reg } = parent
        .deserialize_node()
        .whatever_context("failed to deserialize devicetree cpu node")?;
    let reg = reg
        .into_iter()
        .assume_one()
        .whatever_context("invalid 'reg' entries in cpu node")?;
    let cpuid = Cpuid::from_raw(reg.range().start);
    Ok(Some(cpuid))
}
