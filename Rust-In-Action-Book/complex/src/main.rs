use  num::complex::Complex;


fn main() {
    let a = Complex {re: 2.1, im: -1.2};
    let b = Complex::new(11.1, 22.2);

    let result = a + b;
    println!("{} and {}", result.re, result.im);

    for n in 0..10 {
        if n % 2 == 0 {
            continue;
        } else {
            println!("{}", n);
        }
    }
}
