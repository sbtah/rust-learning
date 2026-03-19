fn main() {
    let mut st = String::new();

    let str_data = "Initial content";
    let data = str_data.to_string();

    let s_data = "Initial content 2".to_string();
    
    // Strings can grow in size like Vectors.
    st.push_str("Hello World!");
    println!("st after pushing: {}", st);

    st.push('X');
    println!("st after pushing 2: {}", st);

    // We need to mark second argument as reference
    // fn add(self, s: &str) -> String {}
    // Also &data looks like a reference to a String, but actually, compiler is converting that to &str under the hood.
    let new =  st + &data;
    println!("New str: {}", new);
    println!("Data is still accessible ! {}", data);

    // Indexing:
}
