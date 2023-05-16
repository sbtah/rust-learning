// Floats
// Have 2 types: f32 - single precision type, and f64 - doouble precision type.
// Both are Signed types.


fn main() {
    let pi: f32 = 3.1400; // f32
    let golden_ratio = 1.610000; // f64 is default
    let five = 5.00; // decimal point indicates that it must be inferred as a float.
    let six: f64 = 6.; // even the type is annotated here , a decimal point is still necessary

    println!("pi: {pi}");
    println!("golden ratio: {golden_ratio}");
    println!("five: {five}");
    println!{"six {six}"};
}