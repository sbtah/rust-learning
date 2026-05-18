fn main() {
    if 2 == 2 {
        println!("This will run");
    }

    if !2 == 2 {
        println!("This will not run!")
    }

    let season = "summer";

    if season == "summer" {
        println!("School is out!");
    } else if season == "winter" {
        println!("Santa time!");
    } else if season == "sprint" {
        println!("It's getting warmer");
    } else if season == "fall" {
        println!("Getting colder");
    } else {
        println!("Not a valid season.")
    }

    even_or_odd(17);
}

fn even_or_odd(number: i32) -> &'static str {
    let result = if number % 2 == 0 { "Even" } else { "Odd" };
    println!("The number is {result}");
    result
}
