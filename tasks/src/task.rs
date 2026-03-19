#[derive(Debug)]
pub struct Task {
    content: String,
}

impl Task {
    pub fn create(content: String) -> Self {
        Task { content: content }
    }
}
