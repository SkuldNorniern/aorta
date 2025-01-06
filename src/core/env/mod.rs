mod paths;
mod vars;

pub use paths::EnvPaths;
pub use vars::EnvVarManager;

use std::path::PathBuf;

#[derive(Debug)]
pub enum EnvError {
    HomeDirNotFound,
    VarNotFound(String),
    IoError(std::io::Error),
    InvalidPath(PathBuf),
    InvalidValue(&'static str),
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvError::HomeDirNotFound => write!(f, "Home directory not found"),
            EnvError::VarNotFound(var) => write!(f, "Environment variable not found: {}", var),
            EnvError::IoError(e) => write!(f, "IO error: {}", e),
            EnvError::InvalidPath(path) => write!(f, "Invalid path: {}", path.display()),
            EnvError::InvalidValue(val) => write!(f, "Invalid value: {}", val),
        }
    }
}

impl std::error::Error for EnvError {}

impl From<std::io::Error> for EnvError {
    fn from(e: std::io::Error) -> Self {
        EnvError::IoError(e)
    }
}

impl From<std::env::VarError> for EnvError {
    fn from(e: std::env::VarError) -> Self {
        match e {
            std::env::VarError::NotPresent => EnvError::HomeDirNotFound,
            std::env::VarError::NotUnicode(_) => EnvError::InvalidValue("Invalid Unicode"),
        }
    }
}
