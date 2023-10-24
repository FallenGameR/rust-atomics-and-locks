use std::cell::UnsafeCell;
use std::mem::MaybeUninit;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }

    // Since channel is explicitly borrowed here (mut) Rust would prevent
    // code to borrow or move it unless both Sender and Receiver are dropped.
    //
    // The lifetime of the signature comes from self, so it actually can be ellided:
    // pub fn split(& mut self) -> (Sender<T>, Receiver<T>)
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        // This is a safety added in case we reuse the channel after
        // both the sender and receiver were used once.
        // If channel would not be reused then this line can be skipped,
        // but we can't guarantee that via static analysis.
        //
        // This is also where the drop on a reused channel would happen.
        // Otherwise the drop would happen when the channel be out of scope.
        *self = Self::new();

        (Sender { channel: self }, Receiver { channel: self })
    }
}

// Not possible to elide the lifetime parameter, it is important part of the signature
impl<T> Sender<'_, T> {
    pub fn send(self, message: T) {
        unsafe { (*self.channel.message.get()).write(message) };
        self.channel.ready.store(true, Release);
    }
}

// Not possible to elide the lifetime parameter, it is important part of the signature
impl<T> Receiver<'_, T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    pub fn receive(self) -> T {
        if !self.channel.ready.swap(false, Acquire) {
            panic!("no message available!");
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
    use std::thread;

    // Channel needs to be created before the scope
    // so that the compiler can check that its existence
    // would outlive both the sender and receiver.
    let mut channel = Channel::new();

    thread::scope(|s| {
        let (sender, receiver) = channel.split();
        let t = thread::current();
        s.spawn(move || {
            sender.send("hello world!");
            t.unpark();
        });
        while !receiver.is_ready() {
            thread::park();
        }
        assert_eq!(receiver.receive(), "hello world!");
    });
}
