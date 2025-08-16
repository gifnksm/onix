use alloc::{
    collections::vec_deque::VecDeque,
    sync::{Arc, Weak},
};
use core::{cell::UnsafeCell, ffi::c_void};

pub use self::context::Context;
use super::{Task, TaskSharedData};
use crate::{
    cpu, interrupt,
    sync::spinlock::{SpinMutex, SpinMutexGuard},
    task::TaskState,
};

mod context;

static RUNNABLE_TASKS: SpinMutex<VecDeque<Weak<Task>>> = SpinMutex::new(VecDeque::new());

cpu_local! {
    static SCHEDULER_STATE: SchedulerState = SchedulerState::new();
}

#[derive(Debug)]
struct SchedulerState {
    context: UnsafeCell<Context>,
    current_task: SpinMutex<Option<Arc<Task>>>,
}

unsafe impl Sync for SchedulerState {}

impl SchedulerState {
    const fn new() -> Self {
        Self {
            context: UnsafeCell::new(Context::zeroed()),
            current_task: SpinMutex::new(None),
        }
    }

    fn set_current_task(&self, task: Option<Arc<Task>>) {
        assert!(!interrupt::is_enabled());
        *self.current_task.lock() = task;
    }

    fn try_current_task(&self) -> Option<Arc<Task>> {
        assert!(!interrupt::is_enabled());
        self.current_task.lock().as_ref().map(Arc::clone)
    }
}

#[track_caller]
fn try_get_state() -> Option<&'static SchedulerState> {
    SCHEDULER_STATE.try_get()
}

#[track_caller]
fn get_state() -> &'static SchedulerState {
    try_get_state().unwrap()
}

pub fn start() -> ! {
    assert!(!interrupt::in_interrupt_handler());
    assert!(!interrupt::is_enabled());
    assert_eq!(interrupt::disabled_depth(), 0);

    let cpu = cpu::current();
    let sched_state = get_state();
    assert!(sched_state.try_current_task().is_none());

    loop {
        interrupt::enable();
        interrupt::disable();

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
                context::switch(sched_state.context.get(), &raw const shared.sched_context);
            }
            int_state.restore();

            // assert that scheduler task runs on the same CPU
            assert_eq!(cpu.id(), cpu::current().id());
            sched_state.set_current_task(None);
        }

        interrupt::wait();
    }
}

#[track_caller]
pub(super) fn push_task(task: Weak<Task>) {
    RUNNABLE_TASKS.lock().push_back(task);
}

#[track_caller]
pub fn try_current_task() -> Option<Arc<Task>> {
    let _interrupt_guard = interrupt::push_disabled();
    try_get_state()?.try_current_task()
}

#[track_caller]
pub fn current_task() -> Arc<Task> {
    try_current_task().unwrap()
}

#[track_caller]
pub fn yield_execution(shared: &mut SpinMutexGuard<TaskSharedData>) {
    assert_eq!(shared.state, TaskState::Running);
    shared.state = TaskState::Runnable;
    return_to_scheduler(shared);
}

pub(super) fn return_to_scheduler(shared: &mut SpinMutexGuard<TaskSharedData>) {
    assert!(Weak::ptr_eq(&shared.task, &Arc::downgrade(&current_task())));
    assert_ne!(shared.state, TaskState::Running);
    if shared.state == TaskState::Runnable {
        push_task(Weak::clone(&shared.task));
    }

    let sched_state = get_state();

    // Interrupt state is a property of this kernel thread, not this CPU,
    // but the state is saved per CPU. so we need to restore it manually.
    let int_state = interrupt::save_state();
    unsafe {
        context::switch(&raw mut shared.sched_context, sched_state.context.get());
    }
    int_state.restore();
}

fn task_entry(entry: extern "C" fn(*mut c_void) -> !, arg: *mut c_void) -> ! {
    assert!(!interrupt::in_interrupt_handler());
    let task = current_task();
    unsafe { task.shared.remember_locked() }.unlock();
    interrupt::enable();
    entry(arg);
}
