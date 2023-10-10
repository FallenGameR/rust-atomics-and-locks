use std::sync::Mutex;
use std::thread;
use std::time::Duration;

fn main() {
    let n = Mutex::new(0);
    thread::scope(|s| {
        // Handling of panics
        let mut threads = Vec::with_capacity(10);

        for _ in 0..10 {
            let thread = s.spawn(|| {
                let tid = thread::current().id();
                println!("{tid:2?} - Enter");

                // Scoping guard to release the lock as soon as possible
                {
                    let mut guard = match n.lock() {
                        Ok(guard) => guard,
                        Err(err) => {
                            // Lock is poisoned if the thread that hold it panicked
                            eprintln!("ERR: {tid:2?} - Lock was poisoned");
                            err.into_inner()
                        },
                    };

                    for _ in 0..100 {
                        *guard += 1;
                    }

                    if *guard > 800 {
                        // This will panic and poison the lock
                        // thread::scope by dafult would re-panic,
                        // to handle the poison error we have to join manually
                        // Plus the panic statement would dump out the stack
                        panic!("{tid:?} - Test panic");
                    }
                }

                thread::sleep(Duration::from_millis(1000));
                println!("{tid:2?} - Done");
            });
            threads.push(thread);
        }

        for thread in threads {
            match thread.join() {
                Ok(_) => println!("Thread joined"),
                Err(err) => println!("Thread panicked: {err:?}"),
            };
        }
    });

    let inner = n.into_inner();
    let value = match inner {
        Ok(value) => value,
        Err(err) => {
            eprintln!("ERR: Had to handle a poison error");
            err.into_inner()
        },
    };
    assert_eq!(value, 1000);
}
