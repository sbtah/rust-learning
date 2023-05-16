// When a programmer declares a new variable with the same name as an already declared variable, it is known as variable shadowing.


fn main() {
    let a = 108;
    println!("\nAddr of a: {:p}, value of a: {a}", &a);

    let a = 56;
    println!("\nAddr of a: {:p} value of a: {a} // post shadowing", &a);


    let mut b = 82;
    println!("\nAddr of b: {:p}, value of b: {b}", &b);

    let mut b = 120;
    println!("\nAddr of b: {:p}, value of b: {b} // post shadowing", &b);


    let mut c = 128;
    println!("\nAddr of c: {:p}, value of c: {c}", &c);

    c = 29;
    println!("\nAddr of c: {:p}, value of c: {c} // post shadowing", &c);
}