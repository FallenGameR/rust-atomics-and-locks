use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

fn allocate_new_id() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    let mut id = NEXT_ID.load(Relaxed);
    loop {
        // Theshold can be set to U32::MAX and we still would not get an overflow.
        assert!(id < 1000, "too many IDs!");
        // _week version is faster but it may return Err even if the expected value matched.
        match NEXT_ID.compare_exchange_weak(id, id + 1, Relaxed, Relaxed) {
            Ok(_) => return id,
            Err(v) => id = v,
        }
    }

    // There is a convinience method that would allow to do the same in one line:
    // NEXT_ID.fetch_update(Relaxed, Relaxed, |n| n.checked_add(1)).expect("We should not reach the MAX value")
}

fn main() {
    dbg!(allocate_new_id());
    dbg!(allocate_new_id());
    dbg!(allocate_new_id());
    // TODO
}
