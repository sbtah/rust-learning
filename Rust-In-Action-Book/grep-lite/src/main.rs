use regex::Regex;
use clap::{App, Arg};


fn main() {
    let args = App::new("grep-lite")
        .version("0.1")
        .about("Searches for patterns")
        .arg(Arg::with_name("pattern")
            .help("The pattern to search for")
            .takes_value(true)
            .required(true))
        .get_matches();


    let pattern = args.value_of("pattern").unwrap();
    let re = Regex::new(pattern).unwrap();


    // Multiline strings does not require a special syntax in Rust.
    // Only new character has to be escaped.\
    let quote = "\
    Every face, every shop, bedroom window, public-house, and
    dark square is a picture feverishly turned--in search of what?
    It is the same with books.
    What do we seek through millions of pages?\
    ";

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
    for line in quote.lines() {
            let contains_substring = re.find(line);
            match contains_substring {
                Some(_) => println!("{}", line),
                None => (),
            }
        }
    }

