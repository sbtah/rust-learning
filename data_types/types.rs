// Rust have 2 main data types :
// Scalar data type: Types that store only a single value.
// Compound data type: Types that store multiple values even of different types.

// Scalar data types:
// Integers: Stores whole numbers. Has sub-types for each specific use case.
// Floats: Stores numbers with a fractional value. Has two sub-types based on size.
// Characters: Stores a single character of UTF-8 encoding
// Booleans: Stores either a true or a false.


// INTEGERS:
// 2 types: signed and unsigned
// Unsigned store only 0 and positive numbers
// Signed can store 0, positive and negative numbers

// Available lengths : 8bit, 16, 32, 64, 128


fn main() {
    let bin_value = 0b100_0101; // use prefix '0b' for binary values.
    let oct_value = 0o105; // prefix '0o' for octals.
    let hex_value = 0x45; // use prefix '0x' for Hexadecimals.
    let dec_value = 1_000_000; // split long integers.

    println!("bin_value: {bin_value}");
    println!("oct_value: {oct_value}");
    println!("hex_value: {hex_value}");
    println!("dec_value: {dec_value}");
}
