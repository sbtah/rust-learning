use std::collections::HashMap;

const TEST_STR: &str = "hello my darling hello my honey hello my dearest friend";


fn main() {
    let result = count_words(TEST_STR);

    println!("Counting result {:#?}", result);
}



fn count_words(sentence: &str) -> HashMap<&str, u32> {

    let mut counter: HashMap<&str, u32> = HashMap::new();

    for word in sentence.split_whitespace() {
        let count = counter.entry(word).or_insert(0);
        *count += 1;
    }
    return counter;
}