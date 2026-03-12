use std::io;

fn main() -> Result<(), std::num::ParseFloatError> {
    println!("Basic calculator");

    let mut first_number_input = String::new();
    let first_number: f64;
    println!("First number: ");
    io::stdin().read_line(&mut first_number_input).expect("Failed to read the first number!");
    
    match first_number_input.trim().parse::<f64>() {
        Ok(val) => first_number = val,
        Err(e) => return Err(e),
    }

    let mut second_number_input = String::new();
    let second_number: f64;
    println!("Second number: ");
    io::stdin().read_line(&mut second_number_input).expect("Failed to read the second number!");
    match second_number_input.trim().parse::<f64>() {
        Ok(val) => second_number = val,
        Err(e) => return Err(e),
    }

    println!("Calculation symbol:");
    let mut calculation_input = String::new();
    io::stdin().read_line(&mut calculation_input).expect("Failed to read the calculation symbol!");
    match calculation_input.trim() {
        "+" => println!("{} + {} = {}", first_number, second_number, first_number + second_number),
        "-" => println!("{} - {} = {}", first_number, second_number, first_number - second_number),
        "*" => println!("{} * {} = {}", first_number, second_number, first_number * second_number),
        "/" => println!("{} / {} = {}", first_number, second_number, first_number / second_number),
        "^" => println!("{} ^ {} = {}", first_number, second_number, first_number.powf(second_number)),
        "%" => println!("{} % {} = {}", first_number, second_number, first_number % second_number),
        _ => println!("Unsuported calculation type!")
    }

    // let result = first_number.parse::<i128>().unwrap() + second_number.parse::<i128>().unwrap();
    Ok(())
}
