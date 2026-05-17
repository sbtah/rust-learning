fn main() {
    apply_to_jobs(32, "Rust Developer");

    println!("{}", is_even(8));
    println!("{}", is_even(9));

    println!("{:?}", alphabets("aardvark"));
}

fn apply_to_jobs(number: u32, title: &str) -> () {
    println!("I'm applying to {number} {title} jobs");
}

fn is_even(number: i32) -> bool {
    number % 2 == 0
}

fn alphabets(text: &str) -> (bool, bool) {
    (text.contains("a"), text.contains("z"))
}
