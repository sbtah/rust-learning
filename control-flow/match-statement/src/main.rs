fn main() {
    let evaluation = true;
    let season = "Winter";

    let rs = match evaluation {
        true => 40,
        false => 20,
    };

    match season {
        "spring" => {
            println!("Lot's of rain");
        }
        "summer" => {
            println!("School is out!");
        }
        "fall" => {
            println!("Lot's of rain and cold");
        }
        "winter" => {
            println!("Santa time");
        }
        _ => {
            println!("Not a proper season!");
        }
    }
}
