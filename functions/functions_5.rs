// Returning multiple values.
// You can have a fuincion that return multiple values of many types.

fn main() {
    let (maths, english, science, chemistry) = tuple_func();
    println!("Student obtained in Maths: {maths}");
    println!("Student obtained in English: {english}");
    println!("Student obtained in Science: {science}");
    println!("Student obtained in Chemistry: {chemistry}");
}

fn tuple_func() -> (f64, f64, f64, f64) {
    // return marks for student
    let maths = 84.5;
    let english = 85.0;
    let science = 75.5;
    let chemistry = 67.25;

    (maths, english, science, chemistry) // return via expression
}

