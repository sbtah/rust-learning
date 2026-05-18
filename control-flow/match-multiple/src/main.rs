fn main() {
    let number = 8;

    match number {
        2 | 4 | 6 | 8 => println!("Number is even"),
        1 | 3 | 5 => println!("Number is odd"),
        _ => println!("Hmm!"),
    }

    match number {
        value if value % 2 == 0 => println!("Value {value} is even"),
        value if value % 2 == 1 => println!("Value {value} is odd"),
        _ => unreachable!(),
    }
}

// fn main() {
//     let number = 8;
//     match number {
//         n if n % 2 == 0 => println!("{n} is even"),
//         _ => println!("{number} is odd"),
//     }
// }
