// We don't have that capability in Windows, this is why Rust analyzer can't see main here.
// Windows have a similar functionality that allows to wait on changes of a memory address.
#![cfg(target_os = "linux")]

use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::Relaxed;
use std::thread;
use std::time::Duration;

pub fn wait(a: &AtomicU32, expected: u32) {
    // Refer to the futex (2) man page for the syscall signature.
    unsafe {
        libc::syscall(
            libc::SYS_futex, // The futex syscall.
            a as *const AtomicU32, // The atomic to operate on.
            libc::FUTEX_WAIT, // The futex operation.
            expected, // The expected value.
            std::ptr::null::<libc::timespec>(), // No timeout.
        );
    }
}

pub fn wake_one(a: &AtomicU32) {
    // Refer to the futex (2) man page for the syscall signature.
    unsafe {
        libc::syscall(
            libc::SYS_futex, // The futex syscall.
            a as *const AtomicU32, // The atomic to operate on.
            libc::FUTEX_WAKE, // The futex operation.
            1, // The number of threads to wake up.
        );
    }
}

// This is essentially thread parkin/unparking implementation
// in Rust on Linux before 1.48. This kind of synchronization
// is fast because if we don't need to wait no syscall is
// being made since we track all the state in user mode ourselves.
//
// For park/unpark Rust stores a AtomicU32 variable per thread
// that has 3 states: 1) init/idle 2) unparked 3) parked.

fn main() {
    let a = AtomicU32::new(0);

    thread::scope(|s| {
        s.spawn(|| {
            thread::sleep(Duration::from_secs(3));
            a.store(1, Relaxed);
            wake_one(&a);
        });

        println!("Waiting...");
        while a.load(Relaxed) == 0 {
            // Ocasionally it's possible that the wait is interrupted
            // and the thread wakes up but actually the variable was
            // not really modified. So we need a loop that checks the
            // value of a. This is similar to the thread park/unpark.
            wait(&a, 0);
        }
        println!("Done!");
    });
}
