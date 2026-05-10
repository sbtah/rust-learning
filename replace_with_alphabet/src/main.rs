// Welcome.

// In this kata you are required to, given a string, replace every letter with its position in the alphabet.

// If anything in the text isn't a letter, ignore it and don't return it.

// "a" = 1, "b" = 2, etc.
// Example

// Input = "The sunset sets at twelve o' clock."
// Output = "20 8 5 19 21 14 19 5 20 19 5 20 19 1 20 20 23 5 12 22 5 15 3 12 15 3 11"
use std::collections::HashSet;

fn main() {
    let input = "The sunset sets at twelve o' clock.";
    let result = alphabet_position(input);
    println!("Result: {}", result);
}


fn alphabet_position(text: &str) -> String {
    let alphabet = "abcdefghijklmnopqrstuvwxyz";
    let mut new: Vec<String> = vec![];
    for ch in text.chars() {
        if alphabet.contains(ch.to_ascii_lowercase()) {
            let dx = alphabet.find(ch.to_ascii_lowercase()).unwrap();
            new.push(format!("{}", dx + 1));
        }
    }
    new.join(" ")
}