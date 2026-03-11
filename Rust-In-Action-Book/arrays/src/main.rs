fn main() {
    let one = [1, 2, 3];

    let two: [u8; 3] = [1, 2, 3];

    let blank = [0; 3];

    let blank_2: [u8; 3] = [0; 3];

    let arrays = [one, two, blank, blank_2];

    for a in &arrays {
        println!("{:#?}", a);
        for n in a.iter() {
            println!("\t{} + 10 = {}", n, n + 10);
        }

        let mut sum = 0;
        for i in 0..a.len() {
            sum += a[i];
        }
        println!("\t(x{:#?} = {})",a, sum);
    }
}
