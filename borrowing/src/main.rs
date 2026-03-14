fn main() {
    let mut s1 = String::from("Hello");
    let len = calculate_length(&s1);  // we need to pass a reference as well `&`

    println!("The length of `{}` is {}", s1, len); // Here we still can access s1 and len.

    change_by_reference(&mut s1);
}


fn calculate_length(s: &String) -> usize {  // takes a read-only reference as an argument.
    // What will happen if we try to modify something we're borrowing:
    // Compiler will hint us that we can set this variable as mutable reference and function argument should be updated as well.
    s.push_str(", World!");

    s.len()
}

// Mutable references have one big restriction:
// If you have a mutable reference to a value, you can have no other references to that value.

fn change_by_reference(some_string: &mut String) {
    some_string.push_str(", World");
}


fn dangle() -> &String {
    let s = String::from("Hello,");
    &s
}