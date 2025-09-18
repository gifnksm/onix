use alloc::vec::Vec;

use devtree::{
    DeserializeNode, Devicetree,
    cursor::ReadNodeError,
    de::{DeserializeError, util},
    types::property::Reg,
};
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::Cpu;
use crate::{cpu::Cpuid, iter::IteratorExt as _};

#[derive(Debug, Snafu)]
#[snafu(module)]
pub enum DeserializeDevicetreeError {
    #[snafu(display("failed to read root node in devicetree"))]
    #[snafu(provide(ref, priority, Location => location))]
    ReadRootNode {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ReadNodeError,
    },
    #[snafu(display("failed to deserialize cpu node"))]
    #[snafu(provide(ref, priority, Location => location))]
    DeserializeCpuNode {
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
}

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

pub fn deserialize(dt: &Devicetree) -> Result<Vec<Cpu>, DeserializeDevicetreeError> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use deserialize_devicetree_error::*;

    let mut all_cpus = Vec::new();

    let root = dt.read_root_node().context(ReadRootNodeSnafu)?;
    root.try_visit_deserialize_all_nodes_by_query("/cpus/cpu", |cpu_node: CpuNode| {
        let CpuNode {
            reg,
            timebase_frequency,
        } = cpu_node;
        let reg = reg.into_iter().assume_one().context(InvalidRegCpuSnafu)?;
        all_cpus.push(Cpu {
            id: Cpuid::from_raw(reg.range().start),
            timer_frequency: timebase_frequency,
        });
        Ok(())
    })
    .context(DeserializeCpuNodeSnafu)?
    .map_or(Ok(()), Err)?;

    Ok(all_cpus)
}
