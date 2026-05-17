fn main() {
    // Tuple can store elements of different types.
    let employee: (&str, i32, &str) = ("Molly", 32, "marketing");

    // Destructuring.
    let (name, age, departament) = employee;

    println!("{:#?}", employee);
    println!("Employee {name}, age {age} from {departament} departament.");
}
