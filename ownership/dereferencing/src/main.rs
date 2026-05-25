fn main() {
    // & - Operator implying reference (Borrowed value)
    let my_stack_value = 2;
    let my_stack_reference = &my_stack_value;
    println!("{}", *my_stack_reference);

    let my_heap_value = String::from("Turbo Cat");
    let my_heap_reference = &my_heap_value;
    println!("{}", *my_heap_reference);
}
