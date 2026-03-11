// Iterators are lazy
// When created nothing happens until you call next.
use std::str::FromStr;

fn print_elements(collection: &[String]) {

    // Lambdas?
    collection
        .iter()
        .map(|element|format!("{} {}", element, element))
        .for_each(|element|println!("{}", element)); // for_each - iterator consumer. Calls next and runs closure on each element.

    // For x in vec implicitly calls `into_iter` which consumes vec and takes ownership of values
    // This depends on if collection received is a reference or a value.
    // for element in collection {
    //     println!("{}", element);
    // }
}

fn shorten_strings(collection: &mut [String]) {
    collection.iter_mut().for_each(|element| element.truncate(1));
}


fn to_uppercase(collection: &[String]) -> Vec<String> {
    //  My Solution:
    // let mut uppercased: Vec<String> = Vec::new();
    // collection.iter().for_each(|element| uppercased.push(element.to_uppercase()));
    // return uppercased;
    collection.iter().map(|element| element.to_uppercase()).collect::<Vec<String>>()
}


fn move_elements(collection_a: Vec<String>, collection_b: &mut Vec<String>) {
    collection_a.into_iter().for_each(|element| collection_b.push(element));
}

fn explode(collection: &[String]) -> Vec<Vec<String>> {
    collection
        .iter()
        .map(
            |element| element
                .chars()
                .map(|char| char.to_string()).collect()
        ).collect()
}

fn my_explode(collection: &[String]) -> Vec<Vec<String>> {
    let mut outer_vec: Vec<_> = Vec::new();

    for element in collection {

        let mut inner_vec: Vec<_> = Vec::new();

        for char in element.chars() {
            inner_vec.push(char.to_string());
        }
        outer_vec.push(inner_vec);
    }
    outer_vec
}


fn find_color_or(collection: &[String], search: &str, fallback: &str) -> String {
    collection
        .iter()
        .find(|element| element.contains(search))
        .map_or(String::from(fallback), |el| el.to_string())
}


fn main() {
    let mut colors: Vec<String> = vec![
        String::from("Red"),
        String::from("Green"),
        String::from("Blue"),
    ];

    // shorten_strings(&mut colors[0..2]);
    // print_elements(&colors[1..3]);

    let uppercased: Vec<String> = to_uppercase(&colors);
    println!("{:#?}", uppercased);
    println!("{:#?}", colors);

    let result = my_explode(&colors);

    println!("{:#?}", result);

    // let mut destination: Vec<String> = Vec::new();
    // move_elements(colors, &mut destination);
    //
    // println!("DEST: {:#?}", destination);
}


