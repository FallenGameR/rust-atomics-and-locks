/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl
*/
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release, AcqRel, SeqCst};

/*

ARM64 is said to be weekly ordered. Potentially any memory operation can be reordered.

Relaxed is very different from other orderings. It's code uses different instructions
that become theirstrickly ordered analogs with other types of memory ordering:

- str  (store register)           -> strL  (store-reLease register)
- stxr (store exclusive register) -> stLxr (store-reLease exclusive register)
- ldr  (load register)            -> ldAr  (load-Acquire register)
- ldxr (load exclusive register)  -> ldAxr (load-Acquire exclusive register)

ldAr and ldAxr will never be reordered with with any later memory operations.
strL and stLxr will never be reordered with any earlier memory operations.

AcqRel is different from an Acquire and a Release pair:

- AcqRel would use  ldAxr and stLxr
- Acquire would use ldAxr and stxr
- Release would use ldxr  and stLxr

SeqCst is the same as AcqRel. And Relaxed is cheaper than on AMD64 but doesn't
provide any ordering guarantees.

Because of difference how strongly oredered and weekly ordered Relaxed vs AcqRel
works it is possible for a program to work correctly on AMD64 but not on ARM64.
This happens when Relaxed memory ordering is used when AcqRel needs to be used.

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
