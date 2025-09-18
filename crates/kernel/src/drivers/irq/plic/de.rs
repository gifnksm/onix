use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};

use devtree::{
    DeserializeNode, Devicetree,
    cursor::{NodeCursor, ReadNodeError},
    de::{DeserializeError, util},
    types::{
        node::{InterruptGeneratingDevice, NodePath},
        property::Reg,
    },
};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::{Plic, PlicContext};
use crate::{
    cpu::Cpuid, drivers::irq::plic::PlicMmio, iter::IteratorExt as _, sync::spinlock::SpinMutex,
};

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum DeserializeDevicetreeError {
    #[snafu(display("failed to read devicetree node"))]
    #[snafu(provide(ref, priority, Location => location))]
    Read {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ReadNodeError,
    },
    #[snafu(display("failed to deserialize plic node"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializePlic {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: DeserializeError,
    },
    #[snafu(display("invalid 'reg' entries in plic node"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidRegPlic {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to deserialize cpu node"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeCpu {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: DeserializeError,
    },
    #[snafu(display("invalid 'reg' entries in cpu node"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidRegCpu {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid specifier len"))]
    #[snafu(provide(ref, priority, Location => location))]
    InvalidSpecifierLen {
        #[snafu(implicit)]
        location: Location,
    },
}

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

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Arc<Plic>>, DeserializeDevicetreeError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::deserialize_devicetree_error::*;

    let mut plic_devices = Vec::new();
    let root_node = dt.read_root_node().context(ReadSnafu)?;
    root_node
        .try_visit_deserialize_all_nodes_by_query("/soc/plic", |plic_node| {
            plic_devices.push(Plic::from_node(&root_node, plic_node)?);
            Ok(())
        })
        .context(DeserializePlicSnafu)?
        .map_or(Ok(()), Err)?;
    Ok(plic_devices)
}

impl Plic {
    fn from_node(
        root_node: &NodeCursor<'_, '_>,
        plic_node: PlicNode,
    ) -> Result<Arc<Self>, DeserializeDevicetreeError> {
        #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
        use self::deserialize_devicetree_error::*;

        let PlicNode {
            path,
            device,
            ndev,
            reg,
        } = plic_node;
        let reg = reg.into_iter().assume_one().context(InvalidRegPlicSnafu)?;
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
) -> Result<BTreeMap<Cpuid, PlicContext>, DeserializeDevicetreeError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::deserialize_devicetree_error::*;

    let mut map = BTreeMap::new();
    for (id, interrupt) in device.interrupts().iter().enumerate() {
        let specifier = interrupt
            .specifier()
            .into_iter()
            .assume_one()
            .context(InvalidSpecifierLenSnafu)?;
        // 9 means supervisor interrupt
        if specifier != 9 {
            continue;
        }

        root_node
            .visit_node_by_query(
                interrupt.parent_path(),
                |parent| -> Result<(), DeserializeDevicetreeError> {
                    let Some(cpu_node) = parent.parent() else {
                        return Ok(());
                    };
                    if cpu_node.node().name() != "cpu" {
                        return Ok(());
                    }
                    let reg = cpu_node
                        .deserialize_property::<Reg<'_>>("reg")
                        .context(DeserializeCpuSnafu)?;
                    let reg = reg.into_iter().assume_one().context(InvalidRegCpuSnafu)?;
                    let cpuid = Cpuid::from_raw(reg.range().start);
                    map.insert(cpuid, PlicContext { id });
                    Ok(())
                },
            )
            .context(ReadSnafu)?;
    }
    Ok(map)
}
