/// Copy trait
/// If type has implemented a `Copy` Trait, it means variable of this type cam be copied.
/// This will result in creation of full duplicate of variable.

// Basic types, like bools, ints, chars have Copy trait.
// These types live on stack and can be easily copied.
fn main() {
    // Here we will copy, since time is an integer and has a copy.
    let time = 2025;
    let year = time;

    // We can check memory address with :p
    // time: value = 2025, address = 0x7ffd76f8f550
    // year: value = 2025, address = 0x7ffd76f8f554
    println!("time: value = {}, address = {:p}", time, &time);
    println!("year: value = {}, address = {:p}", year, &year);
}
