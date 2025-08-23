use alloc::{borrow::ToOwned, boxed::Box, string::String, vec::Vec};
use core::fmt;

use devicetree::parsed::{
    Devicetree,
    node::{Node, PropertyError},
};
use either::Either;
use snafu::{OptionExt as _, ResultExt as _, Snafu};
use snafu_utils::Location;

use super::{Cpu, Cpuid};

struct NodeNameFormat<'a> {
    name: &'a String,
    address: &'a Option<String>,
}

impl fmt::Display for NodeNameFormat<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(address) = &self.address {
            write!(f, "{}@{}", self.name, address)
        } else {
            write!(f, "{}", self.name)
        }
    }
}

#[derive(Debug, Snafu)]
pub enum ParseDevicetreeError {
    #[snafu(display("missing `cpus` node in devicetree"))]
    MissingCpusNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to parse `{}` node: {source}", NodeNameFormat { name, address }))]
    ParseCpuNode {
        name: String,
        address: Option<String>,
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ParseCpuError,
    },
}

pub(super) fn parse(dtree: &Devicetree) -> Result<Vec<Cpu>, Box<ParseDevicetreeError>> {
    let cpus_node = dtree
        .find_node_by_path("/cpus")
        .context(MissingCpusNodeSnafu)?;
    let all_cpus = cpus_node
        .children()
        .filter(|node| node.name() == "cpu")
        .map(|cpu_node| {
            Cpu::parse(&cpu_node).with_context(|_| ParseCpuNodeSnafu {
                name: cpu_node.name().to_owned(),
                address: cpu_node.address().map(ToOwned::to_owned),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(all_cpus)
}

#[derive(Debug, Snafu)]
pub enum ParseCpuError {
    #[snafu(display("{source}"))]
    Property {
        #[snafu(source)]
        source: PropertyError,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("`reg` property contains no addresses"))]
    NoAddressInReg {
        #[snafu(implicit)]
        location: Location,
    },

    #[snafu(display("`reg` property contains too many addresses"))]
    TooManyAddressesReg {
        #[snafu(implicit)]
        location: Location,
    },
}

impl Cpu {
    fn parse(cpu_node: &Node) -> Result<Self, ParseCpuError> {
        let reg = cpu_node
            .reg()
            .context(PropertySnafu)?
            .assume_one()
            .context(NoAddressInRegSnafu)?;

        let id = Cpuid(reg.address);
        let timer_frequency = cpu_node
            .fetch_common_property_as::<Either<u32, u64>>("timebase-frequency")
            .context(PropertySnafu)?
            .map_left(u64::from)
            .into_inner();
        assert!(
            timer_frequency > 0,
            "timer frequency must be greater than 0"
        );

        Ok(Self {
            id,
            timer_frequency,
        })
    }
}
