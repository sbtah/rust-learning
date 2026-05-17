fn main() {
    open_store("Severs");
    bake_pizza(1, "Cheese");
    count_profit();
    open_store("Tatooine");
    block_definition();
}

fn open_store(neighborhood: &str) -> () {
    println!("Opening my pizza store in the {}.", neighborhood);
}

fn bake_pizza(number: i32, topping: &str) -> () {
    println!(
        "Baking {} {} pizza{}!",
        number,
        topping,
        if number > 1 { "s" } else { "" }
    );
}

fn count_profit() -> () {
    println!("Counting DOLLARZ!");
}

fn square(number: usize) -> usize {
    number.pow(2)
}

fn block_definition() {
    let multiplier = 3;

    // Isoleted scope:
    let calculation = {
        // Inner scope has access to outer scope.
        let value = 5 + 4;
        value * multiplier
    };

    println!("Calculation for inner block is {}", calculation);
}
