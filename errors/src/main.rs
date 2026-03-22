use std::fs::File;
use std::io::{self, Read, ErrorKind, Error};


// fn main() {
//     let greeting_file_result = File::open("hello.txt");

//     let greeting_file = match greeting_file_result {
//         Ok(file) => file,
//         Err(reason) => match reason.kind() {
//             ErrorKind::NotFound => match File::create("hello.txt") {
//                 Ok(fc) => {
//                     println!("Created new file");
//                     fc
//                 },
//                 Err(e) => panic!("Problem creating the file: {:#?}", e)
//             },
//             _ => {
//                 panic!("Problem opening the file: {:#?}", reason);
//             }
//         }
//     };

//     println!("FIle?: {:#?}", greeting_file);

// }


// Different way:
fn main() {
    let greeting_file = File::open("hello.txt").unwrap_or_else(|error| {
        if error.kind() == ErrorKind::NotFound {
            println!("Creating new file!");
            File::create("hello.txt").unwrap_or_else(|error| {
                panic!("Failed while creating a new file {:#?}", error);
            })
        } else {
            panic!("Problem opening the file: {:#?}", error);
        }
    });
}

// Propagating Error to the caller:
fn read_username() -> Result<String, Error> {
    let username_file_result = File::open("hello.txt");

    let mut username_file = match username_file_result {
        Ok(file) => file,
        Err(reason) => return Err(reason),
    };

    let mut username = String::new();

    return match username_file.read_to_string(&mut username) {
        Ok(_) => Ok(username),
        Err(reason) => Err(reason),
    }
}

fn read_username_v2() -> Result<String, Error> {
    let mut username_file = File::open("hello.txt")?;

    let mut username = String::new();

    username_file.read_to_string(&mut username)?;
    Ok(username)
}