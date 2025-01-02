use std::fmt;

pub mod executor;
pub mod signal;

#[derive(Debug)]
pub enum ProcessError {
    CommandNotFound(String),
    SignalError(String),
    Other(String),
}

impl From<std::io::Error> for ProcessError {
    fn from(e: std::io::Error) -> Self {
        ProcessError::Other(e.to_string())
    }
}

impl fmt::Display for ProcessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessError::CommandNotFound(cmd) => write!(f, "Command not found: {}", cmd),
            ProcessError::SignalError(msg) => write!(f, "Signal error: {}", msg),
            ProcessError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}
