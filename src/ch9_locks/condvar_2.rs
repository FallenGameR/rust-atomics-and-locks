use atomic_wait::{wait, wake_all, wake_one};
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use super::mutex_3::MutexGuard;

pub struct Condvar {
    counter: AtomicU32,
    // This new implementation optimizes the syscall usage.
    //
    // We can make wake call conditional and skip them in
    // the case there are no other threads that can be awoken.
    num_waiters: AtomicUsize,
}

impl Condvar {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
            num_waiters: AtomicUsize::new(0),
        }
    }

    pub fn notify_one(&self) {
        // My explanation why the Relaxed ordering is enough here is:
        // since all the cores would observe the modifications in the
        // same order and we structure the code in a way that the num_waiters
        // is incremented as the first thing that we do and decremented
        // as the last thing that we do. There is no way for the
        // read operation to read a value outside of that order.
        // But that explanation is wrong.
        //
        // Think of out-of-order operations as cached operations that
        // still didn't finish. So let's say condvar's wait started
        // for the first time and it set num_waiters from 0 to 1.
        // This operation takes some time and it is not finished yet.
        // So other cores could read the value as 0. But since the
        // wait operation started that means that there is at least
        // one more thread that needs to be woken up.
        //
        // The book explanation is that we have a risk of missing
        // a notification only when load reads 0 while there exists
        // a thread to wake up. In this case the load observes the
        // value either before the increment or after the decrement.
        //
        // Again, Mara says that the mutax's lock and unlock/drop
        // are guranteeing the ordering here. Their implementation
        // contains acquire/release that makes sure that all the
        // pending operations on the CPU are finished before we
        // get Instruction Pointer that executes our code.
        // If mutex's code returned then all the pending CPU
        // operations were finished and there could not be
        // a caching problem here.
        if self.num_waiters.load(Relaxed) > 0 {
            self.counter.fetch_add(1, Relaxed);
            wake_one(&self.counter);
        }
    }

    pub fn notify_all(&self) {
        if self.num_waiters.load(Relaxed) > 0 {
            self.counter.fetch_add(1, Relaxed);
            wake_all(&self.counter);
        }
    }

    pub fn wait<'a, T>(&self, guard: MutexGuard<'a, T>) -> MutexGuard<'a, T> {
        self.num_waiters.fetch_add(1, Relaxed);
        let counter_value = self.counter.load(Relaxed);
        let mutex = guard.mutex;

        // Mutex unlock/drop here ensures that num_waiters fetch_add completed.
        // Meaning that wait here would only start if num_waiters is at least 1
        // and that is the signal that there is at least one waiting thread
        // for the wake operation to wake up if needed.
        drop(guard);

        // There is nothing to optimize here (no additional ifs).
        // The futex wait call would first check if the value did change
        // since the last load and would only wait if it stayed the same.
        wait(&self.counter, counter_value);

        self.num_waiters.fetch_sub(1, Relaxed);

        // Mutex lock here ensures that num_waiters fetch_sub completed.
        // Meaning that wait here was completed and thus there is no
        // waiting thread that can be woken up in the case num_waiters is 0.
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
            thread::sleep(Duration::from_secs(1));
            *mutex.lock() = 123;
            condvar.notify_one();
        });

        let mut m = mutex.lock();
        while *m < 100 {
            m = condvar.wait(m);
            wakeups += 1;
        }

        assert_eq!(*m, 123);
    });

    // Check that the main thread actually did wait (not busy-loop),
    // while still allowing for a few spurious wake ups.
    assert!(wakeups < 10);
}
