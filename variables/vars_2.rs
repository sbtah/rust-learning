// Rust is strongly typed, but to define a variable we don't need to specify a type.
// Rust compiler can infer the data type of variable based on the value assigned.

// If you want to be explicit with data type here is the syntax:
let variable_name: data_type = value;

// TYPES:
// Integer type: i32 and u32 for signed and unsigned 32 bit integers.
// Floating point type: f32 and f64 : 32bit and 64bit floats
// Boolean type: bool
// Character type: char

// Rust also enforces that a variable be initialized before the value stored in it is read.

{
    // this won't compile
    let a;
    println!("{}", a); // error on this line.
    // reading the value of an uninitialized variable is a compile-time error.
}

{
    // this will compile
    let a;
    a = 128;
    println!("{}", a); // no error here
    // variable 'a' has an initial value.
}