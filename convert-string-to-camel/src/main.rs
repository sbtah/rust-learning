// Complete the method/function so that it converts dash/underscore delimited words into camel casing. The first word within the output should be capitalized only if the original word was capitalized (known as Upper Camel Case, also often referred to as Pascal case). The next words should be always capitalized.
// Examples

// "the-stealth-warrior" gets converted to "theStealthWarrior"

// "The_Stealth_Warrior" gets converted to "TheStealthWarrior"

// "The_Stealth-Warrior" gets converted to "TheStealthWarrior"

fn main() {
    let _first_test = "";
    let _second_test = "The_Stealth_Warrior";
    let _third_test = "The_Stealth-Warrior";
    let result = to_camel_case(_first_test);
    println!("{}", result)
}

fn to_camel_case(text: &str) -> String {
    let mut result: Vec<String> = vec![];
    let splitted: Vec<&str> = text.split(&['-', '_'][..]).collect();

    if text.len() == 0 {
        return text.to_string();
    }
    for (idx, word) in splitted.iter().enumerate() {
        if idx == 0 && word.chars().nth(0).unwrap().is_lowercase() {
            result.push(word.to_string());
            continue;
        }
        let uppercased = format!(
            "{}{}",
            word.chars().nth(0).unwrap().to_uppercase(),
            &word[1..]
        );
        result.push(uppercased);
    }
    result.join("")
}

#[cfg(test)]
mod tests {
    use super::to_camel_case;

    const ERR_MSG: &str = "\nYour result (left) did not match the expected output (right)";

    fn dotest(s: &str, expected: &str) {
        assert_eq!(to_camel_case(s), expected, "{ERR_MSG} with text = \"{s}\"")
    }

    #[test]
    fn fixed_tests() {
        dotest("", "");
        dotest("the_stealth_warrior", "theStealthWarrior");
        dotest("The-Stealth-Warrior", "TheStealthWarrior");
        dotest("A-B-C", "ABC");
    }
}
