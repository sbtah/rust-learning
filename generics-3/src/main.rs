struct Point<T> {
    x: T,
    y: T,
}

impl<T> Point<T> {
    fn x(&self) -> &T {
        &self.x
    }
}



fn main() {
    let numbers_1 = vec![34, 50, 25, 100, 65];
    let numbers_2 = vec![102, 34, 6000, 89, 54, 2, 43, 8];
    let char_list = vec!['y', 'm', 'a', 'q'];

    let largest_1 = largest(&numbers_1);
    let largest_2 = largest(&numbers_2);
    let largest_char = largest(&char_list);

    println!("The largest number for 1 is: {}", largest_1);
    println!("The largest number for 2 is: {}", largest_2);
    println!("The largest char is: {}", largest_char);

    let integer_point = Point { x: 1, y: 2 };
    let float_point = Point { x: 2.0, y: 3.0 };

    println!("POINT X:{:#?}", float_point.x());
}


fn largest<T: PartialOrd>(collection: &[T]) -> &T {
    let mut largest = &collection[0];

    for number in collection {
        if number > largest {
            largest = number
        }
    }
    largest

}
