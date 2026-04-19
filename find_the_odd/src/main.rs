/// Given an array of integers, find the one that appears an odd number of times.

/// There will always be only one integer that appears an odd number of times.
/// Examples

/// [7] should return 7, because it occurs 1 time (which is odd).
/// [0] should return 0, because it occurs 1 time (which is odd).
/// [1,1,2] should return 2, because it occurs 1 time (which is odd).
/// [0,1,0,1,0] should return 0, because it occurs 3 times (which is odd).
/// [1,2,2,3,3,3,4,3,3,3,2,2,1] should return 4, because it appears 1 time (which is odd).
// /// 
/// 
/// 
/// 
/// 

use std::collections::HashMap;


fn main() {
    let test_arr = [1,2,2,3,3,3,4,3,3,3,2,2,1];
    let res = find_odd(&test_arr);
    println!("res, {}", res);
}



fn find_odd(arr: &[i32]) -> i32 {
    let mut results: HashMap<i32, usize> = HashMap::new();
    for num in arr {
        let count = results.entry(*num).or_insert(0);
        *count += 1;
    }

    for (key, value) in results {
        if value % 2 == 1 {
            return key
        }
    }
    0
}