fn main() {
    let search_term = "picture";


    // Multiline strings does not require a special syntax in Rust.
    // Only new character has to be escaped.\
    let quote = "\
    Every face, every shop, bedroom window, public-house, and
    dark square is a picture feverishly turned--in search of what?
    It is the same with books.
    What do we seek through millions of pages?\
    ";

    let some = 'x';

    // lines() creates an iterator over quote 
    // where each interation is a line of text.
    // Rust uses each operating system conventions on what constitues a new line.

    // COUNTING LINES:
    // let mut line_num: usize = 1;

    // for line in quote.lines() {
    //     if line.contains(search_term) {
    //         println!("{}: {}", line_num, line);
    //     }
    //     line_num += 1;
    // }

    // COUNTING LINES WITH enumerate()
    // Chaining: .enumerate() method
    for (idx, line) in quote.lines().enumerate() {
        if line.contains(search_term) {
            let line_num = idx + 1;
            println!("{}: {}", line_num, line);
        }
    }
}
