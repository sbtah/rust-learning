// Returning data with functions
// Rust functions can either return via return keyword or by using expression instead of statement.

fn main() {
    println!("If I but 2 kilogram of apples from fruit vendor, I have to pay: {}", retail_price(2.0));
    println!("But if I buy 30 kilograms of apples from a fruit vendorm, I have to pay: {}", wholesale_price(30.0));
}

// This function returns a value via return keyword.
fn retail_price(weight: f64) -> f64 {
    return weight * 500.0;
}

// But this one returns via expression

fn wholesale_price(weight: f64) -> f64 {
    weight * 400.0
}

// Statement is a line of code that ends with a semicolon and does not evaluate to some value.
// Expression is a line of code that does not end with semicolon and evaluates to some value.