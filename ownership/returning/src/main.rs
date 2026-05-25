fn main() {
    let cake = bake_cake();
    println!("I now have a: {}", cake);

    let current_meal = String::new();
    add_flour(current_meal);
}

fn bake_cake() -> String {
    String::from("Chocolate cake")
}

fn add_flour(mut meal: String) {
    meal.push_str("Add Flour");
}
