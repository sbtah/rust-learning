fn main() {
    let res = factorial(5);
    println!("{res}");

    let fact = factorial_recursive(5);
    println!("{fact}");
}

fn color_to_number(color: &str) -> u32 {
    match color {
        "red" | "Red" | "RED" => 1,
        "green" | "Green" | "GREEN" => 2,
        "blue" | "Blue" | "BLUE" => 3,
        _ => 0,
    }
}

fn factorial(mut number: i64) -> i64 {
    for num in 1..number {
        number *= num;
    }
    number
}

fn factorial_recursive(number: i64) -> i64 {
    if number < 1 {
        return 1;
    }
    number * factorial_recursive(number - 1)
}
