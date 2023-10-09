use std::{rc::Rc, sync::Arc, thread};

fn main() {
    // a,b reference the same allocation but are different objects with different ownership
    let a = Rc::new([1, 2, 3]);
    let b = a.clone();

    assert_eq!(a.as_ptr(), b.as_ptr()); // Same allocation!

    // When a,b are moved to a thread, they are copied but the referenced data is not
    // When a thread drops an object it owns, the reference count is decremented
    // and the last thread will decrement it to 0 and would drop the allocation
    let a = Arc::new([1, 2, 3]);
    let b = a.clone();

    let at = thread::spawn(move || dbg!(a));
    let bt = thread::spawn(move || dbg!(b));
    at.join().unwrap();
    bt.join().unwrap();

    // To not name every clone of the same allocation we can shadow the same variable in a new scope {}
    let data = Arc::new([1, 2, 3]);
    let at = thread::spawn({ let data = data.clone(); move || { dbg!(data);} });
    let bt = thread::spawn({ let data = data.clone(); move || { dbg!(data);} });
    at.join().unwrap();
    bt.join().unwrap();
}
