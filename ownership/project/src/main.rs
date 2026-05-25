fn main() {
    // Rust wont move overship here.
    // Bool has a Copy trait. Both is_concert and is_event can be accessed.
    let is_concert = true;
    let is_event = is_concert;

    // Same situation string slice (&str) has a Copy trait implemented.
    let sushi = "Salomon";
    let dinner = sushi;
}

fn eat_mean(mut meal: String) -> String {
    meal.clear();
    meal
}
