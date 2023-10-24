use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;

static X: AtomicI32 = AtomicI32::new(0);

fn a1() {
    X.fetch_add(5, Relaxed);
}

fn a2() {
    X.fetch_add(10, Relaxed);
}

fn b() {
    let a = X.load(Relaxed);
    let b = X.load(Relaxed);
    let c = X.load(Relaxed);
    let d = X.load(Relaxed);
    println!("{a} {b} {c} {d}");
}

// Possible orders: 0-0-0  5-5-5  10-10-10  0-5-15  0-10-15
// If a single b sees 5 then no other b can see 10
fn main() {
    thread::scope(|s| {
        s.spawn(a1);
        s.spawn(a2);
        s.spawn(b);
        s.spawn(b);
        s.spawn(b);
    });
}
