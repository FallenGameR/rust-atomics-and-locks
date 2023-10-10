use std::sync::Mutex;
use std::thread;

fn main() {
    let n = Mutex::new(0);
    thread::scope(|s| {
        for _ in 0..10 {
            s.spawn(|| {
                let tid = thread::current().id();
                println!("{tid:?} - Enter");
                let mut guard = n.lock().unwrap();
                println!("{tid:?} - Processing");
                for _ in 0..100 {
                    *guard += 1;
                }
                println!("{tid:?} - Finish");
            });
        }
    });
    assert_eq!(n.into_inner().unwrap(), 1000);
}
