use alloc::{
    collections::vec_deque::VecDeque,
    sync::{Arc, Weak},
};
use core::{
    cell::UnsafeCell,
    fmt, hint,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use crate::{
    interrupt::{self, InterruptGuard},
    task::{self, Task, scheduler},
};

pub struct SpinMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    locked_at: UnsafeCell<&'static Location<'static>>,
}

impl<T> Default for SpinMutex<T>
where
    T: Default,
{
    #[track_caller]
    fn default() -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(T::default()),
            locked_at: UnsafeCell::new(Location::caller()),
        }
    }
}

unsafe impl<T> Sync for SpinMutex<T> where T: Send {}

impl<T> fmt::Debug for SpinMutex<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("SpinMutex");
        match self.try_lock() {
            Some(guard) => d.field("data", &&*guard),
            None => d.field("data", &"<locked>"),
        };
        d.finish()
    }
}

impl<T> SpinMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            locked_at: UnsafeCell::new(Location::caller()),
        }
    }

    #[track_caller]
    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        let interrupt_guard = interrupt::push_disabled();

        while self.locked.swap(true, Ordering::Acquire) {
            hint::spin_loop();
        }

        unsafe {
            *self.locked_at.get() = Location::caller();
        }

        SpinMutexGuard {
            mutex: self,
            _interrupt_guard: interrupt_guard,
        }
    }

    #[track_caller]
    pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
        let interrupt_guard = interrupt::push_disabled();

        if self.locked.swap(true, Ordering::Acquire) {
            return None;
        }

        unsafe {
            *self.locked_at.get() = Location::caller();
        }

        Some(SpinMutexGuard {
            mutex: self,
            _interrupt_guard: interrupt_guard,
        })
    }

    fn is_locked(&self) -> bool {
        assert!(!interrupt::is_enabled());
        self.locked.load(Ordering::Relaxed)
    }

    pub unsafe fn remember_locked(&self) -> SpinMutexGuard<'_, T> {
        assert!(self.is_locked());
        SpinMutexGuard {
            mutex: self,
            _interrupt_guard: unsafe { interrupt::remember_disabled() },
        }
    }
}

pub struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
    _interrupt_guard: InterruptGuard,
}

unsafe impl<T> Send for SpinMutexGuard<'_, T> where T: Send {}
unsafe impl<T> Sync for SpinMutexGuard<'_, T> where T: Sync {}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        assert!(
            self.mutex.is_locked(),
            "SpinMutexGuard dropped without holding the lock"
        );
        self.mutex.locked.store(false, Ordering::Release);
    }
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.data.get() }
    }
}

impl<T> SpinMutexGuard<'_, T> {
    pub fn unlock(self) {
        let _ = self; // drop
    }
}

#[derive(Debug)]
pub struct SpinMutexCondVar {
    generation: AtomicU64,
    waiters: SpinMutex<VecDeque<Weak<Task>>>,
}

impl SpinMutexCondVar {
    pub const fn new() -> Self {
        Self {
            generation: AtomicU64::new(0),
            waiters: SpinMutex::new(VecDeque::new()),
        }
    }

    pub fn wait<'a, T>(&self, mut guard: SpinMutexGuard<'a, T>) -> SpinMutexGuard<'a, T> {
        let start_generation = self.generation.load(Ordering::Relaxed);

        let int = interrupt::push_disabled();
        let task = scheduler::current_task().unwrap();
        drop(int);

        loop {
            let mut waiters = self.waiters.lock();
            waiters.push_back(Arc::downgrade(&task));
            drop(waiters);

            let mut shared = task.shared.lock();
            let mutex = guard.mutex;
            guard.unlock();

            task::sleep(&mut shared);

            guard = mutex.lock();
            let current_generation = self.generation.load(Ordering::Acquire);
            if current_generation != start_generation {
                break;
            }
        }

        guard
    }

    pub fn notify_all(&self) {
        self.generation.fetch_add(1, Ordering::Release);
        while let Some(task) = { self.waiters.lock().pop_front() } {
            let Some(task) = Weak::upgrade(&task) else {
                continue;
            };
            let mut shared = task.shared.lock();
            task::wakeup(&mut shared);
        }
    }

    pub fn notify_one(&self) {
        self.generation.fetch_add(1, Ordering::Release);
        while let Some(task) = { self.waiters.lock().pop_front() } {
            let Some(task) = Weak::upgrade(&task) else {
                continue;
            };
            let mut shared = task.shared.lock();
            task::wakeup(&mut shared);
            break;
        }
    }
}
