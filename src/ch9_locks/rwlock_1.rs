use atomic_wait::{wait, wake_all, wake_one};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct RwLock<T> {
    /// The number of readers, or u32::MAX if write-locked.
    state: AtomicU32,
    value: UnsafeCell<T>,
}

// T needs to be Sync since mutiple readers would need
// to be able to access the data at the same time.
//
// RwLock does not implement Send so that it would not
// be used to Send an Rc to another thread.
unsafe impl<T> Sync for RwLock<T> where T: Send + Sync {}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0), // Unlocked.
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        let mut s = self.state.load(Relaxed);
        loop {
            if s < u32::MAX {
                assert!(s < u32::MAX - 1, "too many readers");
                match self.state.compare_exchange_weak(
                    s, s + 1, Acquire, Relaxed
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => s = e,
                }
            }
            if s == u32::MAX {
                // Wait until the state changes from MAX that means writer-locked.
                wait(&self.state, u32::MAX);
                s = self.state.load(Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        while let Err(state_value) = self.state.compare_exchange(
            0, u32::MAX, Acquire, Relaxed
        ) {
            // Wait while already locked.
            // Wait untill the state_value changes.
            wait(&self.state, state_value);
        }
        WriteGuard { rwlock: self }
    }
}


pub struct WriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

// This gives the Read semantic to RwLock that a writer acquired
impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

// This gives the Write semantic to RwLock that a writer acquired
impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.rwlock.value.get() }
    }
}

// Unlocking done by a writer unlocks everybody
impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.store(0, Release);
        // Wake up all waiting readers and writers.
        //
        // If we have competing readers and writers
        // there would be a race with a random winner.
        wake_all(&self.rwlock.state);
    }
}

pub struct ReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

// This gives the Read semantic to RwLock that a reader acquired
impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

// Unlocking done by a reader can onlock only a writer
// all the readers are not blocked by each other
impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        if self.rwlock.state.fetch_sub(1, Release) == 1 {
            // We don't have any waiting readers now.
            // But we can have a waiting writer that awaits
            // till the state is zero. We need to wake up that
            // writer for it to recheck the state and wake up.
            wake_one(&self.rwlock.state);
        }
    }
}

// Release - flush all the pending operations
// Acquire - wait for all the pending operations to complete
// They work only when they are used cooperatively