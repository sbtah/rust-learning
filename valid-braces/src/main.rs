// Write a function that takes a string of braces, and determines if the order of the braces is valid. It should return true if the string is valid, and false if it's invalid.

// This Kata is similar to the Valid Parentheses Kata, but introduces new characters: brackets [], and curly braces {}. Thanks to @arnedag for the idea!

// All input strings will be nonempty, and will only consist of parentheses, brackets and curly braces: ()[]{}.
// What is considered Valid?

// A string of braces is considered valid if all braces are matched with the correct brace.
// Examples

// "(){}[]"   =>  True
// "([{}])"   =>  True
// "(}"       =>  False
// "[(])"     =>  False
// "[({})](]" =>  False

fn main() {
    let first_test_case = "(){}[]";
    let result = valid_braces_from_codewars(first_test_case);
    println!("result is {}", result);
}

fn valid_braces(s: &str) -> bool {
    use std::collections::HashMap;

    let mut matches: HashMap<char, char> = HashMap::new();

    matches.insert('(', ')');
    matches.insert('[', ']');
    matches.insert('{', '}');

    matches.insert(')', '(');
    matches.insert(']', '[');
    matches.insert('}', '{');

    if s.len() % 2 != 0 {
        return false;
    };

    let mut found_openings: Vec<char> = vec![];

    for (idx, chr) in s.chars().enumerate() {
        if idx == 0 && ")]}".contains(chr) {
            return false;
        };

        if "([{".contains(chr) {
            found_openings.push(chr);
            continue;
        }

        if ")]}".contains(chr) {
            if let Some(last_open) = found_openings.last() {
                if matches.get(last_open) == Some(&chr) {
                    found_openings.pop();
                    continue;
                }
            }
        }
    }
    if found_openings.len() == 0 {
        return true;
    } else {
        return false;
    }
}

fn valid_braces_from_codewars(s: &str) -> bool {
    let mut stack = vec![];
    for ch in s.chars() {
        println!("Current char : {}", ch);
        match ch {
            // If you see opening push closing to the stack.
            '(' => {
                println!("Found `(` opening, adding exected closing: `)` onto stack");
                stack.push(')');
            }
            '[' => {
                println!("Found `[` opening, adding expected closing `]` onto stack");
                stack.push(']');
            }
            '{' => {
                println!("Found `[` opening, adding expected closing `]` onto stack");
                stack.push('}');
            }
            // If you see closing char check if last pushed closing matches it.
            x => {
                println!("Found closing `{}` checking the last closing in stack", x);
                if Some(x) != stack.pop() {
                    println!("Last expected closing in stack is not same as current closing.");
                    return false;
                } else {
                    println!("Last in stack matches the current closing: {}", x);
                }
            }
        }
    }
    stack.is_empty()
}
