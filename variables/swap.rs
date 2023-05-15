fn main() {
    let mut a: &str = "71830";
    let mut b: &str = "03817";
    println!("a: {}, b: {}", a, b);

    // swap the values
    let temp = a;
    a = b;
    b = temp;
    println!("a: {a}, b: {b}");
}