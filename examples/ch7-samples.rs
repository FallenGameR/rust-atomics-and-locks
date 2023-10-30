/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl

*/
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release, AcqRel, SeqCst};

/*

example::add_ten:
  add dword ptr [rdi], 10
  ret

example::add_ten:
        ldr     w8, [x0]
        add     w8, w8, #10
        str     w8, [x0]
        ret

*/
pub fn add_ten(num: &mut i32) {
    *num += 10;
}

/*

example::store:
  mov dword ptr [rdi], 0
  ret

example::store:
        str     wzr, [x0]
        ret

*/
pub fn store(num: &mut i32) {
    *num = 0;
}

/*

example::store_relaxed:
  mov dword ptr [rdi], 0
  ret

example::store_relaxed:
        str     wzr, [x0]
        ret

*/
pub fn store_relaxed(num: &AtomicI32) {
    num.store(0, Relaxed);
}

fn main() {
}