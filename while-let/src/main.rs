fn main() {
    let mut fruits: Vec<&str> = vec!["apple", "banana", "cherry"];
    drain_a_vector(&mut fruits);
    println!("Current state of vector: {:#?}", fruits);
}


fn drain_a_vector(collection: &mut Vec<&str>) {
    while let Some(fruit) = collection.pop() {
        println!("Found a fruit!: {}", fruit);
    }
}