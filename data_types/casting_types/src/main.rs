fn main() {
    // let miles_away = 50;
    // let miles_away_i8 = miles_away as i8;

    // println!("{} and {}", miles_away, miles_away_i8);

    // let miles_away = 100.329032;
    // let miles_away_int = miles_away as i32;

    let floor_division = 5 / 3;

    println!("Floor division result : {floor_division}");

    let decimal_division = 5.0 / 3.0;  // You need to divide float by float to get a decimal result.
    
    println!("Devimal division result : {decimal_division}");

    // Modulo operator
    let num = 77;
    let res = fizz_buzz(num);
    println!("Restult for {} is {}", num, res);

    // Augumented assignment:
    let year = 2025;
}


fn fizz_buzz(number: i64) ->  &'static str {
    if number % 5 == 0 && number % 3 == 0 {
        "Fizz Buzz!"
    } else if number % 5 == 0 {
        "Fizz"
    } else if number % 3 == 0{
        "Buzz"
    } else {
        "0"
    }
}