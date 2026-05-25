// Write an algorithm that takes an array and moves all of the zeros to the end, preserving the order of the other elements.

// moveZeros([false,1,0,1,2,0,1,3,"a"]) // returns[false,1,1,2,1,3,"a",0,0]

fn main() {
    let _test_array = [1, 2, 0, 1, 0, 1, 0, 3, 0, 1];
    let result = move_zeros(&_test_array);
    println!("Res {:?}", result);
}

fn move_zeros(arr: &[u8]) -> Vec<u8> {
    let mut sorted = vec![];

    for element in arr {
        if *element == 0 {
            continue;
        } else {
            sorted.push(*element);
        }
    }

    for element in arr {
        if *element == 0 {
            sorted.push(*element)
        } else {
            continue;
        }
    }
    sorted
}

// Code from codewars:
// use std::iter;

fn move_zeros_from_codewars(arr: &[u8]) -> Vec<u8> {
    use std::iter;
    arr.iter()
        .cloned()
        .filter(|&x| x != 0)
        .chain(iter::repeat(0))
        .take(arr.len())
        .collect()
}
