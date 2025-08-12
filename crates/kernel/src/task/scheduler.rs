use alloc::{
    collections::vec_deque::VecDeque,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{arch::naked_asm, cell::UnsafeCell, iter, mem::offset_of};

use dataview::{Pod, PodMethods as _};
use spin::once::Once;

use super::{Task, TaskSharedData};
use crate::{
    cpu, interrupt,
    memory::kernel_space::KernelStack,
    sync::spinlock::{SpinMutex, SpinMutexGuard},
    task::TaskState,
};

static RUNNABLE_TASKS: SpinMutex<VecDeque<Weak<Task>>> = SpinMutex::new(VecDeque::new());

static SCHEDULER_STATE: Once<Vec<SchedulerState>> = Once::new();

#[derive(Debug)]
struct SchedulerState {
    context: UnsafeCell<Context>,
    current_task: SpinMutex<Option<Arc<Task>>>,
}

unsafe impl Sync for SchedulerState {}

impl SchedulerState {
    fn new() -> Self {
        Self {
            context: UnsafeCell::new(Context::zeroed()),
            current_task: SpinMutex::new(None),
        }
    }

    fn set_current_task(&self, task: Option<Arc<Task>>) {
        assert!(!interrupt::is_enabled());
        *self.current_task.lock() = task;
    }

    fn current_task(&self) -> Option<Arc<Task>> {
        assert!(!interrupt::is_enabled());
        self.current_task.lock().as_ref().map(Arc::clone)
    }
}

#[derive(Debug, Clone, Copy, Pod)]
#[repr(C)]
pub struct Context {
    ra: usize,
    // callee-saved registers
    sp: usize,
    s0: usize,
    s1: usize,
    s2: usize,
    s3: usize,
    s4: usize,
    s5: usize,
    s6: usize,
    s7: usize,
    s8: usize,
    s9: usize,
    s10: usize,
    s11: usize,
}
impl Context {
    pub(crate) fn new(entry: extern "C" fn() -> !, stack: &KernelStack) -> Self {
        let mut context = Self::zeroed();
        context.ra = task_entry as usize;
        context.sp = stack.top();
        context.s1 = entry as usize;
        context
    }
}

fn try_get_state() -> Option<&'static SchedulerState> {
    let cpu = cpu::try_current()?;
    SCHEDULER_STATE.get()?.get(cpu.index())
}

fn get_state() -> &'static SchedulerState {
    try_get_state().unwrap()
}

pub fn init() {
    let states = iter::repeat_with(SchedulerState::new)
        .take(cpu::len())
        .collect::<Vec<_>>();
    SCHEDULER_STATE.call_once(|| states);
}

pub fn start() -> ! {
    assert!(!interrupt::is_enabled());
    assert_eq!(interrupt::disabled_depth(), 0);

    let cpu = cpu::current();
    let sched_state = get_state();
    assert!(sched_state.current_task().is_none());

    loop {
        unsafe {
            riscv::interrupt::enable();
            riscv::interrupt::disable();
        }

        while let Some(task) = { RUNNABLE_TASKS.lock().pop_front() } {
            let Some(task) = Weak::upgrade(&task) else {
                continue;
            };
            let mut shared = task.shared.lock();
            if shared.state != TaskState::Runnable {
                continue;
            }
            shared.state = TaskState::Running;

            sched_state.set_current_task(Some(Arc::clone(&task)));

            // Interrupt state is a property of this kernel thread, not this CPU,
            // but the state is saved per CPU. so we need to restore it manually.
            let int_state = interrupt::save_state();
            unsafe {
                switch(sched_state.context.get(), &raw const shared.sched_context);
            }
            int_state.restore();

            // assert that scheduler task runs on the same CPU
            assert_eq!(cpu.id(), cpu::current().id());
            sched_state.set_current_task(None);
        }

        riscv::asm::wfi();
    }
}

