fn main() {
    // without type annotation
    let greeting = ['H', 'e', 'l', 'l', 'o'];

    // with type annotation
    let pi: [i32; 10] = [1, 4, 1, 5, 9, 2, 6, 5, 3, 5];

    for character in greeting {
        print!("{}", character);
    }

    println!("\nPi: 3.1{}{}{}{}", pi[0], pi[1], pi[2], pi[3]);
}
