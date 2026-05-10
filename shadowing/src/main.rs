// Shadowing.
fn main() {
    let grams_of_protein = "100.345";

    let grams_of_protein = 100.345;

    let mut grams_of_protein = 100;

    grams_of_protein = 105;
    // Nested scope.
    {
        // Stuff created here only lives here.
        // But we can access outer scope.
        let cats = 30;
        println!("I ate {} grams of protein", grams_of_protein);
        println!("I can see {} cats here", cats);

    }

    // println!("But I can't see {} cats here", cats);
}
