/// Implement a function that computes the difference between two lists.
/// The function should remove all occurrences of elements from the first list (a) that are present in the second list (b).
/// he order of elements in the first list should be preserved in the result.

/// Examples:
/// If a = [1, 2] and b = [1], the result should be [2].
/// If a = [1, 2, 2, 2, 3] and b = [2], the result should be [1, 3].


fn main() {
    let res = array_diff(vec![1, 2, 2, 2, 3], vec![2]);
    println!("{:#?}", res);
}

fn array_diff<T: PartialEq>(a: Vec<T>, b: Vec<T>) -> Vec<T> {
    let mut result = vec![];
    for num in a {
        if !b.contains(&num) {
            result.push(num);
        }
    }
    result
}
