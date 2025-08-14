use alloc::{slice, vec::Vec};
use core::{fmt, iter::Peekable};

use devicetree::{
    common::property::{ParsePropertyValueError, Reg},
    parsed::{Devicetree, node::Node},
};
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;
use spin::Once;

cpu_local! {
    static CURRENT_CPU: Once<&'static Cpu> = Once::new();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Cpuid(usize);

impl fmt::Display for Cpuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Cpuid {
    pub fn value(self) -> usize {
        self.0
    }

    pub fn from_raw(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct Cpu {
    id: Cpuid,
    timer_frequency: u64,
}

unsafe impl Send for Cpu {}
unsafe impl Sync for Cpu {}

impl Cpu {
    pub fn id(&self) -> Cpuid {
        self.id
    }

    pub fn timer_frequency(&self) -> u64 {
        self.timer_frequency
    }

    pub fn is_current(&self) -> bool {
        try_current().is_some_and(|cpu| cpu.id() == self.id)
    }
}

static ALL_CPUS: Once<Vec<Cpu>> = Once::new();

#[derive(Debug, Snafu)]
pub enum CpuInitError {
    #[snafu(display("missing `cpus` node in devicetree"))]
    MissingCpusNode {
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("failed to get property in `cpus` node: {source}"))]
    PropertyInCpus {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PropertyError,
    },
    #[snafu(display("failed to get property in `cpu` node: {source}"))]
    PropertyInCpu {
        #[snafu(implicit)]
        location: Location,
        #[snafu(source)]
        source: PropertyError,
    },
}

#[derive(Debug, Snafu)]
pub enum PropertyError {
    #[snafu(display("missing property `{name}`"))]
    Missing {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
    },
    #[snafu(display("invalid value for property `{name}`: {source}"))]
    Parse {
        name: &'static str,
        #[snafu(implicit)]
        location: Location,
        #[snafu(implicit)]
        source: ParsePropertyValueError,
    },
    #[snafu(display(
        "invalid value length for property `{name}`. expected: {expected}, actual: {actual}"
    ))]
    InvalidValueLength {
        name: &'static str,
        expected: usize,
        actual: usize,
        #[snafu(implicit)]
        location: Location,
    },
}

fn get_u32_prop(node: &Node, name: &'static str) -> Result<u32, PropertyError> {
    node.properties()
        .find(|p| p.name() == name)
        .context(MissingSnafu { name })?
        .value_as_u32()
        .context(ParseSnafu { name })
}

fn get_u32_or_u64_prop(node: &Node, name: &'static str) -> Result<u64, PropertyError> {
    node.properties()
        .find(|p| p.name() == name)
        .context(MissingSnafu { name })?
        .value_as_u32_or_u64()
        .context(ParseSnafu { name })
}

fn get_reg(node: &Node, address_cells: usize, size_cells: usize) -> Result<Reg, PropertyError> {
    let name = "reg";
    let mut regs = node
        .properties()
        .find(|p| p.name() == name)
        .context(MissingSnafu { name })?
        .value_as_reg(address_cells, size_cells)
        .context(ParseSnafu { name })?;
    ensure!(
        regs.len() == 1,
        InvalidValueLengthSnafu {
            name,
            expected: 1_usize,
            actual: regs.len(),
        }
    );
    Ok(regs.next().unwrap())
}

pub fn init(dtree: &Devicetree) -> Result<(), CpuInitError> {
    let cpus_node = dtree
        .root_node()
        .children()
        .find(|n| n.name() == "cpus")
        .context(MissingCpusNodeSnafu)?;
    let address_cells =
        usize::cast_from(get_u32_prop(&cpus_node, "#address-cells").context(PropertyInCpusSnafu)?);
    let size_cells =
        usize::cast_from(get_u32_prop(&cpus_node, "#size-cells").context(PropertyInCpusSnafu)?);

    let mut cpus = Vec::new();

    for cpu_node in cpus_node.children().filter(|node| node.name() == "cpu") {
        let reg = get_reg(&cpu_node, address_cells, size_cells).context(PropertyInCpuSnafu)?;
        let id = Cpuid(reg.address);
        let timer_frequency = get_u32_or_u64_prop(&cpu_node, "timebase-frequency")
            .or_else(|_| get_u32_or_u64_prop(&cpus_node, "timebase-frequency"))
            .context(PropertyInCpusSnafu)?;
        assert!(
            timer_frequency > 0,
            "timer frequency must be greater than 0"
        );

        cpus.push(Cpu {
            id,
            timer_frequency,
        });
    }

    // sort cpus by cpuid
    cpus.sort_by(|a, b| Cpuid::cmp(&a.id, &b.id));

    ALL_CPUS.call_once(|| cpus);

    Ok(())
}

pub fn set_current_cpuid(cpuid: Cpuid) {
    let cpu = ALL_CPUS
        .get()
        .unwrap()
        .iter()
        .find(|cpu| cpu.id() == cpuid)
        .unwrap();
    CURRENT_CPU.get().call_once(|| cpu);
}

#[track_caller]
pub fn try_current() -> Option<&'static Cpu> {
    CURRENT_CPU.try_get()?.get().copied()
}

#[track_caller]
pub fn current() -> &'static Cpu {
    try_current().unwrap()
}

#[track_caller]
pub fn get_all() -> &'static [Cpu] {
    ALL_CPUS.get().unwrap()
}

#[derive(Clone, Copy)]
pub struct CpuMask {
    pub mask: usize,
    pub base: usize,
}

impl fmt::Debug for CpuMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(
                (0..usize::BITS)
                    .filter(|&i| self.mask & (1 << i) != 0)
                    .map(|i| Cpuid(self.base + usize::cast_from(i))),
            )
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct RemoteCpuMaskIter {
    current_cpuid: Cpuid,
    cpus: Peekable<slice::Iter<'static, Cpu>>,
}

impl Iterator for RemoteCpuMaskIter {
    type Item = CpuMask;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let base_cpu = self.cpus.next()?;
            if base_cpu.id() == self.current_cpuid {
                continue;
            }

            let base = base_cpu.id().value();
            let mut mask = 1;
            while let Some(cpu) = self
                .cpus
                .next_if(|cpu| cpu.id().value() - base < usize::cast_from(usize::BITS))
            {
                if cpu.id() != self.current_cpuid {
                    mask |= 1 << (cpu.id().value() - base);
                }
            }

            return Some(CpuMask { mask, base });
        }
    }
}

pub fn remote_cpu_masks() -> RemoteCpuMaskIter {
    let current_cpuid = current().id();
    let cpus = ALL_CPUS.get().unwrap().iter().peekable();
    RemoteCpuMaskIter {
        current_cpuid,
        cpus,
    }
}
