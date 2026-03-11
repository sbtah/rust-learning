use std::io::{self, Read};


fn main() {
    

    println!("What is your name?: ");
    let mut name_answer = String::new();
    io::stdin().read_line(&mut name_answer).expect("Failed to read the name line!");

    println!("What is your age?:");
    let mut age_answer: String = String::new();
    io::stdin().read_line(&mut age_answer).expect("Failed to read the age line!");

    println!("What is your favourite colour?:");
    let mut color_answer: String = String::new();
    io::stdin().read_line(&mut color_answer).expect("Failed to read the color line!");

    print!(
        "Nice to meet you {}.\
        I understand that you're {} years old and your favourite color is {}",
        name_answer,
        age_answer,
        color_answer,
    );
}
