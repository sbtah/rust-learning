fn main() {
    // String literals:
    // let _literal = "This is string literal, `&static' str`. String literals are know at compile time.";
    // let raw = r"This is a raw string literal, where each character is processeded without any special meanings like \n";
    // println!("{}", raw);

    // // Methods!
    // let value: i32 = -15;
    // println!("{}", value.abs());

    // let empty_space = "    my content      ";
    // println!("Trimmed {}.", empty_space.trim());

    // println!("{}", value.pow(3));

    // Floats
    let pi: f64 = 3.14159;

    println!("{}", pi.floor());
    println!("{}", pi.ceil());
    println!("{}", pi.round());
    println!("{pi:.2}");
}
