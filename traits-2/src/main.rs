mod aggregator;
use aggregator::{SocialPost, Summary};


fn main() {
    let post = SocialPost {
        username: String::from("Jhonny1"),
        content: String::from("This is bullshit!"),
        reply: false,
        repost: false,
    };

    println!("1 new post: {}", post.summarize());
}