pub fn push_task(task: Weak<Task>) {
    RUNNABLE_TASKS.lock().push_back(task);
}

pub fn current_task() -> Option<Arc<Task>> {
    try_get_state()?.current_task()
}

pub fn yield_execution(task: &Task) {
    let mut shared = task.shared.lock();
    assert_eq!(shared.state, TaskState::Running);
    shared.state = TaskState::Runnable;
    return_to_scheduler(&mut shared);
    shared.unlock();
}

fn return_to_scheduler(shared: &mut SpinMutexGuard<TaskSharedData>) {
    assert_ne!(shared.state, TaskState::Running);
    if shared.state == TaskState::Runnable {
        push_task(Weak::clone(&shared.task));
    }

    let sched_state = get_state();

    // Interrupt state is a property of this kernel thread, not this CPU,
    // but the state is saved per CPU. so we need to restore it manually.
    let int_state = interrupt::save_state();
    unsafe {
        switch(&raw mut shared.sched_context, sched_state.context.get());
    }
    int_state.restore();
}

/// Saves current registers in `old`, loads from `new`.
#[unsafe(naked)]
unsafe extern "C" fn switch(old: *mut Context, new: *const Context) {
    naked_asm!(
        "sd ra, {c_ra}(a0)",
        "sd sp, {c_sp}(a0)",
        "sd s0, {c_s0}(a0)",
        "sd s1, {c_s1}(a0)",
        "sd s2, {c_s2}(a0)",
        "sd s3, {c_s3}(a0)",
        "sd s4, {c_s4}(a0)",
        "sd s5, {c_s5}(a0)",
        "sd s6, {c_s6}(a0)",
        "sd s7, {c_s7}(a0)",
        "sd s8, {c_s8}(a0)",
        "sd s9, {c_s9}(a0)",
        "sd s10, {c_s10}(a0)",
        "sd s11, {c_s11}(a0)",
        "ld ra, {c_ra}(a1)",
        "ld sp, {c_sp}(a1)",
        "ld s0, {c_s0}(a1)",
        "ld s1, {c_s1}(a1)",
        "ld s2, {c_s2}(a1)",
        "ld s3, {c_s3}(a1)",
        "ld s4, {c_s4}(a1)",
        "ld s5, {c_s5}(a1)",
        "ld s6, {c_s6}(a1)",
        "ld s7, {c_s7}(a1)",
        "ld s8, {c_s8}(a1)",
        "ld s9, {c_s9}(a1)",
        "ld s10, {c_s10}(a1)",
        "ld s11, {c_s11}(a1)",
        "ret",
        c_ra = const offset_of!(Context, ra),
        c_sp = const offset_of!(Context, sp),
        c_s0 = const offset_of!(Context, s0),
        c_s1 = const offset_of!(Context, s1),
        c_s2 = const offset_of!(Context, s2),
        c_s3 = const offset_of!(Context, s3),
        c_s4 = const offset_of!(Context, s4),
        c_s5 = const offset_of!(Context, s5),
        c_s6 = const offset_of!(Context, s6),
        c_s7 = const offset_of!(Context, s7),
        c_s8 = const offset_of!(Context, s8),
        c_s9 = const offset_of!(Context, s9),
        c_s10 = const offset_of!(Context, s10),
        c_s11 = const offset_of!(Context, s11),
    )
}

#[unsafe(naked)]
unsafe extern "C" fn task_entry() -> ! {
    naked_asm!(
        "mv a0, s1",
        "j {task_entry_secondary}",
        task_entry_secondary = sym task_entry_secondary,
    );
}

extern "C" fn task_entry_secondary(entry: extern "C" fn() -> !) -> ! {
    let task = current_task().unwrap();
    unsafe { task.shared.remember_locked() }.unlock();
    assert_eq!(interrupt::disabled_depth(), 0);
    unsafe {
        riscv::interrupt::enable();
    }
    entry();
}
