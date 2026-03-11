use std::ops::Add;  // Bring Add trait into local scope.
use std::time::Duration;


fn add<T: Add<Output = T>>(i: T, j: T) -> T { // The arguments to add can accept any type with Add Trait.
    i + j
}


fn main() {
    let floats = add(1.2, 3.4);

    let ints = add(10, 20);

    // Calls add with Durations values. 
    let durations = add(Duration::new(5, 0), Duration::new(10, 0));


    println!("{}", floats);
    println!("{}", ints);
    println!("{:#?}", durations);


}
