// Functions in rust dont have an option for default arguments.
// Populating all parameters is compulsory

fn main() {
    food(2, 4);
}

fn food(bananas: i32, apples: i32) {
    println!("I am hungry... I need {bananas} bananas and {apples} apples!");
}
