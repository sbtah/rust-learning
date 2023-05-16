// Constants are always immutable
// Compiler requires a data type for contant
// Constants are mostly used in global scope.


fn main() {
    const PI: f64 = 3.14;
    let radius: f64 = 50.0;

    let circle_area = PI * (radius * radius);
    let circle_perimeter = 2.0 * PI * radius;

    println!("There is a circle with radius of {radius} centimeters.");
    println!("It's area is {} centimetre square.", circle_area);
    println!("And it's circumference of {} centimetres.", circle_perimeter);
}