fn main() {
    let slice = "pasta";
    let from_slice = String::from("New String");
    let mut new = String::new();

    println!("New: {}", new);
    new.push_str("Hello World");
    println!("New: {}", new);
}
