use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;

// if generating key takes a long time it would be better to use OnceLock
// this way we wouldn't spend CPU time on generating the key multiple times
fn get_key() -> u64 {
    static KEY: AtomicU64 = AtomicU64::new(0);
    let key = KEY.load(Relaxed);
    if key == 0 {
        let new_key = generate_random_key();
        // _week version must not be used here since it can return Err(0)
        match KEY.compare_exchange(0, new_key, Relaxed, Relaxed) {
            Ok(_) => new_key,
            Err(k) => k,
        }
    } else {
        key
    }
}

fn generate_random_key() -> u64 {
    123
    // TODO
}

fn main() {
    dbg!(get_key());
    dbg!(get_key());
    dbg!(get_key());
}
