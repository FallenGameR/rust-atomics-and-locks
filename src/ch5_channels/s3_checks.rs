use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    // Indicates if send has already started
    // better name: in_use -> sent_started
    send_started: AtomicBool,
    // Indicates if send has already finished
    // Plus it is used to limit receive to only a single call
    // Plus it is used in drop to make sure we have a message in the channel that was not received and thus needs to be dropped with the channel
    // better name: ready -> sent_finished, plus have a separate non-reentrancy flag for receive and probably something else for drop
    send_finished: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            send_started: AtomicBool::new(false),
            send_finished: AtomicBool::new(false),
        }
    }

    // Panics if no message is available yet.
    // Can be called only once, the second call will panic.
    // Thus it would panic if the message was already consumed.
    //
    // Tip: Use `is_ready` to check first.
    pub fn receive(&self) -> T {
        // If `!self.ready.load(Acquire)` would be used here
        // we would panic only if the message is not available yet
        // but still permit mutiple receive calls.
        //
        // `send_finished` here is also used to limit the number of receive calls to 1.
        // It has an improper name for it. And such behaviour depends on another
        // variable `send_started` which is not obvious.
        if !self.send_finished.swap(false, Acquire) {
            panic!("no message available!");
        }

        // Safety: We've just checked (and reset) the ready flag.
        // Now it is on caller to drop this message.
        unsafe { (*self.message.get()).assume_init_read() }
    }

    // NOTE: The ordering here used to be Acquire.
    // Relaxed insures total modification order.
    //
    // Thus if `is_ready/Relaxed` senced modification of `send_finished` from `false` (in ctor) to `true` (in send)
    // then the `receive/Acquire` would not see a different modification order and be consistent with it.
    //
    // There is no way to see `is_ready` returning true and `receive` still panicking regardless of the
    // memory ordering in `is_ready`.
    //
    // We expect that a user would keep checking for `is_ready` in a loop.
    // In the case of `Acquire` we are effectivelly flush the caches to make sure
    // there are no extra loop cycles but with a potential hit to performance.
    pub fn is_ready(&self) -> bool {
        self.send_finished.load(Relaxed)
    }

    // Panics when trying to send more than one message.
    pub fn send(&self, message: T) {
        // Total modification order of Relaxed ensures that there would be only one
        // modification (swap operation) of `send_started` from `false` (in ctor) to
        // `true` (here, just after the swap that would return false) and that would
        // be the only case which send will attempt to access the UnsafeCell.
        if self.send_started.swap(true, Relaxed) {
            panic!("can't send more than one message!");
        }
        unsafe { (*self.message.get()).write(message) };
        self.send_finished.store(true, Release);
    }

}

// Rust guarantees that value would not be used after drop
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        // If there is a message in the channel and it was not received
        // we need to clean it up.
        //
        // We don't use Atomic API here since drop can happen from a thread that
        // fully owns the object without any outstanding borrows. That also means
        // that we can't be dropping the channel in the middle of receive and
        // thus leak the contained message since the `send_finished` usage alone
        // can't guarantee that (e.g. what if `drop` would be called just before
        // the `assume_init_read`).
        if *self.send_finished.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

#[test]
fn main() {
    use std::thread;
    let channel = Channel::new();
    let t = thread::current();
    thread::scope(|s| {
        s.spawn(|| {
            channel.send("hello world!");
            t.unpark();
        });
        while !channel.is_ready() {
            thread::park();
        }
        assert_eq!(channel.receive(), "hello world!");
    });
}
