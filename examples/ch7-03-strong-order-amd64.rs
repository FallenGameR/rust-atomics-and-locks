/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl
*/
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release, AcqRel, SeqCst};

/*

AMD64 is said to be strongly ordered. The only possible reordering can be a delayed store operation:

- start: store A, load B
- end:   load B, store A

That ensures that almost all memory orderings are equivalent and as cheap as Relaxed.

The only exception is the store operation that can be ordered more strictly with the
SeqCst ordering, but use case for that ordering is very rare.

*/
fn main() {
}

/*

The same CPU operations. All memory orders are equivalent.

example::load_relaxed:
  mov eax, dword ptr [rdi]
  ret

example::load_acquire:
  mov eax, dword ptr [rdi]
  ret

example::load_sequential:
  mov eax, dword ptr [rdi]
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

The same CPU operations. All memory orders are equivalent.

example::load_store_relaxed:
  lock add dword ptr [rdi], 10
  ret

example::load_store_acqrel:
  lock add dword ptr [rdi], 10
  ret

example::load_store_sequential:
  lock add dword ptr [rdi], 10
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

Sequential ordering is different

example::store_relaxed:
  mov dword ptr [rdi], 0
  ret

example::store_release:
  mov dword ptr [rdi], 0
  ret

example::store_sequential:
  xor eax, eax                  ; eax = 0 but takes less space than `mov eax, 0`
  xchg dword ptr [rdi], eax     ; usually xchg is used for a swap operation
                                ; it is used here to make sure CPU would not
                                ; reorder store (mov) with a later load operation
                                ; xchg does both store and load and thus provides
                                ; the globally sequentual ordering
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
