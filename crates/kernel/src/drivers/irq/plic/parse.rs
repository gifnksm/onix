use alloc::{
    borrow::ToOwned, boxed::Box, collections::btree_map::BTreeMap, string::String, sync::Arc,
    vec::Vec,
};
use core::fmt;

use devicetree::parsed::{
    Devicetree,
    node::{Interrupt, Node, PropertyError},
};
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;

use super::{Plic, PlicContext};
use crate::{cpu::Cpuid, drivers::irq::plic::PlicMmio, sync::spinlock::SpinMutex};

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
    #[snafu(display("missing `soc` node in devicetree"))]
    MissingSocNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to parse `{name}` node: {source}", name = NodeNameFormat { name, address }))]
    ParsePlicNode {
        name: String,
        address: Option<String>,
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: ParsePlicNodeError,
    },
}

#[derive(Debug, Snafu)]
pub enum ParsePlicNodeError {
    #[snafu(display("failed to get property in `plic` node: {source}"))]
    PropertyInNode {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PropertyError,
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
    #[snafu(display("invalid specifier len"))]
    InvalidSpecifierLen {
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn parse(dtree: &Devicetree) -> Result<Vec<Arc<Plic>>, Box<ParseDevicetreeError>> {
    let root = dtree.root_node();
    let soc_node = root
        .children()
        .find(|node| node.name() == "soc")
        .context(MissingSocNodeSnafu)?;
    let plic_devices = soc_node
        .children()
        .filter(is_plic_node)
        .map(|plic_node| {
            Plic::parse(&plic_node)
                .with_context(|_e| ParsePlicNodeSnafu {
                    name: plic_node.name().to_owned(),
                    address: plic_node.address().map(ToOwned::to_owned),
                })
                .map(Arc::new)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(plic_devices)
}

fn is_plic_node(node: &Node) -> bool {
    node.name() == "plic" && node.is_compatible_to("riscv,plic0")
}

fn parse_context_map(plic_node: &Node) -> Result<BTreeMap<Cpuid, PlicContext>, ParsePlicNodeError> {
    let interrupts = plic_node.interrupts().context(PropertyInNodeSnafu)?;
    let contexts = interrupts
        .into_iter()
        .enumerate()
        .filter_map(|(id, interrupt)| PlicContext::from_dtree(id, &interrupt).transpose())
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(contexts)
}

impl Plic {
    fn parse(plic_node: &Node) -> Result<Self, ParsePlicNodeError> {
        let path = plic_node.path();
        let mut reg_iter = plic_node.reg().context(PropertyInNodeSnafu)?;
        let reg = reg_iter.next().context(NoAddressInRegSnafu)?;
        ensure!(reg_iter.next().is_none(), TooManyAddressesRegSnafu);
        let ndev = usize::cast_from(
            plic_node
                .fetch_property_as::<u32>("riscv,ndev")
                .context(PropertyInNodeSnafu)?,
        );
        let mmio = SpinMutex::new(PlicMmio {
            base_addr: reg.address,
            size: reg.size,
            ndev,
        });
        let context_map = parse_context_map(plic_node)?;
        Ok(Self {
            path,
            mmio,
            context_map,
            callbacks: SpinMutex::new(BTreeMap::new()),
        })
    }
}

impl PlicContext {
    fn from_dtree(
        id: usize,
        interrupt: &Interrupt,
    ) -> Result<Option<(Cpuid, Self)>, ParsePlicNodeError> {
        let Some(cpu_node) = interrupt.interrupt_parent.parent() else {
            return Ok(None);
        };
        if cpu_node.name() != "cpu" {
            return Ok(None);
        }
        let mut reg_iter = cpu_node.reg().context(PropertyInNodeSnafu)?;
        assert_eq!(reg_iter.len(), 1);
        let reg = reg_iter.next().unwrap();
        let cpuid = Cpuid::from_raw(reg.address);

        ensure!(interrupt.specifier.len() == 1, InvalidSpecifierLenSnafu);
        let specifier = interrupt.specifier[0];
        // 9 means supervisor interrupt
        if specifier != 9 {
            return Ok(None);
        }
        Ok(Some((cpuid, Self { id })))
    }
}
