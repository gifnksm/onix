use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use crate::cpu::{self, Cpuid};

mod imp;
pub mod timer;
pub mod trap;

static BOOT_CPU_STATE: CpuState = CpuState::new();
static BOOT_COMPLETED: AtomicBool = AtomicBool::new(false);

cpu_local! {
    static CPU_STATE:  CpuState = CpuState::new();
}

pub fn init(boot_cpuid: Cpuid) {
    assert!(CPU_STATE.try_get().is_some());
    assert!(!is_enabled());
    assert_eq!(BOOT_CPU_STATE.disabled_depth(), 0);
    BOOT_COMPLETED.store(true, Ordering::Release);
    let cpu = cpu::current();
    assert_eq!(boot_cpuid, cpu.id());
}

#[track_caller]
pub fn disable() {
    imp::disable();
    assert_eq!(disabled_depth(), 0);
}

#[track_caller]
pub fn enable() {
    assert_eq!(disabled_depth(), 0);
    imp::enable();
}

#[track_caller]
pub fn wait() {
    imp::wait();
}

#[track_caller]
pub fn push_disabled() -> InterruptGuard {
    let state = imp::read_and_disable();
    let cpu_state = cpu_state();
    cpu_state.push_state(state);
    InterruptGuard {
        _not_send: PhantomData,
    }
}

#[track_caller]
pub fn disabled_depth() -> usize {
    cpu_state().disabled_depth()
}

pub unsafe fn remember_disabled() -> InterruptGuard {
    assert!(!is_enabled());
    let cpu_state = cpu_state();
    assert!(cpu_state.disabled_depth.load(Ordering::Relaxed) > 0);
    InterruptGuard {
        _not_send: PhantomData,
    }
}

pub fn is_enabled() -> bool {
    imp::is_enabled()
}

#[derive(Debug)]
pub struct InterruptGuard {
    _not_send: PhantomData<*mut ()>,
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        let cpu_state = cpu_state();
        if let Some(initial_state) = cpu_state.pop_state() {
            imp::restore(initial_state);
        }
    }
}

#[track_caller]
fn cpu_state() -> &'static CpuState {
    if !BOOT_COMPLETED.load(Ordering::Acquire) {
        return &BOOT_CPU_STATE;
    }
    CPU_STATE.get()
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

#[derive(Debug)]
pub struct SavedState {
    disabled_depth: usize,
    initial_state: imp::State,
}

pub fn save_state() -> SavedState {
    assert!(!is_enabled());
    let state = cpu_state();
    assert!(state.disabled_depth() > 0);
    SavedState {
        disabled_depth: state.disabled_depth(),
        initial_state: unsafe { *state.initial_state.get() },
    }
}

impl SavedState {
    pub fn restore(self) {
        assert!(!is_enabled());
        let state = cpu_state();
        state
            .disabled_depth
            .store(self.disabled_depth, Ordering::Release);
        unsafe {
            *state.initial_state.get() = self.initial_state;
        }
    }
}
