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


/*

;
; similarly to how we can replace any atomic operation with the
; compare-and-exchange loop the compiler does it for the cases
; when an atomic operation is not implemented by the processor:
;
; examples/ch2-11-increment-with-compare-exchange.rs
;
example::read_modify_write_relaxed_compare_and_exchange:
  mov eax, dword ptr [rdi]
.LBB7_1:
  mov ecx, eax
  or ecx, 10
  lock cmpxchg dword ptr [rdi], ecx
  jne .LBB7_1
  ret

*/
pub fn read_modify_write_relaxed_compare_and_exchange(x: &AtomicI32) -> i32 {
    x.fetch_or(10, Relaxed)
}

/*

;
; the same code as above, but we implemented fetch_or ourselves
; btw, on x86_64 both compare_exchange and compare_exchange_weak
; compile to the `lock cmpxchg` instruction
;
example::read_modify_write_relaxed_compare_and_exchange_explicit:
  mov eax, dword ptr [rdi]
.LBB8_1:
  mov ecx, eax
  or ecx, 10
  lock cmpxchg dword ptr [rdi], ecx
  jne .LBB8_1
  ret

*/
pub fn read_modify_write_relaxed_compare_and_exchange_explicit(x: &AtomicI32) -> i32 {
    let mut current = x.load(Relaxed);
    loop {
        let new = current | 10;
        match x.compare_exchange(current, new, Relaxed, Relaxed) {
            Ok(value) => return value,
            Err(value) => current = value,
        }
    }
}

/*

On RISC compare-and-exchange loop is implemented with load-linked/store-conditional
LL/SC loop. There is no atomic `lock cmpxchg` instruction.

During the linked load the current core core starts to track modifications
of a single memory addres. This memory address can be as precise as 64 bytes
or a page of memory or the whole memory, the less precise it is the more
cycles the loop will makes but the simpler the implementation would be.

During the conditional store the core checks if that memory region got modified.
Sometimes it would indicate that that memory region was modified but in fact
it was not. This is acceptable and would just result in more cycles through the loop.

An example of such an implementation that would give us false positives would
be a single core system where the core assumes the worst case (memory was potentially
modified) any time a thread switch occurs (on interrupt or context switch).
So implementation would play it safe and cause the code to loop one more time.

LL/SC is flexible in a way that it can be used to implement any kind of atomic
operation. No need to have specialized xadd and xor instructions. Compiler would
try to use as less intructions between LL and SC as possible to minimize the
chances of memory modification and improve the chances of the SC to succeed.

*/

/*

RISC, ARMv8

example::atomic_addition_arm:
.LBB0_1:
  ldxr w8, [x0]         ; load exclusive register
  add w8, w8, #10
  stxr w9, w8, [x0]     ; store exclusive register
                        ; also `clrex` (clear exclusive) can be used
                        ; to stop tracking writes to the memory without
                        ; storing anything
  cbnz w9, .LBB0_1      ; compare uses result of the `stxr` store operation
                        ; that would indicate if the memory [x0] was modified
                        ; after it was loaded via `ldxr` load operation
                        ; so `ldxr` stores somewhere the [x0] value
                        ; and `stxr` compares it with the current [x0] value
                        ;
                        ; strx: w9 = store(from: w8, to: [x0], state: implicitly stored state of [x0] from the last ldxr call)
  ret

ARMv8.1 though is not RISC. It adds CISC style of instructions for the most
common operations like `add`, `swap` and `max`. So if hardware supports it
the compiler can write a more performant code for the atomic operations.

*/
pub fn atomic_addition_arm(x: &AtomicI32) {
    x.fetch_add(10, Relaxed);
}

/*

Weak compare exchange can fail and that is reflected by the branch_not_equal
jump that clears the exclusive register and skips exchange altogether.

example::compare_exchange_arm:
  ldxr w8, [x0]         ; [x0] is the memory location of x argument
  cmp w8, #5
  b.ne .LBB0_2
  mov w8, #6
  stxr w9, w8, [x0]     ; w9 holds the success state of the store operation
                        ; but we are not using it in this function
                        ; `ldxr` stores somewhere the [x0] value
                        ; and `stxr` compares it with the current [x0] value
                        ; making sure that the memory was not modified since the load
  ret
.LBB0_2:
  clrex                 ; clears the exclusive register since `stxr` was not
                        ; called and we need to clear the state that `ldxr` stored
  ret

Note that on CSIC there is no LL/SC loop since `lock cmpxchg` is implemented and is atomic.

example::compare_exchange_arm:
  mov ecx, 6
  mov eax, 5
  lock cmpxchg dword ptr [rdi], ecx
  ret

*/
pub fn compare_exchange_week_arm(x: &AtomicI32) {
    x.compare_exchange_weak(5, 6, Relaxed, Relaxed);
}

/*

Strong version of compare_exchange adds a loop that would make sure that the store
operation was successful. Still, as before, if the memory didn't hold the expected
value we short-circuit and exit the loop.

example::compare_exchange_arm:
  mov w8, #6
.LBB0_1:
  ldxr w9, [x0]
  cmp w9, #5
  b.ne .LBB0_4
  stxr w9, w8, [x0]
  cbnz w9, .LBB0_1
  ret
.LBB0_4:
  clrex
  ret

Note that on CSIC the code doesn't change, there is no week version of compare and exchange,
it can not fail since the is implemented in hardware and is atomic.

example::compare_exchange_arm:
  mov ecx, 6
  mov eax, 5
  lock cmpxchg dword ptr [rdi], ecx
  ret

 */
pub fn compare_exchange_arm(x: &AtomicI32) {
    x.compare_exchange(5, 6, Relaxed, Relaxed);
}