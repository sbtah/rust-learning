use tasks::Tasks;
use std::io;

mod task;
mod tasks;



fn main() {
    let mut tasks = Tasks::new();

    println!("Started EASY Task tool!");


    loop {

        println!("1: List current Tasks.");
        println!("2: Add new Task.");
        println!("3: Remove Task by number.");
        println!("4: Finish work.");

        println!("Your choice?: ");
        let mut choice_input = String::new();
        io::stdin().read_line(&mut choice_input).expect("Failed to read choice!");

        match choice_input.trim() {
            "1" => {
                tasks.list();
                println!();
                continue;
            },
            "2" => {
                println!("Your task content?: ");
                let mut task_input = String::new();
                io::stdin().read_line(&mut task_input).expect("Failed read task content!");
                let trimmed = task_input.trim();
                let task_content = String::from(trimmed);
                tasks.add(task_content);
                continue;
            },
            "3" => {
                println!("Please give task number to remove: ");
                let mut remove_input = String::new();
                io::stdin().read_line(&mut remove_input).expect("Failed read task content!");
                let trimmed = remove_input.trim();
                let number_to_remove = trimmed.parse::<usize>().unwrap();
                tasks.done(number_to_remove);
                println!()
            },
            _ => {},
        }
    }


}
