use std::cell::UnsafeCell;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Release};
use std::thread;
use std::thread::Thread;

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
    receiving_thread: Thread, // New!
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
    _no_send: PhantomData<*const ()>, // New! Pointer types are not Send, so Receiver is not Send and can't be moved to another thread
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        *self = Self::new();
        (
            Sender {
                channel: self,
                receiving_thread: thread::current(), // New!
            },

            // If receiver is moved to another thread then sender could be notifying a wrong thread.
            // To prevent that from happening we prohibit moving the receiver to another thread by
            // breaking the Send trait. We add data that doesn't have Send on it. But we don't want
            // to add any unnecessary data, so we use PhantomData of the pointer type that is not Send.
            //
            // We bind the receiver to stay on the same thread that was executing when the sender was
            // created. So even if the sender would be moved to anther thread it would still unpark
            // the correct thread where the receiver is.

            Receiver {
                channel: self,
                _no_send: PhantomData, // New!
            }
        )
    }
}

impl<T> Sender<'_, T> {
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Release);
        self.receiving_thread.unpark(); // New!
    }
}

impl<T> Receiver<'_, T> {
    pub fn receive(self) -> T {
        // The cycle is still needed here since a thread may get CPU time
        // even if send didn't call the unpark yet.
        while !self.channel.ready.swap(false, Acquire) {
            // It looks like we could also do the receiving_thread::park() here
            // but we decided to save some memory and showcase the usage of PhantomData.
            // Plus this way we guarantee that the receiver is not moved to another thread.
            thread::park();
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}

impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe { self.message.get_mut().assume_init_drop() }
        }
    }
}

#[test]
fn main() {
    let mut channel = Channel::new();
    thread::scope(|s| {
        let (sender, receiver) = channel.split();
        s.spawn(move || {
            sender.send("hello world!");
        });
        assert_eq!(receiver.receive(), "hello world!");
    });
}
