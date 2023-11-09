use std::hint::black_box;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::time::Instant;

static A: AtomicU64 = AtomicU64::new(0);

// cargo run --release --example ch7-02-caching FUNCTION
fn main() {
    let choise: Vec<String> = std::env::args().collect();

    match choise[1].as_str() {
        "single_cpu_cache_hit" => single_cpu_cache_hit(),
        "multiple_cpu_cache_hit" => multiple_cpu_cache_hit(),
        "multiple_cpu_cache_miss" => multiple_cpu_cache_miss(),
        _ => println!("Please choose a function to run"),
    }
}

// cargo run --release --example ch7-02-caching single_cpu_cache_hit
// 0.2s release (book says it was 0.3 on Mara Bos'es machine)
fn single_cpu_cache_hit()
{
    println!("Single CPU cache hit");

    // Don't assume A is only referenced in this thread,
    // don't optimize away all the loads
    black_box(&A);

    let start = Instant::now();

    // With MESI cache coherency protocol,
    // the data should be set to be E = Exclusive state
    for _ in 0..1_000_000_000 {
        // Don't optimize away each load
        black_box(A.load(Relaxed));
    }

    println!("{:?}", start.elapsed());
}

// cargo run --release --example ch7-02-caching multiple_cpu_cache_hit
// 0.2s release
fn multiple_cpu_cache_hit()
{
    println!("Multiple CPU cache hit");

    black_box(&A);

    // Second thread reads the same variable.
    //
    // With MESI cache coherency protocol,
    // the data should be set to be S = Shared state
    //
    // We are specifically not measuring the timings
    // of the operations in the background thread,
    // we are measuring the timings on the main thread
    thread::spawn(|| {
        loop {
            black_box(A.load(Relaxed));
        }
    });

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }

    println!("{:?}", start.elapsed());
}

// cargo run --release --example ch7-02-caching multiple_cpu_cache_miss
// 0.9s-1.9s release
fn multiple_cpu_cache_miss()
{
    println!("Multiple CPU cache miss");

    black_box(&A);

    // Second thread writes the same variable.
    //
    // With MESI cache coherency protocol,
    // the data should be set to be M = Modified state
    // here and I = Invalid state in the other thread
    // and then write through and cache invalidation
    // should happen.
    thread::spawn(|| {
        loop {
            black_box(A.store(0,Relaxed));
        }
    });

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(A.load(Relaxed));
    }

    println!("{:?}", start.elapsed());
}