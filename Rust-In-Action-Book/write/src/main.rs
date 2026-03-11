use std::fmt::format;
use std::fs::File;
use std::io::{Write, BufWriter};
use std::path::Path;


fn main() {
    let output_file = Path::new("result.txt");

    let file = File::create(output_file).expect("Error while creating file!");

    let mut writer = BufWriter::new(file);

    for value in 0..=1000 {

        let to_save = format!("Line {}", value);
        writeln!(writer, "{}", to_save);
    }

    writer.flush().expect("OK");
}
