use std::rc::Rc;
use std::sync::{Arc, Mutex};


fn main() {
    let a = 10; // Integer on the stack.
    let b = Box::new(20); // Integer on the heap, also known as boxed Integer.
    let c = Rc::new(Box::new(30)); // Boxed integer warapped in reference counter.
    let d = Arc::new(Mutex::new(40)); // Integer wrapped in atomic reference counter. protected by a lock.

    println!("a: {:#?}, b: {:#?}, c: {:#?}, d: {:#?}", a, b, c, d);
}
