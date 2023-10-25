use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

// We tell the compiler that Channel is safe to share between threads,
// the UnsafeCell resets this guarantee so that we explicitly handle
// this garantuees and mark it for the compiler with this annotation.
//
// We are saying - ok, the compiler can't guarantee safety, but our
// code implementation that would use unsafe code will guarantee it.
//
// Send means that we can copy (move) the data to another thread.
// Sync means that we can copy (move) the reference of that data to another thread.
// Moving in Rust transfers ownership. And ownership means who can drop the object.
unsafe impl<T> Sync for Channel<T> where T: Send {}

// Problem: there is no Drop - it can't drop it's content when it goes out of scope,
//          thus if a message was sent but never received it would never be dropped.
//
// Leaks: are ok in Rust if they are consequences of another leak - leaked vector
//        leaks it's elements, this is not an undefined behavior; but normally
//        Rust would not allow you to leak a vector to begin with.
impl<T> Channel<T> {
    // `const` allows the compiler to pre-create such a structure during
    // compilation so that in runtime we can init is faster. But in order
    // to utilize that fast creation you got to use it during init of
    // some const variable only.
    //
    // The current documentation says that const doesn't change anything
    // in the runtime. Although I think it could pre-create memory layout
    // for such structs faster even in runtime because it can pre-compile
    // them and then just copy from a `template` it has.
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    // Safety: Only call this once!
    // Problem: Two threads can call it at the same time and overwrite each other.
    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, Release);
    }

    // Problem: The user must read docs and ensure that this method is called.
    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    // Safety: Only call this once, and only after is_ready() returns true!
    // Problem: The user may call it twice and that result in 2 copies of data that
    //          doesn't implement Copy (that marks that data can be safely copied).
    pub unsafe fn receive(&self) -> T {
        (*self.message.get()).assume_init_read()
    }
}
