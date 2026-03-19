use std::collections::HashMap;


fn main() {

    let res = find_median();
    println!("res: {}", res);
    // let mut scores: HashMap<String, usize>= HashMap::new();

    // scores.insert(String::from("Blue"), 10);
    // scores.insert(String::from("Red"), 12);

    // println!("Scores: {:#?}", scores);

    // // Accessing value for a key in HashMap
    // let key_to_search = "Green";
    // let blue_score = scores.get(key_to_search);
    // match blue_score {
    //     Some(val) => println!("Score found: {}", val),
    //     None => {
    //         scores.insert(key_to_search.to_string(), 0);
    //         println!("Added new entry {}, with score of 0", key_to_search)
    //     }
    // }

    // // We can iterate over a HashMap!
    // for (key, value) in &scores {
    //     println!("K:{}  V:{}", key, value);
    // }


    // // HashMaps insert is overwrite a value by default.
    // scores.insert(String::from("Blue"), 2);
    // // println!("Scores: {:#?}", scores);


    // // Adding a Key and Value Only If a Key Isn’t Present
    // scores.entry(String::from("Yellow")).or_insert(50);
    // scores.entry(String::from("Blue")).or_insert(100);
    // println!("Scores: {:#?}", scores);
    // // The or_insert method on Entry is defined to return a mutable reference to the value
    // //  for the corresponding Entry key if that key exists,
    // // and if not, it inserts the parameter as the new value for this key and returns a mutable reference to the new value.
    // // This technique is much cleaner than writing the logic ourselves and, in addition, plays more nicely with the borrow checker.

    // // Updating a Value Based on the Old Value

    // let text = "hello world wonderful world";

    // let mut map = HashMap::new();

    // for word in text.split_whitespace() {
    //     let count = map.entry(word).or_insert(0);
    //     *count += 1;
    // }

    // println!("{map:?}");

}


fn find_median() -> usize {
    let mut unsorted_values = [1, 7, 5, 4, 3, 2, 6];

    unsorted_values.sort();
    let total = unsorted_values.len();

    if total % 2 == 0 {
        return (unsorted_values[total / 2 - 1] + unsorted_values[total / 2]) / 2;
    }
    return unsorted_values[total / 2];
}