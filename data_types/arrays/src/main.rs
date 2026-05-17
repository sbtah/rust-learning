fn main() {
    // Arrays - compound type.
    // Fixed! size collection of homogenous data - same type.
    let numbers: [i32; 5] = [4, 8, 16, 32, 64];

    // Mutabiliy for arrays, allows us to replace elements.
    let mut vegtebles: [&str; 3] = ["Onion", "Cucumber", "Salad"];
    vegtebles[0] = "Potato";

    println!(
        "We have {} numbers and {} vegtebles",
        numbers.len(),
        vegtebles.len()
    );

    let first_number = numbers[0];
    // Rust compiler will panic if we try to access invalid index in array.
}

#[derive(Debug)]
struct Person {
    name: String,
}
