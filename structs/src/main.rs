#[derive(Debug)]
struct User {
    active: bool,
    username: String,
    email: String,
    sign_in_count: u64,
}



// Tuple structs.
struct Color(i32, i32, i32);
struct Point(i32, i32, i32);



fn main() {

    let black = Color(0, 0, 0);
    let origin = Point(0, 0, 0);

    let mut user_1 = User {
        active: true,
        username: String::from("Joe Biden"),
        email: String::from("sleepy.joe@usa.gov"),
        sign_in_count: 1,
    };

    println!("{:#?}", user_1);

    user_1.email = String::from("sleepyjoe@usa.gov");

    println!("{:#?}", user_1);
    
}


fn build_user(email: String, username: String) -> User {
    User {
        active: true,
        username: username,
        email: email,
        sign_in_count: 1
    }
}
