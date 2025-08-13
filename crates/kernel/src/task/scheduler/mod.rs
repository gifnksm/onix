use alloc::{
    collections::vec_deque::VecDeque,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{cell::UnsafeCell, ffi::c_void, iter};

use dataview::PodMethods as _;
use spin::once::Once;

pub use self::context::Context;
use super::{Task, TaskSharedData};
use crate::{
    cpu, interrupt,
    sync::spinlock::{SpinMutex, SpinMutexGuard},
    task::TaskState,
};

mod context;

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

#[track_caller]
fn try_get_state() -> Option<&'static SchedulerState> {
    let cpu = cpu::try_current()?;
    SCHEDULER_STATE.get()?.get(cpu.index())
}

#[track_caller]
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
pub fn push_task(task: Weak<Task>) {
    RUNNABLE_TASKS.lock().push_back(task);
}

#[track_caller]
pub fn current_task() -> Option<Arc<Task>> {
    try_get_state()?.current_task()
}

#[track_caller]
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
        context::switch(&raw mut shared.sched_context, sched_state.context.get());
    }
    int_state.restore();
}

fn task_entry(entry: extern "C" fn(*mut c_void) -> !, arg: *mut c_void) -> ! {
    let task = current_task().unwrap();
    unsafe { task.shared.remember_locked() }.unlock();
    interrupt::enable();
    entry(arg);
}
