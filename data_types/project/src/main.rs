fn main() {
    let some_number: i32 = 1_337;

    let some_number_i16: i16 = some_number as i16;

    let some_float: f64 = 3.1456546764;
    println!("{some_float:.3}");

    let with_milk: bool = true;
    let with_sugar: bool = true;

    let is_my_type_of_coffee: bool = with_milk && with_sugar;
    let is_acceptable_coffee: bool = with_milk || with_sugar;

    let some_arr: [i8; 4] = [2, 4, 8, 16];

    let some_tuple: (i32, f64, bool, [i8; 4]) = (12, 3.15, true, some_arr);

    println!("My weird tuple {:#?}", some_tuple);
    println!("{:?}", some_arr);
}
