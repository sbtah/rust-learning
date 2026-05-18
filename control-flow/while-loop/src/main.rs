fn main() {
    let mut attempts = 1;

    while attempts < 4 {
        println!("Retrying {attempts}, attempt...");
        attempts += 1;
    }
}
