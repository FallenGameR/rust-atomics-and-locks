use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

static X: AtomicI32 = AtomicI32::new(0);

fn a() {
    // There is a predictable modification order that happens within the same thread
    X.fetch_add(5, Relaxed);
    X.fetch_add(10, Relaxed);
}

fn b() {
    let a = X.load(Relaxed);
    let b = X.load(Relaxed);
    let c = X.load(Relaxed);
    let d = X.load(Relaxed);
    println!("{a} {b} {c} {d}");
}

// Relaxed memory order: modificasions of a single variable are seen consistently across all the threads
// Possible output:   0-0-0-0   0-0-5-15   0-15-15-15
// Impossible output: 0-5-0-15  0-0-10-15
fn main() {
    thread::scope(|s| {
        // All that happened before the spawn is visible to the thread
        s.spawn(a);
        s.spawn(b);
        // All that happened after the join is visible to the thread
    });
}
