use core::{
    cell::UnsafeCell,
    fmt, hint,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::interrupt::{self, InterruptGuard};

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
