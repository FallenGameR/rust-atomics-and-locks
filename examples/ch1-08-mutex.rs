use std::sync::Mutex;
use std::thread;
use std::time::Duration;

fn main() {
    let n = Mutex::new(0);
    thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                let tid = thread::current().id();
                println!("{tid:2?} - Enter");
                thread::sleep(Duration::from_millis(1000));
                println!("{tid:2?} - Locking");
                let mut guard = n.lock().unwrap();
                println!("{tid:2?} - Processing");
                for _ in 0..100 {
                    *guard += 1;
                }
                println!("{tid:2?} - Done");
            });
        }
    });
    assert_eq!(n.into_inner().unwrap(), 1000);
}
