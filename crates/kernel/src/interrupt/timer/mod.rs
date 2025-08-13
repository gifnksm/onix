use alloc::{
    collections::binary_heap::BinaryHeap,
    sync::{Arc, Weak},
    vec::Vec,
};
use core::{arch::asm, cmp, time::Duration};

use riscv::register::scounteren;
use spin::Once;

pub use self::instant::Instant;
use super::super::cpu;
use crate::{
    sync::spinlock::SpinMutex,
    task::{self, Task, TaskId, scheduler},
};

mod instant;

const SCHEDULER_INTERVAL: Duration = Duration::from_millis(100);

static TIMER_QUEUE: Once<Vec<TimerState>> = Once::new();

#[derive(Debug)]
struct TimerState {
    queue: SpinMutex<BinaryHeap<Event>>,
    cpu_frequency: u64,
}

#[derive(Debug, Clone)]
struct Event {
    deadline: Instant,
    kind: EventKind,
}

#[derive(Debug, Clone)]
enum EventKind {
    Tick,
    Wakeup(Weak<Task>),
}

impl Ord for Event {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        other
            .deadline
            .cmp(&self.deadline)
            .then_with(|| match (&self.kind, &other.kind) {
                (EventKind::Tick, EventKind::Tick) => cmp::Ordering::Equal,
                (EventKind::Tick, _) => cmp::Ordering::Less,
                (_, EventKind::Tick) => cmp::Ordering::Greater,
                (EventKind::Wakeup(t1), EventKind::Wakeup(t2)) => {
                    let tid1 = Weak::upgrade(t1).map_or(TaskId::INVALID, |t| t.id());
                    let tid2 = Weak::upgrade(t2).map_or(TaskId::INVALID, |t| t.id());
                    tid1.cmp(&tid2)
                }
            })
    }
}

impl PartialOrd for Event {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for Event {}

pub fn init() {
    let queues = cpu::get_all()
        .iter()
        .map(|cpu| TimerState {
            queue: SpinMutex::new(BinaryHeap::new()),
            cpu_frequency: cpu.timer_frequency(),
        })
        .collect();
    TIMER_QUEUE.call_once(|| queues);
}

pub fn start() {
    assert!(!super::is_enabled());

    // allow user to use time.
    unsafe {
        scounteren::set_tm();
    }

    let cpu = cpu::current();
    let state = &TIMER_QUEUE.get().unwrap()[cpu.index()];
    let now = now();

    let mut queue = state.queue.lock();
    queue.push(Event {
        deadline: now,
        kind: EventKind::Tick,
    });
    update_timer(&queue, state.cpu_frequency);
}

pub(super) fn handle_interrupt() {
    assert!(!super::is_enabled());
    let cpu = cpu::current();
    let state = &TIMER_QUEUE.get().unwrap()[cpu.index()];

    let now = now();

    let mut do_sched = false;

    let mut queue = state.queue.lock();
    while let Some(event) = queue.peek() {
        if event.deadline > now {
            break;
        }
        let event = queue.pop().unwrap();
        queue.unlock();

        match event.kind {
            EventKind::Tick => {
                queue = state.queue.lock();
                queue.push(Event {
                    deadline: now + SCHEDULER_INTERVAL,
                    kind: EventKind::Tick,
                });
                do_sched = true;
            }
            EventKind::Wakeup(weak) => {
                if let Some(task) = Weak::upgrade(&weak) {
                    let mut shared = task.shared.lock();
                    task::resume(&mut shared);
                }
                queue = state.queue.lock();
            }
        }
    }

    update_timer(&queue, state.cpu_frequency);
    queue.unlock();

    if do_sched && let Some(task) = scheduler::try_current_task() {
        let mut shared = task.shared.lock();
        scheduler::yield_execution(&mut shared);
    }
}

fn update_timer(queue: &BinaryHeap<Event>, cpu_frequency: u64) {
    assert!(!super::is_enabled());
    let timer_ticks = queue
        .peek()
        .map_or(Instant::MAX, |e| e.deadline)
        .as_timer_ticks(cpu_frequency);
    unsafe {
        asm!("csrw stimecmp, {}", in(reg) timer_ticks);
    }
}

pub fn try_now() -> Option<Instant> {
    let int_guard = super::push_disabled();
    let timer_frequency = cpu::try_current()?.timer_frequency();

    let timer_ticks: u64;
    unsafe {
        asm!("csrr {}, time", out(reg) timer_ticks);
    }
    drop(int_guard);

    Some(Instant::from_timer_ticks(timer_ticks, timer_frequency))
}

#[track_caller]
pub fn now() -> Instant {
    try_now().unwrap()
}

pub fn sleep(dur: Duration) {
    let int_guard = super::push_disabled();
    let cpu = cpu::current();

    let task = scheduler::current_task();
    let deadline = now() + dur;
    let state = &TIMER_QUEUE.get().unwrap()[cpu.index()];
    let mut queue = state.queue.lock();
    queue.push(Event {
        deadline,
        kind: EventKind::Wakeup(Arc::downgrade(&task)),
    });
    update_timer(&queue, cpu.timer_frequency());
    queue.unlock();
    drop(int_guard);

    loop {
        let mut shared = task.shared.lock();
        task::pause(&mut shared);
        if now() >= deadline {
            break;
        }
    }
}
