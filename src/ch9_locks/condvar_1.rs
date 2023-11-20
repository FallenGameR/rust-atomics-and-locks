use atomic_wait::{wait, wake_all, wake_one};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use super::mutex_3::MutexGuard;

pub struct Condvar {
    // We don't really count anything, we just need something that
    // would be changed on every notification (every time a signal
    // is sent - it could be either in wake_one or wake_all)
    notify_events: AtomicU32,
}

impl Condvar {
    pub const fn new() -> Self {
        Self { notify_events: AtomicU32::new(0) }
    }

    pub fn notify_one(&self) {
        self.notify_events.fetch_add(1, Relaxed);
        wake_one(&self.notify_events);
    }

    pub fn notify_all(&self) {
        self.notify_events.fetch_add(1, Relaxed);
        wake_all(&self.notify_events);
    }

    // The method accepts a mutex guard (a locked mutex) that is moved into the method.
    // Note that it is not the mutex that is being moved here but just the guard.
    // There is one mutex, there could be several guards.
    //
    // The method returns a mutex that is locked.
    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        let counter_value = self.notify_events.load(Relaxed);

        // Unlock the mutex by dropping the guard, but remember the
        // original mutex so we can lock it again later.
        //
        // When we drop the guard here, the mutex is unlocked and
        // some other thread can lock it and work with the protected data.
        //
        // To make this statement work we defined mutex as `pub(crate)`.
        // This field is not accessible normally. But since we need to use
        // it in the conditional variable that requires access to it we
        // defining both of them in the same create and set crate level visibility.
        let mutex = guard.mutex;
        drop(guard);

        // Wait, but only if the counter hasn't changed since unlocking.
        wait(&self.notify_events, counter_value);

        mutex.lock()
    }
}

#[test]
fn test_condvar() {
    use super::mutex_3::Mutex;
    use std::thread;
    use std::time::Duration;

    let mutex = Mutex::new(0);
    let condvar = Condvar::new();

    let mut wakeups = 0;

    thread::scope(|s| {
        s.spawn(|| {
            // Delay execution to test the wait
            thread::sleep(Duration::from_secs(1));

            // This is what really onblocks the main thread.
            //
            // Also for the lock operation to succeed, the main
            // thread needs to unlock the mutex guard in the wait method.
            //
            // Otherwise we wirst would do a spin for a bit and then
            // promote the mutex to the system wait operation. The OS
            // would coordinate with a scheduler to put this thread
            // to sleep until the main thread would explicitly
            // drop the mutex guard.
            *mutex.lock() = 123;

            // Notify via condition variable that it is time
            // for another thread to check if the data it
            // needs to become unblocked is ready.
            //
            // The same thing could have been achieved with just the mutex
            // that we had but condition variable adds decoupling between
            // threads.
            //
            // It allows complex wait statements (any kind of condition
            // that possibly involves several other threads to do their
            // part of the work) and complex data structures to work
            // with in the mutex (like vectors).
            condvar.notify_one();
        });

        let mut m = mutex.lock();

        // There needs to be a loop to handle possible spurious wakeups.
        while *m < 100 {
            // This wait would actually wait since we delay
            // the notification by 1 second in the spawned thread.
            //
            // The wait operation doesn't add program correctness,
            // it's only goal is to improve CPU utilization. It is
            // just an optmiization to let other threads do their work
            // while the current thread is blocked on something and
            // needs some external thing to unblock it.
            //
            // When wait returns control the current thread is given
            // the opportunity to look up if it was unblocked or not.
            //
            // Also the wait operation would unlock the mutex to let
            // another thread to do something with the data in it.
            m = condvar.wait(m);

            // Wakeup would be at least 1. It could be more
            // since it is possible that we will be woken up
            // spuriously (without a corresponding notification).
            wakeups += 1;
        }

        assert_eq!(*m, 123);
    });

    // Check that the main thread actually did wait (not busy-loop),
    // while still allowing for a few spurious wake ups.
    assert!(wakeups < 10);
}
