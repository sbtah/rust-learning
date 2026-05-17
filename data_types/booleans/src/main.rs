fn main() {
    let is_handsome: bool = true;
    let is_silly: bool = true;

    let age: i32 = 18;
    let is_young: bool = age < 35;

    println!("{}", is_young);
    println!("{}, {}", age.is_positive(), age.is_negative());

    // Inverting a boolean:
    let sexy: bool = !true;
    println!("Iam sexy: {}", sexy);

    // let age = 1
    let can_see_rated = age >= 10;
    let cannot_see_rated = !can_see_rated;

    // Equality operator:
    println!("{}", "Coke" == "Pepsi");
    println!("{}", "Coke" != "Pepsi");

    // Logical && (and) Operator.
    let purchased_ticked: bool = false;
    let plane_on_time: bool = true;
    // Rust is doing short circuit, meaning if first argument before && is false it wont event check the 2nd.
    let event_possible: bool = purchased_ticked && plane_on_time;
    println!("It is {} that I will arrive as expected.", event_possible);

    // Logiacl || (or) Operator.
    let user_paid_for_subscription = false;
    let user_is_admin: bool = false;
    let user_can_watch_content: bool = user_paid_for_subscription || user_is_admin;
    println!("Can this user see the content?: {}", user_can_watch_content);
}
