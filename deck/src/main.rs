use rand::{ random, rng, rngs::ThreadRng, seq::SliceRandom};


#[derive(Debug)]
struct Deck {
    cards: Vec<String>,
}

// Inherent implementation aka: add a function or a method to a struct.
impl Deck {
    fn new() -> Self {
    // List of suits: hearts, spades etc..
    let suits: [&str; 4] = ["Hearts", "Spades", "Diamonds", "Clubs"];
    // List of values : ace, two, three
    let values: [&str; 3] = ["Ace", "Two", "Three"];
    // Double nested for loop to generate all combinations.

    let mut cards: Vec<String> = Vec::new();

    for suit in suits {
        for value in values {
            let card: String = format!("{} Of {}", value, suit);
            cards.push(card);
        }
    }
    // Implicit return!
    Deck{ cards: cards }
    // Explicit return
    // return Deck{ cards: cards };
    }

    fn shuffle(&mut self) {
        let mut rng: ThreadRng = rng();
        self.cards.shuffle(&mut rng);
    }

    fn deal(&mut self, num_cards: usize) -> Vec<String> {
        // 
        let to_return: usize = self.cards.len() - num_cards;
        // Split Vector at given index return new vector 
        let new_vec: Vec<String> = self.cards.split_off(to_return);
        return new_vec;
    }
}

// x x x x x
fn main() {
    let mut deck: Deck = Deck::new();
    println!("Heres your deck: {:#?}", deck);

    deck.shuffle();
    let cards = deck.deal(3);

    println!("Heres your hand: {:#?}", cards);
}
