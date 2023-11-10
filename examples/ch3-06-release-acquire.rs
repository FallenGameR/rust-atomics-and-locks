use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::thread;
use std::time::Duration;

static DATA: AtomicU64 = AtomicU64::new(0);

// Like a gate to the DATA
static READY: AtomicBool = AtomicBool::new(false);

fn main() {
    thread::spawn(|| {
        DATA.store(123, Relaxed);

        // A Release operation may not be reordered
        // with any memory operations that precede it.
        //
        // CPU uses internal buffers and async operations
        // for executing individual instructions. From a
        // single core perspective everything would seem
        // to be in order, but the other cores may see
        // that instructions complete in a different order
        // of how they were started.
        //
        // Memory ordering can cause CPU to flush it's
        // internal buffers and complete all the operations
        // that still aer unfinished in the pipeline and
        // only then to proceed with the next instruction.
        //
        READY.store(true, Release); // Everything from before this store ..
    });


    // An Acquire operation may not be reordered
    // with any memory operations that follow it.
    //
    while !READY.load(Acquire) { // .. is visible after this loads `true`.
        thread::sleep(Duration::from_millis(100));
        println!("waiting...");
    }
    println!("{}", DATA.load(Relaxed));
}
