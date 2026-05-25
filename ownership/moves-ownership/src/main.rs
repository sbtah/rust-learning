fn main() {
    let year = String::from("2025");
    // Clone method:
    // clone() method is actually a requirement of a Clone trait.
    let best_year = year.clone();

    // This works because with cloned the value.
    // Otherwise we would get a compile error that year was moved to best_year.
    println!("Year binding: {}", year);

    // Drop function:
    // Deallocates memory on the heap.
    // Rust automatically calls drop() on the end of the code block.
    // Passing each variable from the scope one by one. (Stack memory does not work with drop at all)
    let person = String::from("Joe Biden");
    drop(person);
    // This will fail.
    // println!("No more {}", person);
}
