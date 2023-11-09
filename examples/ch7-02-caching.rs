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
        "cache_line_miss" => cache_line_miss(),
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
//
// 0.9s-1.9s release - retest in on a more recent machine
// Mara Bos'es experiments:
// - her main machine 0.3 -> 3s
// - 2022 Apple M1 0.3s -> 0.5s
// - 2022 AMD 0.25s -> 0.65
//
fn multiple_cpu_cache_miss()
{
    println!("Multiple CPU cache miss");

    black_box(&A);

    // Second thread writes the same variable.
    //
    // With MESI cache coherency protocol,
    // the data should be set to be E = Exclusive state
    // here and I = Invalid state in the other thread
    // and then write through and cache
    // invalidation should happen.
    //
    // The same performance drop would happen even
    // if we would change the store operation
    // with the compare-and-exchange that fails
    // on a X64 CPU. Theoretically it should not
    // trigger the store path, but practically
    // we see the same performance drop even
    // though this operation is implemented in hardware.
    //
    // Thus for performance boost we should have
    // used load operation in the SpinLock implementation
    // followed by the compare-and-exchange. Even though
    // just a single compare-and-exchange would be enough.
    //
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

static L: [AtomicU64; 3] = [
    AtomicU64::new(0),
    AtomicU64::new(0),
    AtomicU64::new(0),
];

// cargo run --release --example ch7-02-caching cache_line_miss
// 0.9s-1.9s release - retest in on a more recent machine
fn cache_line_miss()
{
    println!("Cache line miss");
    black_box(&L);

    // Second thread writes adjacent variables.
    // But since the cache line is 64 bytes around
    // the requested memory address, we invalidate
    // the whole cache line here without touching
    // the variable that the main thread reads.
    //
    // Mara doesn't use black_box here for some reason.
    // After I added black_box the perf is the same.
    thread::spawn(|| {
        loop {
            black_box(L[0].store(0,Relaxed));
            black_box(L[2].store(0,Relaxed));
        }
    });

    let start = Instant::now();
    for _ in 0..1_000_000_000 {
        black_box(L[1].load(Relaxed));
    }

    println!("{:?}", start.elapsed());
}
