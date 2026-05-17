use std::ops::{Range, RangeInclusive};

fn main() {
    // Declaring a range exclusive.
    // Up to 31 but not including <Range> struct.
    let month_days: Range<i32> = 1..31;

    // Range does not have a display trait, but has a debug trait.
    println!("Range exlusive: {:?}", month_days);

    // Declaring a rang inclusive.
    // Including last number in declaraion <RangeInclusive> struct.
    let days_month: RangeInclusive<i32> = 1..=31;
    println!("Range inclusive: {:?}", days_month);

    // Iterating over a range:
    for number in days_month {
        println!("{number}");
    }

    // Whoooa
    let letters = 'b'..='f';
    for letter in letters {
        println!("{}", letter);
    }

    let colors = ["Yellow", "Green", "Bluee"];
    for color in colors {
        println!("{}", color);
    }
}
