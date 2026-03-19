#[derive(Debug)]
enum SpreadSheetCell {
    Int(i32),
    Float(f64),
    Text(String),
}

fn main() {
    // If we know the number of item in the Vector we can use with_capacity.
    // This will pre-allocate memory in the heap. meaning no re-allocation for the vector.
    let v_1:Vec<i32> = Vec::new();
    println!("V1 Vec : {:#?}, capacity: {}",  v_1, v_1.capacity());

    let mut vv: Vec<i32> = Vec::with_capacity(5);
    println!("VV Vec : {:#?}, capacity: {}",  vv, vv.capacity());

    vv.push(5);
    vv.push(6);
    vv.push(7);
    vv.push(8);

    for v in &mut vv {
        println!("{}", v);
    }

    let row = vec![
        SpreadSheetCell::Int(3),
        SpreadSheetCell::Text(String::from("Blue")),
        SpreadSheetCell::Float(3.65),
    ];

    println!("Row: {:#?}", row);
}
