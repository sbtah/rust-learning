
#[derive(Debug)]
struct Rectangle {
    width: u32,
    height: u32,
}


impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }
    // We can create methods with same names as fields on struct.
    fn width(&self) -> bool {
        self.width > 0
    }
}


fn main() {

    let rect_1 = Rectangle {
        width: 30,
        height: 50,
    };

    if rect_1.width() {
        println!("The rectange has nonzero width: {}", rect_1.width);
    }
    // let width_1 = 30;
    // let height_1 = 50;

    println!("Rect: {:#?}", rect_1);
    println!("The area of the rectangle is: {}", rect_1.area());
}


// fn area(dimensions: &Rectangle) -> u32 {
//     dimensions.width * dimensions.height
// }