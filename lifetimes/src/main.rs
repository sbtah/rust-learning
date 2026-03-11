fn next_language<'a>(collection: &'a[String], searched: &str) -> &'a str {
    let mut found = false;
    for lang in collection {
        if found {
            return lang;
        }
        if lang == searched {
            found = true;
        }
    }
    collection.last().unwrap()
}


fn last_language(collection: &[String]) -> &str {
    collection.last().unwrap()
}


fn longest<'a>(first: &'a str, second: &'a str) -> &'a str {
    if first.len() > second.len() {
        first
    } else if second.len() > first.len() {
        second
    } else {
        "None"
    }
}

fn main() {
    let languages = [
        String::from("Go"),
        String::from("Python"),
        String::from("Rust"),
        String::from("TypeScript"),
    ];

    let res = next_language(&languages, &"Go");

    println!("{:#?}", res);
}