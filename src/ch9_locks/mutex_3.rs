use atomic_wait::{wait, wake_one};
use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct Mutex<T> {
    /// 0: unlocked
    /// 1: locked, no other threads waiting
    /// 2: locked, other threads waiting
    state: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> where T: Send {}

pub struct MutexGuard<'a, T> {
    // pub(crate) visibility is needed for the future
    // condvar implementation that is defined in the
    // same create but in another file.
    //
    // If we didn't need to access the mutex from codvar
    // (that needs to unlock and then lock the mutex for
    // it's normal operation) then normally we would
    // treat this field as private. Nobody outside of the
    // create that is responsible for the thread syncronization
    // primitives should be able to access this mutex. Since
    // that would violate the corectness guarantees that
    // the thread syncronization library (create) provides.
    pub(crate) mutex: &'a Mutex<T>,
}

unsafe impl<T> Sync for MutexGuard<'_, T> where T: Sync {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Mutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0), // unlocked state
            value: UnsafeCell::new(value),
        }
    }

    // One can add #[inline] suggestion to the compiler for such a small method
    // it can result in a slighly larger but faster codebase. For small methods
    // usually the fast benefit is there and the code size is almost the same.
    //
    // However it is not clear why the compiler can't figure it out by itself.
    pub fn lock(&self) -> MutexGuard<T> {
        if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            // Since the lock was already locked we need to wait
            // But probably it would be unlocked soon, so we'll be smart about it
            lock_contended(&self.state);
        }
        MutexGuard { mutex: self }
    }
}

// One can add the #[cold] hint for compiller. It would suggest that this is
// not a common code path and we expect that most of the time lock can be
// done with the `if compare_exchange(0, 1)`.
//
// #[cold]
fn lock_contended(state: &AtomicU32) {
    let mut spin_count = 0;

    // Load is used first since compare and exchange is costlier.
    // xcng would invalidate whole cache line. Load would not do that.
    // So with series of loads we can actively wait for the moment
    // when the state would change without invalidating the cache line
    // that can be in use by the other CPUs.
    //
    // We spin only if there are no other waiters (we are the first one
    // to promote the lock to the contended state). As a heuristic we
    // assume that the first thread that make use of the data would
    // soon release the lock and it would set the state to 0.
    //
    // If that assumption would turn true (and most practical use cases are)
    // then the next if with the compare_and_exchange would lock the state
    // to 1 (locked but no other waiters) without making a syscall.
    //
    // If the state is already 2 then another thread already tried
    // spinning and it didn't help. So we go straight to the syscall.
    //
    // 100 cycles is a random number. There is no single best value here,
    // it all depends on the platform and the OS. 100 is a reasonably
    // good practical value that was used in Rust 1.6 on Linux.
    while state.load(Relaxed) == 1 && spin_count < 100 {
        spin_count += 1;
        std::hint::spin_loop();
    }

    // Try to acquire the lock without making a system call.
    // That is possible only if there is no thread waiting for the lock.
    if state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() {
        return;
    }

    while state.swap(2, Acquire) != 0 {
        wait(state, 2);
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        if self.mutex.state.swap(0, Release) == 2 {
            wake_one(&self.mutex.state);
        }
    }
}

// TODO (bench)
// 0.044s on release
#[test]
fn main() {
    use std::time::Instant;
    let m = Mutex::new(0);

    // Don't let the compiler assume things
    // and optimize away the mutex usage.
    // The compiler would behave similarly as if
    // there are other threads that will use the mutex.
    std::hint::black_box(&m);

    let start = Instant::now();
    for _ in 0..5_000_000 {
        // Lock should be very fast to lock and unlock
        // There are no other threads, so the state
        // would just shift between 0 and 1
        *m.lock() += 1;
    }
    let duration = start.elapsed();
    println!("locked {} times in {:?}", *m.lock(), duration);
}

// TODO (bench)
// 1.5s on release
#[test]
fn main2() {
    use std::thread;
    use std::time::Instant;
    let m = Mutex::new(0);
    std::hint::black_box(&m);
    let start = Instant::now();
    thread::scope(|s| {
        for _ in 0..4 {
            // Note that this is an extreme and unrealistic scenario
            // But it is good to test anyway
            s.spawn(|| {
                for _ in 0..5_000_000 {
                    *m.lock() += 1;
                }
            });
        }
    });
    let duration = start.elapsed();
    println!("locked {} times in {:?}", *m.lock(), duration);

    let value = *m.lock();
    assert!(value == 5_000_000 * 4, "value = {}", value);
}
