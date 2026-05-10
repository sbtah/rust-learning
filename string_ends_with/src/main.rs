// Complete the solution so that it returns true if the first argument(string) passed in ends with the 2nd argument (also a string).

// Examples:

// Inputs: "abc", "bc"
// Output: true

// Inputs: "abc", "d"
// Output: false




fn main() {
    println!("Hello, world!");
}


fn solution(word: &str, ending: &str) -> bool {
    word.ends_with(ending)
}