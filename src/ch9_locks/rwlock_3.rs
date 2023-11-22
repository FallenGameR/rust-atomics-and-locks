use atomic_wait::{wait, wake_all, wake_one};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct RwLock<T> {
    /// The number of read locks times two, plus one if there's
    /// a writer waiting, u32::MAX if write locked.
    ///
    /// This means that readers may acquire the lock only
    /// when the state is even, but need to block when odd.
    ///
    /// We don't track the number of writers here.
    /// It happens naturally in the writer wait method.
    /// They all would just block on the same state value
    /// and unblock only other writers until there is
    /// no other writer left.
    state: AtomicU32,
    /// Incremented to wake up writers.
    writer_wake_counter: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for RwLock<T> where T: Send + Sync {}

impl<T> RwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            writer_wake_counter: AtomicU32::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn read(&self) -> ReadGuard<T> {
        let mut s = self.state.load(Relaxed);
        loop {
            if s % 2 == 0 { // Even.
                assert!(s < u32::MAX - 2, "too many readers");
                match self.state.compare_exchange_weak(
                    s, s + 2, Acquire, Relaxed
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => s = e,
                }
            }
            if s % 2 == 1 { // Odd.
                wait(&self.state, s);
                s = self.state.load(Relaxed);
            }
        }
    }

    pub fn write(&self) -> WriteGuard<T> {
        let mut state = self.state.load(Relaxed);
        loop {
            // If we see it as unlocked, try to get the exclusive access to the data
            // by locking it.
            //
            // 0 means no readers and no writers are waiting.
            // 1 means that there were readers reading the data then a writer
            // arrived and started waiting for the lock. Every reader decremented
            // the state by 2 until there were no more readers left. And the last
            // reader would signal on the writer_wake_counter to wake up the writer
            // that have waited before.
            //
            // The code that observes 1 in the state could be either that writer
            // who was just woken up and got to check the state again after the
            // wait returned. Or it could be a new writer that would also observe
            // 1 and would compete with the old writer in the compare_exchange.
            //
            // Looks like the writers would race. If that's a problem, there should
            // be a queue implemented for the writers. Writers would be waked up
            // in the order they arrived.
            if state <= 1 {
                match self.state.compare_exchange(
                    state, u32::MAX, Acquire, Relaxed
                ) {
                    Ok(_) => return WriteGuard { rwlock: self },
                    Err(e) => { state = e; continue; }
                }
            }

            // Block new readers, by making sure the state is odd.
            // Relaxed is enough here since we don't lock the data?
            if state % 2 == 0 {
                match self.state.compare_exchange(
                    state, state + 1, Relaxed, Relaxed
                ) {
                    // We marked with the odd state that there is a writer waiting.
                    // If state was changed from 0 to 1 then the code below would
                    // not trigger the wait and the next loop would aquire the lock
                    // for the single writer that we have.
                    Ok(_) => {}
                    // State was updated by someone, try again in a new loop iteration
                    Err(e) => { state = e; continue; }
                }
            }

            // This is essentially what the previous code did
            let writer_is_done_notification = self.writer_wake_counter.load(Acquire);
            state = self.state.load(Relaxed);

            // There is either a reader or a writer doing something the the locked data
            // right now. Wait until the state would change to somethind we can unlock.
            if state >= 2 {
                wait(&self.writer_wake_counter, writer_is_done_notification);
                state = self.state.load(Relaxed);
            }
        }
    }
}

pub struct ReadGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

pub struct WriteGuard<'a, T> {
    rwlock: &'a RwLock<T>,
}

impl<T> Deref for WriteGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T> DerefMut for WriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.rwlock.value.get() }
    }
}

impl<T> Deref for ReadGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<T> Drop for ReadGuard<'_, T> {
    fn drop(&mut self) {
        // Decrement the state by 2 to remove one read-lock.
        if self.rwlock.state.fetch_sub(2, Release) == 3 {
            // If we decremented from 3 to 1, that means
            // the RwLock is now unlocked _and_ there is
            // a waiting writer, which we wake up.
            self.rwlock.writer_wake_counter.fetch_add(1, Release);
            wake_one(&self.rwlock.writer_wake_counter);
        }
    }
}

impl<T> Drop for WriteGuard<'_, T> {
    fn drop(&mut self) {
        self.rwlock.state.store(0, Release);
        self.rwlock.writer_wake_counter.fetch_add(1, Release);
        wake_one(&self.rwlock.writer_wake_counter);
        wake_all(&self.rwlock.state);
    }
}
