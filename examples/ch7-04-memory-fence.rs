/*
https://godbolt.org/
rustc 1.73
-O
-O --target=aarch64-unknown-linux-musl
*/
use std::sync::atomic::Ordering::{Acquire, Release, AcqRel, SeqCst};


pub fn main(){
}

// there is no such thing as a relaxed fence
// pub fn fence_relaxed() {
//     std::sync::atomic::fence(Relaxed);
// }


/*

AMD64:

    example::fence_acquire:
        ret

ARM64:

    example::fence_acquire:
        dmb ishld               ; data-memory-barrier inner-shared-domain LOAD
        ret

*/
pub fn fence_acquire() {
    // Prevents past load operations to be reordered with future any (load and store) operations
    std::sync::atomic::fence(Acquire);
}

/*

AMD64:

    example::fence_release:
        ret

ARM64:

    example::fence_release:
        dmb ish
        ret

*/
pub fn fence_release() {
    // Prevents past any (load and store) operations to be reordered with future store operations
    std::sync::atomic::fence(Release);
}

/*

AMD64:

    example::fence_acqrel:
        ret

ARM64:

    example::fence_acqrel:
        dmb ish
        ret

*/
pub fn fence_acqrel() {
    std::sync::atomic::fence(AcqRel);
}

/*

AMD64:

    example::fence_seqcst:
        mfence
        ret

ARM64:

    example::fence_seqcst:
        dmb ish                 ; data-memory-barrier inner-shared-domain
        ret

*/
pub fn fence_seqcst() {
    // Prevents past any (load and store) operations to be reordered with future any (load and store) operations
    std::sync::atomic::fence(SeqCst);
}

/*

On x64 we get release and acquire fences for free due to the strong ordering.
SeqCst emmits the mfence instruction that makes sure that all memory operations
that started before it would be completed after the CPU proceeds to the next instruction.

On ARM we have a special instructions emmited `dmb ish` that is equivalent to mfence.
For Acquire fence this operation is relaxed to `dmb ishld` since Aquire cares
only about load operations non-reordering. `dmb ishld` waits only for the load
operations to complete before proceeding to the next ones.

*/