use std::io::{ self, stdin };
use std::fs;


fn main() {
    let file_result = read_file();

    match file_result {
        Ok(contents) => println!("{}", contents),
        Err(reason) => {
            eprintln!("The was an error: {}", reason);
        }
    }
}


fn read_file() -> Result<String, io::Error> {
    println!("Please enter the name of a file you'd like to read: ");

    let mut input = String::new();
    stdin().read_line(&mut input)?;


    fs::read_to_string(input.trim())

    // let mut file_contents = String::new();

    // File::open(input.trim())?.read_to_string(&mut file_contents)?;

    // Ok(file_contents)
}