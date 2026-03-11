mod basket;
mod stack;
mod container;


use basket::Basket;
use stack::Stack;
use container::Container;


fn add_string<T: Container<String>>(container: &mut T, s: String) {
    container.put(s);
}

fn main() {
    let mut b1 = Basket::new(String::from("Hi There"));

    let b2 = Basket::new(false);

    let b3 = Basket::new(22);

    let mut s1 = Stack::new(
        vec![
            String::from("1"),String::from("2"), String::from("3")
        ]
    );

    add_string(&mut b1, String::from("LOL"));
    add_string(&mut s1, String::from("LOL"));
}
