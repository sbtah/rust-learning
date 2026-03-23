use std::fs::File;
use std::io::{Read, stdin, self};
use std::process::exit;



fn main() {
    let file_result = read_file();

    let file_contents = match file_result {
        Ok(contents) => println!("{}", contents),
        Err(reason) => {
            eprintln!("Error while reading the file: {}", reason);
            exit(1);
        }
    };
}


fn read_file() -> Result<String, io::Error> {
    println!("Please enter the name of a file: ");

    let mut input = String::new();
    let user_requested = stdin().read_line(&mut input);
    if let Err(reason) = user_requested {
        return Err(reason);
    }

    let mut file = match File::open(&input.trim()) {
        Ok(file) => file,
        Err(reason) => {
            return Err(reason);
        }
    };
    
    let mut file_content = String::new();
    let read_operation = file.read_to_string(&mut file_content);
    if let Err(reason) = read_operation {
        return Err(reason);
    }

    Ok(file_content)
}