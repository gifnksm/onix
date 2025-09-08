use alloc::{boxed::Box, slice, vec::Vec};
use core::{fmt, iter::Peekable};

use devicetree::parsed::Devicetree;
use platform_cast::CastFrom as _;
use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;
use spin::Once;

use self::parse::ParseDevicetreeError;

mod parse;

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
#[snafu(module)]
pub enum CpuInitError {
    #[snafu(display("failed to parse devicetree"))]
    #[snafu(provide(ref, priority, Location => location))]
    ParseDevicetree {
        #[snafu(source)]
        source: Box<ParseDevicetreeError>,
        #[snafu(implicit)]
        location: Location,
    },
}

pub fn init(dtree: &Devicetree) -> Result<(), Box<CpuInitError>> {
    #[cfg_attr(not(test), expect(clippy::wildcard_imports))]
    use self::cpu_init_error::*;

    let mut all_cpus = parse::parse(dtree).context(ParseDevicetreeSnafu)?;

    // sort cpus by cpuid
    all_cpus.sort_by(|a, b| Cpuid::cmp(&a.id, &b.id));

    ALL_CPUS.call_once(|| all_cpus);

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
