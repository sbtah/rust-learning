// The marketing team is spending way too much time typing in hashtags.
// Let's help them with our own Hashtag Generator!

// Here's the deal:

//     It must start with a hashtag (#).
//     All words must have their first letter capitalized, and remaining letters lowercased.

//     If the final result is longer than 140 chars it must return None.
//     If the input or the result is an empty string it must return None.

// Examples

// " Hello there thanks for trying my Kata"  =>  Some("#HelloThereThanksForTryingMyKata")
// "    Hello     World   "                  =>  Some("#HelloWorld")
// ""                                        =>  None
use std::option::Option;

fn main() {
    let test = "    Hello     World   ";

    generate_hashtag(test);
}

fn generate_hashtag(s: &str) -> Option<String> {
    if s.len() == 0 {
        return None;
    }
    let splitted: Vec<&str> = s.split(" ").filter(|c| c.len() > 0).collect();
    let mut res: Vec<String> = vec![];

    for word in splitted {
        let new_word = word[0..1].to_uppercase() + &word[1..].to_lowercase();
        res.push(new_word);
    }

    let joined = res.join("");
    let final_result = format!("#{}", joined);
    if final_result.len() > 140 {
        return None;
    }
    return Some(final_result);
}
