use alloc::vec::Vec;
use core::{arch::asm, ptr};

use devicetree::{
    common::property::{ParsePropertyValueError, Reg},
    parsed::{Devicetree, node::Node},
};
use platform_cast::CastFrom as _;
use snafu::{OptionExt as _, ResultExt as _, Snafu, ensure};
use snafu_utils::Location;
use spin::Once;

use crate::memory::{
    kernel_space::{self, KernelPageTable},
    page_table::sv39::{MapPageFlags, PageTableError},
};

#[derive(Debug)]
pub struct Cpu {
    id: usize,
    index: usize,
    stack_top: *mut u8,
}

unsafe impl Send for Cpu {}
unsafe impl Sync for Cpu {}

impl Cpu {
    pub fn id(&self) -> usize {
        self.id
    }

    pub fn stack_top(&self) -> *mut u8 {
        self.stack_top
    }
}

static CPUS: Once<Vec<Cpu>> = Once::new();

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

    for (index, cpu_node) in cpus_node
        .children()
        .filter(|node| node.name() == "cpu")
        .enumerate()
    {
        let reg = get_reg(&cpu_node, address_cells, size_cells).context(PropertyInCpuSnafu)?;
        let id = reg.address;
        let stack_range = kernel_space::kernel_stack_ranges(index);

        cpus.push(Cpu {
            id,
            index,
            stack_top: ptr::with_exposed_provenance_mut(stack_range.end),
        });
    }

    CPUS.call_once(|| cpus);

    Ok(())
}

pub fn update_kernel_page_table(kpgtbl: &mut KernelPageTable) -> Result<(), PageTableError> {
    let cpus = CPUS.get().unwrap();
    for cpu in cpus {
        let stack_range = kernel_space::kernel_stack_ranges(cpu.index);
        kpgtbl.allocate_virt_addr_range(stack_range, MapPageFlags::RW)?;
    }
    Ok(())
}

pub fn set_current_cpuid(cpuid: usize) {
    let cpus = CPUS.get().unwrap();
    let cpu = cpus.iter().find(|cpu| cpu.id == cpuid).unwrap();
    unsafe {
        asm!("mv tp, {}", in(reg) cpu.index);
    }
}

pub fn current_cpu() -> &'static Cpu {
    let cpus = CPUS.get().unwrap();
    let index: usize;
    unsafe {
        asm!("mv {}, tp", out(reg) index);
    }
    &cpus[index]
}
