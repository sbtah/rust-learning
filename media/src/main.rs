mod content;

use content::media::Media;
use content::catalog::Catalog;

fn main() {

    let audio_book: Media = Media::Audiobook { title: String::from("The Story") };
    let good_movie: Media = Media::Movie { title: String::from("Good Movie"), director: String::from("Joe Biden!") };
    let bad_book: Media = Media::Book { title: String::from("Hashi"), author: String::from("Uio") };
    let podcast: Media = Media::Podcast(10);
    let placeholder: Media = Media::Placeholder;

    // println!("{}", audio_book.description());
    // println!("{}", good_movie.description());
    // println!("{}", bad_book.description());

    let mut catalog: content::catalog::Catalog = Catalog::new();
    catalog.add(audio_book);
    catalog.add(good_movie);
    catalog.add(bad_book);
    catalog.add(podcast);
    catalog.add(placeholder);

    let item = catalog.get_by_index(110);
    // 
    // 3 Other ways to deal with Option:
    // 
    
    // .unwrap() - Option method. Expects a value to be returned. Panics if None.
    // Not recomended to use. 
    // println!("{:#?}", item.unwrap());

    // .expect() - Option method, also panics for None. But with custom message.
    // Debugging?
    // println!("{:#?}", item.expect("NOT FOUND !"));


    // .unwrap_or() - Option method, returns value if exists or placeholder given to it.
    //  ?
    let placeholder = Media::Placeholder;
    println!("{:#?}", item.unwrap_or(&placeholder))

    // println!("{:#?}", catalog.items.get(10));

    // match catalog.items.get(111) {
    //     Some(value) => {
    //         println!("{:#?}", value);
    //     },
    //     None => {
    //         println!("Nothing at that index!")
    //     },
    // }
    // match catalog.get_by_index(1) {
    //     Some(value) => {
    //         println!("Item: {:#?}", value);
    //     },
    //     None => {
    //         println!("No value");
    //     }
    // }

}
