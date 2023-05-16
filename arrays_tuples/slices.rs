// Slices are parts on coumpounds data types.
// Slice contains 3 elements:
// 1) Starting index, 2) slice operator .. or ..= 3) An ending index.



// Example for slice of and Array.
fn main() {
    let my_array = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let my_slice = &my_array[0..4];

    for element in my_slice {
        println!("{element}");
    }
}

// .. is traditional operator

// ..= is inclusive so in this example above 4 will be printed with this.