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

                // Scoping guard to release the lock as soon as possible
                {
                    let mut guard = n.lock().map_err(|e| {
                        // Lock is poisoned if the thread that hold it panicked
                        eprintln!("{tid:2?} - Lock was poisoned");
                        e.into_inner()
                    }).expect("Should be mapped to guard in either case");

                    for _ in 0..100 {
                        *guard += 1;
                    }

                    if *guard > 500 {
                        // This will panic and poison the lock
                        // For now thread::scope would re-panic,
                        // don't know how to change that
                        panic!("{tid:?} - Test panic");
                    }
                }

                thread::sleep(Duration::from_millis(1000));
                println!("{tid:2?} - Done");
            });
        }
    });
    assert_eq!(n.into_inner().unwrap(), 1000);
}
