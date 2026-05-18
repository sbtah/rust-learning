fn main() {
    // Iteration with loop keyword.
    let mut start = 21;
    loop {
        println!("Running {}", start);
        if start <= 0 {
            break;
        }
        if start % 2 == 0 {
            start -= 3;
            continue;
        }
        start -= 1;
    }
}
