fn main() {
    let to_test = String::from("Hello World my little puppies!");

    first_word(&to_test);

    let res = first_word(&to_test);

    println!("result: {}, original: {}", res, to_test);
}



fn get_first_word(some_string: &String) -> &str {
    let result = some_string.split_once(' ').unwrap().0;  // `split_once`` gives Option with tuple of results, we can unwrap and use `.0` Indexing to access 1st element.
    result
}

// Different implementation:
fn first_word(some_string: &str) -> &str {
    let bytes = some_string.as_bytes();

    // println!("DEBUG BYTES: {:#?}", bytes);

    for (idx, &item) in bytes.iter().enumerate() {
        if item == b' ' {
            return &some_string[0..idx];
        }
    }
    &some_string[..]
}