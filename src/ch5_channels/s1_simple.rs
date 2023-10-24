use std::collections::VecDeque;
use std::sync::Condvar;
use std::sync::Mutex;

pub struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    item_ready: Condvar,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            item_ready: Condvar::new(),
        }
    }

    pub fn send(&self, message: T) {
        // Problem: capacity growth is slow and receiver is blocked while we do that.
        // Problem: if receiver is not fast enough to process messages, the queue will grow indefinitely.
        self.queue.lock().unwrap().push_back(message);
        self.item_ready.notify_one();
    }

    pub fn receive(&self) -> T {
        // Problem: we are blocking queue for every item, fast bulk processing is not possible.
        // Problem: there could be only one receiver (we would not adress this in that book).
        let mut b = self.queue.lock().unwrap();
        loop {
            if let Some(message) = b.pop_front() {
                return message;
            }
            b = self.item_ready.wait(b).unwrap();
        }
    }
}
