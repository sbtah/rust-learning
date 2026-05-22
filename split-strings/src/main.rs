// Complete the solution so that it splits the string into strings of two characters in a list/array (depending on the language you use). If the string contains an odd number of characters then it should replace the missing second character of the final pair with an underscore ('_').

// Examples:

// * 'abc' =>  ['ab', 'c_']
// * 'abcdef' => ['ab', 'cd', 'ef']

fn main() {
    let res = solution("abc");
    println!("{:?}", res);
}

fn solution(s: &str) -> Vec<String> {
    let mut result = vec![];

    let mut current_string = "".to_string();

    for (idx, chr) in s.chars().enumerate() {
        current_string.push_str(&chr.to_string());

        if current_string.len() == 2 {
            result.push(current_string.clone());
            current_string = "".to_string();
            continue;
        }

        if current_string.len() < 2 && idx + 1 == s.len() {
            current_string.push_str(&'_'.to_string());
            result.push(current_string.clone());
        }
    }
    result
}

// From Codewars:
fn solution_2(s: &str) -> Vec<String> {
    match s.len() {
        0 => vec![],
        1 => vec![s.to_string() + "_"],
        2 => vec![s.to_string()],
        _ => {
            let mut v = vec![s[0..2].to_string()];
            v.append(&mut solution(&s[2..]));
            v
        }
    }
}
