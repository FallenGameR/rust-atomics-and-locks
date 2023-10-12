use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;

fn allocate_new_id() -> u32 {
    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    let id = NEXT_ID.fetch_add(1, Relaxed);
    if id >= 1000 {
        // That would work with the current threshold.
        // There is still a possiblility of multiple threads to increment the value
        // before reverting the change. But that many threads would not be possible
        // to fit in memory. However, if the threshold is set to u32::MAX, then
        // we can have an overflow problem.
        NEXT_ID.fetch_sub(1, Relaxed);
        panic!("too many IDs!");
    }
    id
}

fn main() {
    dbg!(allocate_new_id());
    dbg!(allocate_new_id());
    dbg!(allocate_new_id());
    // TODO
}
