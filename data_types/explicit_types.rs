fn main() {
    let a = 3 as f64; // float 64
    let b = 3.14159265359 as i32; // this is `lossy` meaning the fractional element is lost.

    println!("a: {a}");
    println!("b: {b}");
}
