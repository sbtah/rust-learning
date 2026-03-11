use std::convert::TryInto;  // Enables a try_into() to be called.


fn main() {
    let a: i32 = 10;
    let b: u16 = 100;

    let b_new: i32 = b.try_into().unwrap();

    if a < b_new {
        println!("Ten is less than Hundred!");
    }
}
