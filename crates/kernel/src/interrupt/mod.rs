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
    assert_eq!(BOOT_CPU_STATE.interrupt_disabled_depth(), 0);
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
    cpu_state.push_interrupt_state(state);
    InterruptGuard {
        _not_send: PhantomData,
    }
}

#[track_caller]
pub fn disabled_depth() -> usize {
    cpu_state().interrupt_disabled_depth()
}

pub unsafe fn remember_disabled() -> InterruptGuard {
    assert!(!is_enabled());
    let cpu_state = cpu_state();
    assert!(cpu_state.interrupt_disabled_depth.load(Ordering::Relaxed) > 0);
    InterruptGuard {
        _not_send: PhantomData,
    }
}

#[track_caller]
pub fn is_enabled() -> bool {
    imp::is_enabled()
}

#[track_caller]
pub fn in_interrupt_handler() -> bool {
    cpu_state().in_interrupt_handler()
}

#[derive(Debug)]
pub struct InterruptGuard {
    _not_send: PhantomData<*mut ()>,
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        let cpu_state = cpu_state();
        if let Some(initial_state) = cpu_state.pop_interrupt_state() {
            imp::restore(initial_state);
        }
    }
}

impl InterruptGuard {
    pub fn pop(self) {
        let _ = self; // drop
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
    interrupt_disabled_depth: AtomicUsize,
    interrupt_initial_state: UnsafeCell<imp::State>,
    irq_depth: AtomicUsize,
}

unsafe impl Sync for CpuState {}

impl CpuState {
    const fn new() -> Self {
        Self {
            interrupt_disabled_depth: AtomicUsize::new(0),
            interrupt_initial_state: UnsafeCell::new(imp::State::new()),
            irq_depth: AtomicUsize::new(0),
        }
    }

    fn interrupt_disabled_depth(&self) -> usize {
        assert!(!is_enabled());
        self.interrupt_disabled_depth.load(Ordering::Relaxed)
    }

    fn push_interrupt_state(&self, state: imp::State) {
        assert!(!is_enabled());
        let depth = self
            .interrupt_disabled_depth
            .fetch_add(1, Ordering::Acquire);
        if depth == 0 {
            unsafe {
                *self.interrupt_initial_state.get() = state;
            }
        }
    }

    fn pop_interrupt_state(&self) -> Option<imp::State> {
        assert!(!is_enabled());
        let depth = self
            .interrupt_disabled_depth
            .fetch_sub(1, Ordering::Release);
        if depth == 1 {
            unsafe { Some(*self.interrupt_initial_state.get()) }
        } else {
            None
        }
    }

    #[track_caller]
    fn irq_depth(&self) -> usize {
        assert!(!is_enabled());
        self.irq_depth.load(Ordering::Relaxed)
    }

    #[track_caller]
    fn increment_irq_depth(&self) {
        assert!(!is_enabled());
        let depth = self.irq_depth.fetch_add(1, Ordering::Relaxed);
        assert_ne!(depth, usize::MAX, "IRQ depth overflow");
    }

    #[track_caller]
    fn decrement_irq_depth(&self) {
        assert!(!is_enabled());
        let depth = self.irq_depth.fetch_sub(1, Ordering::Relaxed);
        assert_ne!(depth, 0, "IRQ depth cannot be negative");
    }

    #[track_caller]
    fn in_interrupt_handler(&self) -> bool {
        assert!(!is_enabled());
        self.irq_depth() > 0
    }
}

#[derive(Debug)]
pub struct SavedState {
    disabled_depth: usize,
    initial_state: imp::State,
    irq_depth: usize,
}

pub fn save_state() -> SavedState {
    assert!(!is_enabled());
    let state = cpu_state();
    assert!(state.interrupt_disabled_depth() > 0);
    SavedState {
        disabled_depth: state.interrupt_disabled_depth(),
        initial_state: unsafe { *state.interrupt_initial_state.get() },
        irq_depth: state.irq_depth(),
    }
}

impl SavedState {
    pub fn restore(self) {
        assert!(!is_enabled());
        let state = cpu_state();
        state
            .interrupt_disabled_depth
            .store(self.disabled_depth, Ordering::Relaxed);
        state.irq_depth.store(self.irq_depth, Ordering::Relaxed);
        unsafe {
            *state.interrupt_initial_state.get() = self.initial_state;
        }
    }
}
