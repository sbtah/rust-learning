// Tuples also have fixed length.
// Elements can be of same/different Scalar data types.
// The tuple is stored on stack ie faster access.

// Example:

fn main() {
    let a = (38, 923.329, true);
    let b: (char, i32, f64, bool) = ('r', 43, 3.14, false);

    println!("a.0: {}, a.1: {}, a.2: {}", a.0, a.1, a.2);
    println!("b.0: {}, b.1: {}, b.2: {}, b.3: {}", b.0, b.1, b.2, b.3);

    // Desctructuring a Tuple
    let pixel = (50, 0, 200);
    let (red, green, blue) = pixel;
    println!("red: {}, green: {}, blue: {}", red, green, blue);
}