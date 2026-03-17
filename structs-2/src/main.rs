
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

    fn can_hold(&self, other: &Rectangle) -> bool {
        self.area() >= other.area()
    }

    fn square(size: u32) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}


fn main() {

    let rect_1 = Rectangle {
        width: 30,
        height: 50,
    };

    let rect_2 = Rectangle {
        width: 10,
        height: 40,
    };

    let rect_3 = Rectangle {
        width: 60,
        height: 45,
    };

    let square_1 = Rectangle::square(30);

    if rect_1.width() {
        println!("The rectange has nonzero width: {}", rect_1.width);
    }
    // let width_1 = 30;
    // let height_1 = 50;

    println!("Rect: {:#?}", rect_1);
    println!("The area of the rectangle is: {}", rect_1.area());

    println!("Can rect_1 hold rect_2: {}", rect_1.can_hold(&rect_2));
    println!("Can rect_1 hold rect_3: {}", rect_1.can_hold(&rect_3));

}


// fn area(dimensions: &Rectangle) -> u32 {
//     dimensions.width * dimensions.height
// }