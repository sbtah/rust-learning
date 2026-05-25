/*
String - A dynamic piece of text stored on the heap at runtime.

&String = Reference to the String living on the heap.

str = A hardcoded, read only piece of text, encoded in the binary.

&str = A reference to the text in the memory that has loaded the binary file.
*/

// Copy Trait with reference.
fn main() {
    let ice_cream: &str = "Cookies and Cream";
    let desert = ice_cream;

    println!("{}, {}", ice_cream, desert);

    // apples variable will never transfer an ownership to the value.
    // Integer is Copy.!
    let apples = 6;
    print_my_value(apples);
    // apples still valid!
    println!("{}", apples);

    // We will transfer an ownership here.
    let oranges = String::from("Oranges 123!");
    print_my_message(oranges);
    // println!("{}", oranges); - This won't compile!

    let burger = String::from("Burger");
    let new_food = add_fries(burger);
    println!("New meal is: {}", new_food);
}

fn print_my_value(value: i32) {
    println!("Your value is: {}", value);
}

fn print_my_message(string: String) {
    println!("Your message is: {}", string);
}

// Function parameters are immutable by default.
fn add_fries(mut meal: String) -> String {
    meal.push_str(" with Fries");
    meal
}
