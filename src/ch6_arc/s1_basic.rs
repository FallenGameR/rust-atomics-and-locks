use std::ops::Deref;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::fence;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release, AcqRel, SeqCst};
use std::ptr::NonNull;

struct ArcData<T> {
    ref_count: AtomicUsize,
    data: T,
}

pub struct Arc<T> {
    // NotNull not only makes sure statically the pointer is never null
    // but it also alows the compiler to optimize memory placement
    // and use the same size of data for both Arc and Option<Arc>.
    ptr: NonNull<ArcData<T>>,
}

// For structs with pointers and NonNull Rust drops Send and Sync
//
// Arc can be Send to another thread - it can be moved and that other thread
// will be the new owner of it and it would eventually have to drop it.
// - When Arc implements Send it means that the data can be moved
// to another thread and that thread then can drop it, so T must be Send.
// - When Arc implements Send it means that the data it stores
// (the reference) is also sent to another thread, so T must be Sync.
unsafe impl<T: Send + Sync> Send for Arc<T> {}
// Also since &Arc can be cloned into Arc, the Arc need to implement Sync.
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            // Box::leak gives up exclusive ownership of the allocation
            // It returns the & mut and instructs the compiler not to call drop on it
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                ref_count: AtomicUsize::new(1),
                data,
            }))),
        }
    }

    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    // Since this method takes & mut we ensure that no other thread
    // would be able to clone Arc (Clone requires &) while we are
    // exclusivelly holding the & mut T reference with the same
    // lifetime as Arc.
    //
    // We don't use self here and the caller would need to use the
    // Arc::get_mut(&mut a) syntax. That is Rust convention for
    // types that implement Deref since otherwise it would be
    // easy to mistake get_mut call on Arc vs T that is holds.
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            // Safety: Nothing else can access the data, since
            // there's only one Arc, to which we have exclusive access.
            unsafe { Some(&mut arc.ptr.as_mut().data) }
        } else {
            None
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // Increment the reference count in a thread safe way
        let new_count = self.data().ref_count.fetch_add(1, Relaxed);

        // Fast way to handle possible overflows with an abort.
        // It is impossible to have usize::MAX / 2 threads running concurrently.
        //
        // Relaxed is ok to use here since we don't have operations
        // on other variagbles that got to happen strictly before
        // or after this += operation and the Relaxed ordering
        // would observe the same modification order anyway.
        if new_count > usize::MAX / 2 {
            std::process::abort();
        }

        Arc {
            ptr: self.ptr,
        }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        // Last decrement that gets the value to 0 means we need to drop the Arc
        // We can't use Relaxed here since we need to ensure that all concurrent
        // drops (that just did -- and ended through the if statement) have
        // already happened before the last drop.
        if self.data().ref_count.fetch_sub(1, Release) == 1 {
            // Strictly speaking all decrements except the last one need to use Release.
            // But the very last decrement needs to use the Acquire.
            // We optimized this a bit with doing all decrements with Release
            // and adding an explicit Aquire fence to ensure the ordering.
            //
            // Alternativelly we could have used AcqRel ordering for all decrements
            // and don't use the fence. But Mara Bos says that this is more
            // effient this way.
            //
            // All release operations need to finish strictly before this fence.
            fence(Acquire);

            unsafe {
                // Box::from_raw reverses Box::leak we did in ctor
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

#[test]
fn test() {
    static NUM_DROPS: AtomicUsize = AtomicUsize::new(0);

    struct DetectDrop;

    impl Drop for DetectDrop {
        fn drop(&mut self) {
            NUM_DROPS.fetch_add(1, Relaxed);
        }
    }

    // Create two Arcs sharing an object containing a string
    // and a DetectDrop, to detect when it's dropped.
    let x = Arc::new(("hello", DetectDrop));
    let y = x.clone();

    // Send x to another thread, and use it there.
    let t = std::thread::spawn(move || {
        assert_eq!(x.0, "hello");
    });

    // In parallel, y should still be usable here.
    assert_eq!(y.0, "hello");

    // Wait for the thread to finish.
    t.join().unwrap();

    // One Arc, x, should be dropped by now.
    // We still have y, so the object shouldn't have been dropped yet.
    assert_eq!(NUM_DROPS.load(Relaxed), 0);

    // Drop the remaining `Arc`.
    drop(y);

    // Now that `y` is dropped too,
    // the object should've been dropped.
    assert_eq!(NUM_DROPS.load(Relaxed), 1);
}
