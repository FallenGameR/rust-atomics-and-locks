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
// Send means that we can copy the data to another thread.
// Sync means that we can copy the reference of that data to another thread.
unsafe impl<T> Sync for Channel<T> where T: Send {}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    /// Safety: Only call this once!
    pub unsafe fn send(&self, message: T) {
        (*self.message.get()).write(message);
        self.ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.ready.load(Acquire)
    }

    /// Safety: Only call this once,
    /// and only after is_ready() returns true!
    pub unsafe fn receive(&self) -> T {
        (*self.message.get()).assume_init_read()
    }
}
