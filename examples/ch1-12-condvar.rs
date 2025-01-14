use std::collections::VecDeque;
use std::sync::Condvar;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

fn main() {
    let queue = Mutex::new(VecDeque::new());
    let not_empty = Condvar::new();

    thread::scope(|s| {
        s.spawn(|| {
            // This pattern of using condvar should be similar for all it's usages.
            // There are only 2 places that would be different, I wonder if there is a macro for this.
            loop {
                let mut guard = queue.lock().expect("Mutex is not poisoned");
                let item = loop {
                    // The way how to check for the value would be different (in the macro).
                    if let Some(item) = guard.pop_front() {
                        break item;
                    } else {
                        // Coordinates wait between the guarded value (vector) and some condition (not empty)
                        //
                        // This is an atomic operation that unlocks the mutex (otherwise we are holding it
                        // opened in a loop and blocking other threads to update the vector) and waits for the
                        // condition to be true (that needs to happen for our thread to actually continue and
                        // do something useful).
                        guard = not_empty.wait(guard).expect("Mutex is not poisoned");
                    }
                };

                // Explicitcly dropping the guard to unlock the mutex.
                // That is normal for the condition variable usage.
                //
                // Otherwise the guard would protect the vector till the end of this block
                // preventing other threads to work with the vector. While this thread
                // would only need to process a single item from it that it already took.
                drop(guard);

                // The way how to process a value outside of guard would be different (in the macro).
                // Some item processing that doesn't need to be protected by the mutex.
                dbg!(item);
            }
        });

        for i in 0.. {
            queue.lock().unwrap().push_back(i);
            not_empty.notify_one();
            thread::sleep(Duration::from_secs(1));
        }
    });
}

// Here the Producer thread doesn't need to know how many consumers are there (it could be one or many)
// and it doesn't need to know the exact condition the consumers are waiting on (it could be a compound on
// several conditions).
//
// On the other hand the thread parking while easier to understand leaves posibility of some code paths
// that are not intuitive and can lead to bugs. So it is a false sense of understanding.
//
// It can be summarized this way: if you understand that thread parking would solve your problem here
// think how to do the same with condvar and use this template as a starting point.
