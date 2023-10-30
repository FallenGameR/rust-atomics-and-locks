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