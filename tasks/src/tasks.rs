use crate::task::Task;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Tasks {
    queue: HashMap<usize, Task>,
}

impl Tasks {
    pub fn new() -> Tasks {
        Tasks {
            queue: HashMap::new(),
        }
    }

    pub fn get_next_number(&self) -> usize {
        self.queue.len()
    }

    pub fn add(&mut self, task_value: String) {
        let next_number: usize = self.get_next_number();
        let new_task: Task = Task::create(task_value);
        self.queue.insert(next_number, new_task);
    }

    pub fn list(&self) {
        self.queue.iter().for_each(|task| println!("{:#?}", task));
    }

    pub fn done(&mut self, number: usize) {
        match self.queue.remove(&number) {
            Some(val) => println!("Sucessfully removed Task : {:#?}", val),
            None => println!("No Task found by given number")
        };
    }
}
