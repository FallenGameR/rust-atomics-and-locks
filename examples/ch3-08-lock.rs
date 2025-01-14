use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use std::thread;

static mut DATA: String = String::new();
static LOCKED: AtomicBool = AtomicBool::new(false);

fn f() {
    // load  -read  -acquire
    // store -write -release
    //
    // success: read-modify-write, load-modify-store, acquire-modify-release
    // fail:    read,              load,              acquire
    //
    // Acquire = AcqRel
    //
    // Relaxed can be used here for the fail case since we don't process
    // the fail case and thus we don't care about ordering for it

    // This also can be written as LOCKED.swap(true, Acquire) == false
    if LOCKED.compare_exchange(false, true, Acquire, Relaxed).is_ok() {
        // Safety: We hold the exclusive lock, so nothing else is accessing DATA.
        unsafe { DATA.push('!') };
        LOCKED.store(false, Release);
    }
}

fn main() {
    thread::scope(|s| {
        for _ in 0..100 {
            s.spawn(f);
        }
    });
    // DATA now contains at least one exclamation mark (and maybe more).
    assert!(unsafe { DATA.len() } > 0);
    assert!(unsafe { DATA.chars().all(|c| c == '!') });
    println!("{}", unsafe{ DATA.len() });
}
