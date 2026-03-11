use std::{fs, io::Error};


fn extract_errors(text: &str) -> Vec<String> {
    let split_text = text.split("\n");

    let mut results = Vec::new();

    for line in split_text {
        if line.starts_with("ERROR") {
            results.push(line.to_string());  // Here we create new Strings from slices.
        }
    }

    return results;
}

fn main() -> Result<(), Error> {
    let text: String = fs::read_to_string("logs.txt")?;
    let error_logs = extract_errors(text.as_str());  // We need to put read only str slice here.
    fs::write("errors.txt", error_logs.join("\n"))?;
    Ok(())
}

// fn main() {

//     let text: String = fs::read_to_string("ssslogs.txt").expect("Initial file does not exist");

//     let error_logs = extract_errors(text.as_str());  // We need to put read only str slice here.

//     fs::write("errors.txt", error_logs.join("\n")).expect("Saving file failed")

    // MATCH WAY!
    // let read_result: Result<String, Error> = fs::read_to_string("logs.txt");
    // match read_result {
    //     Ok(text) => {
    //         let error_logs = extract_errors(text.as_str());
    //         match fs::write("errors.txt", error_logs.join("\n")) {
    //             Ok(..) => println!("Ok"),
    //             Err(reason) => println!("Error while saving {:#?}", reason)
    //         }
    //     },
    //     Err(reason) => println!("Read failed: {}", reason)
    // }
// }
    

    // match divide(5.0, 0.0) {
    //     Ok(value) => {
    //         println!("{:#?}", value);
    //     },
    //     Err(err) => {
    //         println!("{:#?}", err);
    //     }
    // }

    // match validate_email(String::from("testtest.com")) {
    //     Ok(..) => {
    //         println!("Email is valid");
    //     }
    //     Err(failed) => {
    //         println!("Failed: {:#?}", failed);
    //     }
    // }
// }


// fn divide(a: f64, b: f64) -> Result<f64, Error> {
//     if b == 0.0 {
//         Err(Error::other("Cant divide by 0"))
//     } else {
//         Ok(a / b)
//     }
// }


// fn validate_email(email: String) -> Result<(), Error> {
//     if email.contains("@") {
//         Ok(())
//     } else {
//         Err(Error::other("This is not an email"))
//     }
// }