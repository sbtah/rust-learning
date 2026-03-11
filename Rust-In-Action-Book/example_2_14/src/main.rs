fn add_with_lifetimes<'a, 'b>(i: &'a i32, j: &'b i32) -> i32 {
    *i + *j // Dereferencing - meaning add values not references.
}

fn add<T: std::ops::Add<Output = T>>(i: T, j: T) -> T {
    i + j
}



fn main() {
    let a = 10;
    let b = 20;
    let result = add_with_lifetimes(&a, &b);

    println!("{}", result);
}
