use alloc::vec::Vec;

use devtree::{
    DeserializeNode, Devicetree,
    de::util,
    tree_cursor::{TreeCursor as _, TreeIterator as _},
    types::property::Reg,
};
use snafu::{OptionExt as _, ResultExt as _};

use super::Cpu;
use crate::{cpu::Cpuid, error::GenericError, iter::IteratorExt as _};

#[derive(Debug, DeserializeNode)]
struct CpuNode<'blob> {
    #[devtree(property)]
    reg: Reg<'blob>,
    #[devtree(property(
        name = "timebase-frequency",
        fallback = "parent",
        default,
        deserialize_with = util::deserialize_u64_or_u32_property,
    ))]
    timebase_frequency: u64,
}

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Cpu>, GenericError> {
    let mut all_cpus = Vec::new();
    let mut cursor = dt.tree_cursor();
    let iter = cursor
        .read_descendant_nodes_by_glob("/cpus/cpu")
        .deserialize_node::<CpuNode>();
    for cpu_node in iter {
        let CpuNode {
            reg,
            timebase_frequency,
        } = cpu_node.whatever_context("failed to deserialize cpu node in devicetree")?;
        let reg = reg
            .into_iter()
            .assume_one()
            .whatever_context("invalid 'reg' entries in cpu node")?;
        let cpu = Cpu {
            id: Cpuid::from_raw(reg.range().start),
            timer_frequency: timebase_frequency,
        };
        all_cpus.push(cpu);
    }
    Ok(all_cpus)
}
