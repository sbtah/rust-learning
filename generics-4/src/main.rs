// Generics with structs.
#[derive(Debug)]
struct Point<T> {
    x: T,
    y: T,
}


impl<T> Point<T> {
    fn x(&self) -> &T {
        &self.x
    }
}
// Generics inside Enums:



fn main() {
    let col_1 = vec![34, 50, 25, 100, 65];
    let col_2 = vec![102, 34, 6000, 89, 54, 2, 43, 8];
    let col_3 = vec!['x', 'w', 'g', 'o', 'f', 'm', 'a'];

    let result_1: &i32 = largest(&col_1);
    let result_2: &i32 = largest(&col_2);
    let result_3 = largest(&col_3);

    println!("The largest number in 1 is {}", result_1);
    println!("The largest number in 2 is {}", result_2);
    println!("The largest number in 3 is {}", result_3);

    let p1 = Point {x: 1, y: 10};
    let p2 = Point {x: 2.0, y: 20.2};
}


fn largest<T: PartialOrd>(collection: &[T]) -> &T {
    let mut largest: &T = &collection[0];
    for item in collection {
        if item > largest {
            largest = item;
        }
    }
    largest
}
