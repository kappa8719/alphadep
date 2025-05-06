use std::time::Duration;

pub struct Execution {
    pub command: String,
    pub timeout: Option<Duration>,
}

pub enum ExecutionMessage {
    Stdout(String),
    Stderr(String),
}
