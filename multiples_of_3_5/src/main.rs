// If we list all the natural numbers below 10 that are multiples of 3 or 5, we get 3, 5, 6 and 9. The sum of these multiples is 23.

// Finish the solution so that it returns the sum of all the multiples of 3 or 5 below the number passed in.

// Additionally, if the number is negative, return 0.

// Note: If a number is a multiple of both 3 and 5, only count it once.

fn main() {
    let test = 10;
    let result = solution(test);
    println!("Result for {} is {}", test, result);
}

fn solution(num: i32) -> i32 {
    if num <= 0 {
        return 0;
    };

    let mut to_multiply = vec![];
    for checked in 0..num {
        if checked % 3 == 0 && checked % 5 == 0 {
            to_multiply.push(checked);
            continue;
        } else if checked % 5 == 0 {
            to_multiply.push(checked);
            continue;
        } else if checked % 3 == 0 {
            to_multiply.push(checked);
            continue;
        }
    }
    to_multiply.iter().sum()
}

// From codewars:
fn solution_1(num: i32) -> i32 {
    (1..num).filter(|x| x % 3 == 0 || x % 5 == 0).sum()
}
