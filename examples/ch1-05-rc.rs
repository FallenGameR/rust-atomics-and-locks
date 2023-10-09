use std::{rc::Rc, sync::Arc, thread};

fn main() {
    let a = Rc::new([1, 2, 3]);
    let b = a.clone();

    assert_eq!(a.as_ptr(), b.as_ptr()); // Same allocation!

    let a = Arc::new([1, 2, 3]);
    let b = a.clone();

    let at = thread::spawn(move || dbg!(a));
    let bt = thread::spawn(move || dbg!(b));
    at.join().unwrap();
    bt.join().unwrap();
}
