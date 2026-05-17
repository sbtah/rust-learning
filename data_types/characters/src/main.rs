fn main() {
    // Rust character type represent a single UNICODE character.
    // UTF - Unicode Transformation Format.
    let first_initial: char = 'g';

    // Chars have some methods on them as well.
    println!(
        "{} {}",
        first_initial.is_alphabetic(),
        first_initial.is_uppercase()
    )
}
