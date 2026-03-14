fn main() {
    // Scope of variables:
    // `s` variable is not valid here, not yet declared!
    {
        let s = "hello";  // `s` is valid from this point.

        // We can do stuff with `s` here.
    } // the scope is over `s` is no longer valid

    // Strings are complex types allocated on the heap.
    let mut ss = String::from("Hello");

    // Strings can be mutated
    ss.push_str(", World!");

    println!("{}", ss);

    // Cloning data.
    // Heap data is copied, not just stack.
    let s1 = String::from("Hello");
    let s2 = s1.clone();



}


fn takes_ownership(some_string: String) { // some_string comes into scope
    println!("{}", some_string);
} // some_string goes out of scope, drop is called

fn makes_copy(some_integer: i32) { // some_integer comes into scope
    println!("{}", some_integer);
} // some_integer comes out of scope but since it'a simple type it was copied so nothing happens