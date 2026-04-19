use std::collections::HashMap;


fn main() {
    let test_arr = [1,2,2,3,3,3,4,3,3,3,2,2,1];
    find_odd(&test_arr);
}



fn find_odd(arr: &[i32]) -> i32 {
    let mut results: HashMap<i32, usize> = HashMap::new();
    for num in arr {
        let count = results.entry(*num).or_insert(0);
        *count += 1;
    }
    println!("Results: {:#?}", results);
    1
}