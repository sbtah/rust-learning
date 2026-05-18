fn main() {
    let res = decrease(25);
    println!("Result: {}", res);
}

fn decrease(mut number: i32) -> i32 {
    println!("Current value of number: {number}");
    if number == 0 {
        return number;
    };

    number -= 1;

    return decrease(number);
}
