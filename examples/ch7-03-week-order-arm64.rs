/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl
*/
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release, AcqRel, SeqCst};

/*

ARM64 is said to be weekly ordered

*/
fn main() {
}

/*

The same ordering as Relaxed.

example::load_relaxed:
  ldr w0, [x0]
  ret

example::load_acquire:
  ldar w0, [x0]
  ret

example::load_sequential:
  ldar w0, [x0]
  ret

*/
pub fn load_relaxed(x: &AtomicI32) -> i32 {
    x.load(Relaxed)
}
pub fn load_acquire(x: &AtomicI32) -> i32 {
    x.load(Acquire)
}
pub fn load_sequential(x: &AtomicI32) -> i32 {
    x.load(SeqCst)
}

/*

Relaxed is different. Acquire and Sequential are the same.

example::load_store_relaxed:
.LBB0_1:
  ldxr w8, [x0]
  add w8, w8, #10
  stxr w9, w8, [x0]
  cbnz w9, .LBB0_1
  ret

example::load_store_acqrel:
.LBB1_1:
  ldaxr w8, [x0]
  add w8, w8, #10
  stlxr w9, w8, [x0]
  cbnz w9, .LBB1_1
  ret

example::load_store_sequential:
.LBB2_1:
  ldaxr w8, [x0]
  add w8, w8, #10
  stlxr w9, w8, [x0]
  cbnz w9, .LBB2_1
  ret

*/
pub fn load_store_relaxed(x: &AtomicI32) {
    x.fetch_add(10, Relaxed);
}
pub fn load_store_acqrel(x: &AtomicI32) {
    x.fetch_add(10, AcqRel);
}
pub fn load_store_sequential(x: &AtomicI32) {
    x.fetch_add(10, SeqCst);
}

/*

Relaxed is different. Acquire and Sequential are the same.

example::store_relaxed:
  str wzr, [x0]
  ret

example::store_release:
  stlr wzr, [x0]
  ret

example::store_sequential:
  stlr wzr, [x0]
  ret

*/
pub fn store_relaxed(num: &AtomicI32) {
    num.store(0, Relaxed);
}
pub fn store_release(num: &AtomicI32) {
    num.store(0, Release);
}
pub fn store_sequential(num: &AtomicI32) {
    num.store(0, SeqCst);
}
