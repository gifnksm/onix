use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
    cursor::NodeCursor,
    de::util,
    types::{
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
        deserialize_with = util::deserialize_usize_property_from_u32,
    ))]
    ndev: usize,
    #[devtree(property)]
    reg: Reg<'blob>,
}

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Arc<Plic>>, GenericError> {
    let mut plic_devices = Vec::new();
    let root_node = dt
        .read_root_node()
        .whatever_context("failed to read devicetree root node")?;
    root_node
        .try_visit_deserialize_all_nodes_by_query("/soc/plic", |plic_node| {
            plic_devices.push(Plic::from_node(&root_node, plic_node)?);
            Ok(())
        })
        .whatever_context("failed to deserialize devicetree plic node")?
        .map_or(Ok(()), Err)?;
    Ok(plic_devices)
}

impl Plic {
    fn from_node(
        root_node: &NodeCursor<'_, '_>,
        plic_node: PlicNode,
    ) -> Result<Arc<Self>, GenericError> {
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
        let context_map = deserialize_context_map(root_node, &device)?;
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
    root_node: &NodeCursor<'_, '_>,
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

        root_node
            .visit_node_by_query(
                interrupt.parent_path(),
                |parent| -> Result<(), GenericError> {
                    let Some(cpu_node) = parent.parent() else {
                        return Ok(());
                    };
                    if cpu_node.node().name() != "cpu" {
                        return Ok(());
                    }
                    let reg = cpu_node
                        .deserialize_property::<Reg<'_>>("reg")
                        .whatever_context("failed to deserialize devicetree cpu node")?;
                    let reg = reg
                        .into_iter()
                        .assume_one()
                        .whatever_context("invalid 'reg' entries in cpu node")?;
                    let cpuid = Cpuid::from_raw(reg.range().start);
                    map.insert(cpuid, PlicContext { id });
                    Ok(())
                },
            )
            .whatever_context("failed to read devicetree")?;
    }
    Ok(map)
}
