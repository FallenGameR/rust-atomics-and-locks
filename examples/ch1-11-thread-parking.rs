use std::collections::VecDeque;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

fn main() {
    let queue = Mutex::new(VecDeque::new());

    thread::scope(|s| {
        // Consuming thread - park
        let t = s.spawn(|| loop {
            let item = queue.lock().unwrap().pop_front();
            if let Some(item) = item {
                dbg!(item);
            } else {
                thread::park();
            }
        });

        // Producing thread - unpark
        for i in 0.. {
            queue.lock().unwrap().push_back(i);
            t.thread().unpark();
            thread::sleep(Duration::from_secs(1));
        }
    });
}

/*
P
            queue.lock().unwrap().push_back(i);
C
            let item = queue.lock().unwrap().pop_front();
            if let Some(item) = item
                dbg!(item);
            let item = queue.lock().unwrap().pop_front();
            } else {
                thread::park();
P
            t.thread().unpark();
            // one more cycle here that can't retrieve any elements
            // this problem is solved via Condvar

Plus with parking the Producer must know exactly what consumer threads need to be notified and
what condition are they waiting for. So that is a lot of coupling that gets complicated very fast.
*/