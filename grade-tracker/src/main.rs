#[derive(Debug)]
struct Student {
    grades: Vec<f64>,
}

impl Student {
    fn add_grade(&mut self, grade: f64) {
        self.grades.push(grade);
    }

    fn average(&self) -> f64 {
        self.grades.iter().sum::<f64>() / self.grades.len() as f64
    }

    fn highest(&self) -> f64 {
        let mut highest: f64 = 0.0;
        for grade in &self.grades {
            if grade > &highest {
                highest = *grade
            }
        }
        highest
    } 
}

fn main() {
    println!("Hello, world!");
}
