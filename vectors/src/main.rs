fn main() {
    let mut vv: Vec<i32> = Vec::new();

    vv.push(5);
    vv.push(6);
    vv.push(7);
    vv.push(8);

    // println!("{:#?}", vv);

    let mut v2 = vec![1, 2, 3, 4, 5];
    let third_of_v2 = v2[2];
    // let fourth_of_v2 = v2.get(3);

    v2.push(8);



    println!("The third element of v2 is {}", third_of_v2);
    // if let Some(val) = fourth_of_v2 {
    //     println!("The fourth element of v2 is {}", val);
    // }
    
    println!("v2: {:#?}", v2);


}
