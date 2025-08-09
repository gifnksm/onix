use alloc::vec::Vec;
use core::{
    cell::UnsafeCell,
    iter,
    marker::PhantomData,
    sync::atomic::{AtomicUsize, Ordering},
};

use spin::Once;

use crate::cpu::{self, Cpuid};

mod imp;
pub mod timer;
pub mod trap;

static BOOT_CPU_STATE: CpuState = CpuState::new();
static CPU_STATE: Once<Vec<CpuState>> = Once::new();

pub fn init(boot_cpuid: Cpuid) {
    assert!(!is_enabled());
    assert_eq!(BOOT_CPU_STATE.disabled_depth(), 0);
    let cpu = cpu::current();
    assert_eq!(boot_cpuid, cpu.id());
    CPU_STATE.call_once(|| iter::repeat_with(CpuState::new).take(cpu::len()).collect());
}

pub fn disable() -> Guard {
    let state = imp::read_and_disable();
    let cpu_state = cpu_state();
    cpu_state.push_state(state);
    Guard {
        _not_send: PhantomData,
    }
}

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

#[derive(Debug)]
pub struct Guard {
    _not_send: PhantomData<*mut ()>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let cpu_state = cpu_state();
        if let Some(initial_state) = cpu_state.pop_state() {
            imp::restore(initial_state);
        }
    }
}

fn cpu_state() -> &'static CpuState {
    if !CPU_STATE.is_completed() {
        return &BOOT_CPU_STATE;
    }
    &CPU_STATE.get().unwrap()[cpu::current_index()]
}

#[derive(Debug)]
struct CpuState {
    disabled_depth: AtomicUsize,
    initial_state: UnsafeCell<imp::State>,
}

unsafe impl Sync for CpuState {}

impl CpuState {
    const fn new() -> Self {
        Self {
            disabled_depth: AtomicUsize::new(0),
            initial_state: UnsafeCell::new(imp::State::new()),
        }
    }

    fn disabled_depth(&self) -> usize {
        assert!(!is_enabled());
        self.disabled_depth.load(Ordering::Relaxed)
    }

    fn push_state(&self, state: imp::State) {
        assert!(!is_enabled());
        let depth = self.disabled_depth.fetch_add(1, Ordering::Acquire);
        if depth == 0 {
            unsafe {
                *self.initial_state.get() = state;
            }
        }
    }

    fn pop_state(&self) -> Option<imp::State> {
        assert!(!is_enabled());
        let depth = self.disabled_depth.fetch_sub(1, Ordering::Release);
        if depth == 1 {
            unsafe { Some(*self.initial_state.get()) }
        } else {
            None
        }
    }
}
