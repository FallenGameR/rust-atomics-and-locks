/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl
*/
use std::sync::atomic::AtomicI32;
use std::sync::atomic::Ordering::{Relaxed, Acquire, Release, AcqRel, SeqCst};

fn main() {
}

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

/*

example::load:
  mov eax, dword ptr [rdi]
  ret

example::load:
  ldr     w0, [x0]
  ret

*/
pub fn load(x: &i32) -> i32 {
    *x
}

/*

example::load_relaxed:
  ; It is the same atomic operation for processor,
  ; but not for the compiler. The compiler could have
  ; split this operation into two that use 16-bit registers
  mov eax, dword ptr [rdi]
  ret

example::load_relaxed:
  ldr w0, [x0]
  ret

*/
pub fn load_relaxed(x: &AtomicI32) -> i32 {
    x.load(Relaxed)
}

/*

; X64 is CISC architecture - complex instruction set computer
example::read_modify_write:
  ;
  ; although it looks atomic, it is actually not
  ; the processor would split this `add` operation
  ; into several microinstructions similar to RISC
  ;
  ; that would be save on a single core CPU though since
  ; the context switches happen only betweern instructions,
  ; not between microinstructions
  ;
  ; but modern CPUs are muti-core and there we don't
  ; have such guarantee
  ;
  add dword ptr [rdi], 10
  ret

; AARCH64 is RISC architecture - reduced instruction set computer
example::read_modify_write:
  ldr w8, [x0]
  ; [x0] can be modified by another thread here
  add w8, w8, #10
  ; and here
  str w8, [x0]
  ret

*/
pub fn read_modify_write(x: &mut i32) {
    *x += 10;
}

/*

;
; `lock` would prohibit other cores to access
; the same memory region until the `add` operation
; is finished. The very first implementation of
; the `lock` stopped all other cores altogether.
;
; lock is implemented for add, sub, and, or, xor
; lock is automatically applied to all xchg operations (atomic swap)
;
example::read_modify_write_relaxed:
  lock add dword ptr [rdi], 10
  ret
*/

pub fn read_modify_write_relaxed(x: &AtomicI32) {
    x.fetch_add(10, Relaxed);
}

/*
;
; add doesn't return the result of the operation, it only populates
; the flag register with the metadata of the operation results
;
; if we need to return the result of our operation, we need to use
; xadd that is `add and exchange` that in our case populates eax
; that by the x86 convention is used to return the result of the function
;
; note that there is no xsub, xand, xor, xxor operations.
; - xsub can be implemented as xadd -value
; - xor and xand can be implemented as `bts` = `bit test and set` or
;   `btc` = `bit test and complement` operation in the case we are
;    affecting only a single bit .or(1) .xor(-1)
; - otherwise we need to use several instructions for fetch_or and fetch_and
; - similarly there is no `lock` instruction for fetch_max and fetch_min
;
example::read_modify_write_relaxed_return:
  mov eax, 10
  lock xadd dword ptr [rdi], eax
  ret

*/

pub fn read_modify_write_relaxed_return(x: &AtomicI32) -> i32 {
    x.fetch_add(10, Relaxed)
}