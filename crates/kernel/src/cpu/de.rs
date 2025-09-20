use alloc::vec::Vec;

use devtree::{DeserializeNode, Devicetree, de::util, types::property::Reg};
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
        deserialize_with = |pctx| util::deserialize_u64_or_u32_property(pctx),
    ))]
    timebase_frequency: u64,
}

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Cpu>, GenericError> {
    let mut all_cpus = Vec::new();

    let root = dt
        .read_root_node()
        .whatever_context("failed to read root node in devicetree")?;
    root.try_visit_deserialize_all_nodes_by_query("/cpus/cpu", |cpu_node: CpuNode| {
        let CpuNode {
            reg,
            timebase_frequency,
        } = cpu_node;
        let reg = reg
            .into_iter()
            .assume_one()
            .whatever_context("invalid 'reg' entries in cpu node")?;
        all_cpus.push(Cpu {
            id: Cpuid::from_raw(reg.range().start),
            timer_frequency: timebase_frequency,
        });
        Ok(())
    })
    .whatever_context("failed to deserialize cpu node")?
    .map_or(Ok(()), Err)?;

    Ok(all_cpus)
}
