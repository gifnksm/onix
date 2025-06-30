use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    panic::Location,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct SpinMutex<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    owner: UnsafeCell<&'static Location<'static>>,
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
            owner: UnsafeCell::new(Location::caller()),
        }
    }
}

unsafe impl<T> Sync for SpinMutex<T> where T: Send {}

impl<T> SpinMutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            owner: UnsafeCell::new(Location::caller()),
        }
    }

    pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
        // TODO: disable interrupts

        if self.locked.swap(true, Ordering::Acquire) {
            // TODO: enable interrupts
            return None;
        }

        unsafe {
            *self.owner.get() = Location::caller();
        }

        Some(SpinMutexGuard { mutex: self })
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        // TODO: disable interrupts

        while self.locked.swap(true, Ordering::Acquire) {}

        unsafe {
            *self.owner.get() = Location::caller();
        }

        SpinMutexGuard { mutex: self }
    }

    fn is_locked(&self) -> bool {
        // TODO: check if the current hardware thread is the owner
        self.locked.load(Ordering::Relaxed)
    }
}

pub struct SpinMutexGuard<'a, T> {
    mutex: &'a SpinMutex<T>,
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

        // TODO: enable interrupts
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
