use std::cell::UnsafeCell;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::atomic::fence;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::{Acquire, Relaxed, Release};

// Arc doesn't depend on Week.
// But ArcData is aware of both.
pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Sync + Send> Send for Arc<T> {}
unsafe impl<T: Sync + Send> Sync for Arc<T> {}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Sync + Send> Send for Weak<T> {}
unsafe impl<T: Sync + Send> Sync for Weak<T> {}

// The data that both Arc and Week share and modify.
// Week can be created only from an Arc.
struct ArcData<T> {
    /// Number of `Arc`s.
    data_ref_count: AtomicUsize,

    /// Number of `Weak`s, plus one if there are any `Arc`s.
    ///
    /// That is the optimization implemented to not pay the additional
    /// cost when you are using Arc without Weak. Cloning an Arc doesn't
    /// need to touch this counter at all. Only dropping the very
    /// last Arc would decrements this pointer too.
    ///
    /// When getting the &mut t_data we would need to view the state of
    /// both alloc_ref_count and data_ref_count at the same time. To do that
    /// we would store usize::MAX in alloc_ref_count.
    alloc_ref_count: AtomicUsize,

    /// The data. Dropped if there are only weak pointers left.
    /// ManuallyDrop is smaller than Option (ManuallyDrop has 0 cost,
    /// it is an instruction to compiler not to call drop).
    ///
    /// Previously we relied on Option = None to tell that there are
    /// on Arcs left. But here we can rely on the data_ref_count and
    /// alloc_ref_count.
    t_data: UnsafeCell<ManuallyDrop<T>>,
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Arc<T> {
        Arc {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                // The additional cost for the first Arc.
                // Upgraded Weak pointers will not increment it.
                // And the last Arc will decrement it by explicitly
                // calling drop on the ArcData wrapped in a Week
                // struct.
                //
                // Note that there is no Week::new method that
                // would do something similar to Arc::new.
                alloc_ref_count: AtomicUsize::new(1),
                data_ref_count: AtomicUsize::new(1),
                t_data: UnsafeCell::new(ManuallyDrop::new(data)),
            }))),
        }
    }

    fn arc_data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    // Here is the kind of code we would like to be able to handle:
    //
    // let mut arc = Arc::new(42);
    //
    // // Unclear how this is possible, but it's like the author
    // // assumes that this is possible and it does happen.
    // pass_arc_to_another_thread(&arc);
    //
    // loop {
    //      let week = Arc::downgrade(&arc);
    //      drop(arc);
    //      println!("There is no Arc, only Week");
    //
    //      arc = week.upgrade().unwrap();
    //      drop(week);
    //      println!("There is no Week, only Arc");
    // }
    //
    // Let's say that pass_arc_to_another_thread doesn't happen
    // and we just cycle through Arc and Week that operate on
    // the same ArcData. Internally they would need to deal
    // wtith two atomic values: alloc_ref_count and data_ref_count.
    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        // Acquire matches Weak::drop's Release decrement, to make sure any
        // upgraded pointers are visible in the next data_ref_count.load.
        //
        // Locking operations uses the Acquire operation.
        if arc
            .arc_data()
            .alloc_ref_count
            .compare_exchange(1, usize::MAX, Acquire, Relaxed)
            .is_err()
        {
            return None;
        }

        // We could race with:
        // - Arc.downgrade - but it knows about usize::MAX convention and
        // it would spin loop until some other value would be stored here.
        // - Week.clone and Week.drop - but we just observed
        // alloc_ref_count == 1 meaning and we are inside of an Arc method,
        // so it means there were no other week references just now. The
        // only way to have a Week reference at this point is to downgrade
        // the current Arc in another thread. And that is possible only
        // though the Arc.downgrade method that knows that we are still
        // doing our work here.
        //
        // Since we have &mut Arc access here we can assume no other method
        // would be called on it, Clone and Drop methods included.
        // So the next check seems to be redundant.
        let is_unique = arc.arc_data().data_ref_count.load(Relaxed) == 1;

        // Release matches Acquire increment in `downgrade`, to make sure any
        // changes to the data_ref_count that come after `downgrade` don't
        // change the is_unique result above.
        //
        // Unlocking operation uses the Release operation.
        arc.arc_data().alloc_ref_count.store(1, Release);
        if !is_unique {
            // Would this line ever be hit?
            //
            // The original comments imply that is a guard against concurrent
            // downgrade calls, but I don't see how that could happen since
            // we get exclusive reference to the Arc here.
            //
            // The author also says that Arc is one of the most complex
            // cases of working with memory ordering correctly. I just
            // hope that they tested the shit out of the std Arc implementation.
            return None;
        }

        // Acquire to match Arc::drop's Release decrement, to make sure nothing
        // else is accessing the data.
        fence(Acquire);
        unsafe { Some(&mut *arc.arc_data().t_data.get()) }
    }

    pub fn downgrade(arc: &Self) -> Weak<T> {
        let mut n = arc.arc_data().alloc_ref_count.load(Relaxed);
        loop {
            if n == usize::MAX {
                // Using a spin lock is a valid strategy that deals
                // with mutiple atomic variables at once.
                std::hint::spin_loop();
                n = arc.arc_data().alloc_ref_count.load(Relaxed);
                continue;
            }
            assert!(n <= usize::MAX / 2);
            // Acquire synchronises with get_mut's release-store.
            if let Err(e) =
                arc.arc_data()
                    .alloc_ref_count
                    .compare_exchange_weak(n, n + 1, Acquire, Relaxed)
            {
                n = e;
                continue;
            }
            return Weak { ptr: arc.ptr };
        }
    }
}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: Since there's an Arc to the data,
        // the data exists and may be shared.
        unsafe { &*self.arc_data().t_data.get() }
    }
}

