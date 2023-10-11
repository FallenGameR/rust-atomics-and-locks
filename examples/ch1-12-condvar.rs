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
            loop {
                let mut guard = queue.lock().expect("Mutex is not poisoned");
                let item = loop {
                    if let Some(item) = guard.pop_front() {
                        break item;
                    } else {
                        guard = not_empty.wait(guard).expect("Mutex is not poisoned");
                    }
                };
                drop(guard);
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
