use alloc::{
    collections::btree_map::BTreeMap,
    sync::{Arc, Weak},
};
use core::{
    ffi::c_void,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use snafu::{ResultExt as _, Snafu};
use snafu_utils::Location;

use self::scheduler::Context;
use crate::{
    memory::kernel_space::{self, KernelStack, KernelStackError},
    sync::spinlock::{SpinMutex, SpinMutexGuard},
};

pub mod scheduler;

static TASK_MAP: SpinMutex<BTreeMap<TaskId, Arc<Task>>> = SpinMutex::new(BTreeMap::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId(u64);

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl TaskId {
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskState {
    Runnable,
    Running,
    Sleep,
}

#[derive(Debug)]
pub struct TaskSharedData {
    state: TaskState,
    sched_context: Context,
    task: Weak<Task>,
}

#[derive(Debug, Snafu)]
pub enum TaskCreateError {
    #[snafu(display("failed to create kernel stack: {source}"))]
    KernelStack {
        #[snafu(implicit)]
        location: Location,
        #[snafu(implicit)]
        source: KernelStackError,
    },
}

#[derive(Debug)]
pub struct Task {
    id: TaskId,
    _kernel_stack: KernelStack,
    pub shared: SpinMutex<TaskSharedData>,
}

impl Task {
    fn new(
        entry: extern "C" fn(*mut c_void) -> !,
        arg: *mut c_void,
    ) -> Result<Arc<Self>, TaskCreateError> {
        let kernel_stack = kernel_space::allocate_kernel_stack().context(KernelStackSnafu)?;
        let sched_context = Context::new(&kernel_stack, entry, arg);
        let task = Arc::new_cyclic(|task| Self {
            id: TaskId::new(),
            _kernel_stack: kernel_stack,
            shared: SpinMutex::new(TaskSharedData {
                state: TaskState::Runnable,
                sched_context,
                task: Weak::clone(task),
            }),
        });
        Ok(task)
    }

    pub fn id(&self) -> TaskId {
        self.id
    }
}

pub fn spawn(
    entry: extern "C" fn(*mut c_void) -> !,
    arg: *mut c_void,
) -> Result<TaskId, TaskCreateError> {
    let task = Task::new(entry, arg)?;
    assert!(
        TASK_MAP
            .lock()
            .insert(task.id(), Arc::clone(&task))
            .is_none()
    );
    scheduler::push_task(Arc::downgrade(&task));
    Ok(task.id())
}

pub fn sleep(shared: &mut SpinMutexGuard<'_, TaskSharedData>) {
    assert!(Weak::ptr_eq(
        &shared.task,
        &Arc::downgrade(&scheduler::current_task().unwrap())
    ));
    shared.state = TaskState::Sleep;

    scheduler::return_to_scheduler(shared);
    assert_eq!(shared.state, TaskState::Running);
}

pub fn wakeup(shared: &mut SpinMutexGuard<'_, TaskSharedData>) {
    if shared.state == TaskState::Sleep {
        shared.state = TaskState::Runnable;
        scheduler::push_task(Weak::clone(&shared.task));
    }
}
