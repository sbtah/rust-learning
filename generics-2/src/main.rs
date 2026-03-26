struct TreasureChest<T> {
    captain: String,
    treasure: T,
}

impl<T> TreasureChest<T> {
    fn capital_captain(&self) -> String {
        self.captain.to_uppercase()
    }
}


enum Cheesesteak<T> {
    Plain,
    Topping(T)
}


fn main() {

    let mushroom = Cheesesteak::Topping("mushroom");
    let onions = Cheesesteak::Topping("Onions".to_string());
    
    // let gold_chest = TreasureChest {captain: String::from("boe"), treasure: "elo"};
    // let silver_chest = TreasureChest {captain: String::from("Joel"), treasure: 22};
}


// fn identity(value: i32) -> i32 {
//     value
// }


fn identity<T>(value: T) -> T {
    value
}


// Many Generics.
fn make_tuple<T, U>(first: T, second: U) -> (T, U) {
    (first, second)
}