impl<T> Weak<T> {
    fn arc_data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        // Note that we don't modify alloc_ref_count here anymore.

        let mut n = self.arc_data().data_ref_count.load(Relaxed);
        loop {
            if n == 0 {
                return None;
            }
            assert!(n <= usize::MAX / 2);
            if let Err(e) =
                self.arc_data()
                    .data_ref_count
                    .compare_exchange_weak(n, n + 1, Relaxed, Relaxed)
            {
                n = e;
                continue;
            }
            return Some(Arc { ptr: self.ptr });
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        // Relaxed is said to be ok here. It is unclear why since drop is using Release
        // and if we need to sync drop and get_mut why we shouldn't sync get_mut and clone?
        //
        // Found the following passage that addresses this question: in the chapter summary
        // the author says that incrementing the reference counter can be done in a relaxed way,
        // but the **final** decrement must be syncronized with all the previous decrements.
        if self.arc_data().alloc_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Weak { ptr: self.ptr }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        // Release here is needed to coordinate with get_mut's Acquire load.
        if self.arc_data().alloc_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        // All this optimization is needed to skip the following commented line
        // that invloves another atomic operation. Atomic operations are costly.
        //let weak = self.weak.clone();

        if self.arc_data().data_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            std::process::abort();
        }
        Arc { ptr: self.ptr }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.arc_data().data_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);

            // Safety: The data reference counter is zero,
            // so nothing will access the data anymore.
            //
            // But what if somebody works with t_data from a week pointer
            // while we are dropping it here? It is not possible because
            // we are the last Arc and we are dropping it. And to work
            // with the data via a weak pointer you need to upgrade it
            // to Arc first.
            //
            // This drops the t_data.
            unsafe {
                ManuallyDrop::drop(&mut *self.arc_data().t_data.get());
            }

            // Now that there's no `Arc<T>`s left,
            // drop the implicit weak pointer that represented all `Arc<T>`s.
            // That would cause Weak::drop to be called and alloc_ref_count to be decremented.
            //
            // This drops the ArcData.
            drop(Weak { ptr: self.ptr });
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

    // Create an Arc with two weak pointers.
    let x = Arc::new(("hello", DetectDrop));
    let y = Arc::downgrade(&x);
    let z = Arc::downgrade(&x);

    let t = std::thread::spawn(move || {
        // Weak pointer should be upgradable at this point.
        let y = y.upgrade().unwrap();
        assert_eq!(y.0, "hello");
    });
    assert_eq!(x.0, "hello");
    t.join().unwrap();

    // The data shouldn't be dropped yet,
    // and the weak pointer should be upgradable.
    assert_eq!(NUM_DROPS.load(Relaxed), 0);
    assert!(z.upgrade().is_some());

    drop(x);

    // Now, the data should be dropped, and the
    // weak pointer should no longer be upgradable.
    assert_eq!(NUM_DROPS.load(Relaxed), 1);
    assert!(z.upgrade().is_none());
}
