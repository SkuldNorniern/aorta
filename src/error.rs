#[derive(Debug)]
pub enum ShellError {
    Readline(rustyline::error::ReadlineError),
    Io(std::io::Error),
    HomeDirNotFound,
    InvalidShellPath,
    CommandNotFound(String),
    ConfigError(String, String),
    FlagError(String),
}

impl From<rustyline::error::ReadlineError> for ShellError {
    fn from(err: rustyline::error::ReadlineError) -> Self {
        ShellError::Readline(err)
    }
}

impl From<std::io::Error> for ShellError {
    fn from(err: std::io::Error) -> Self {
        ShellError::Io(err)
    }
}

impl std::fmt::Display for ShellError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellError::Readline(e) => write!(f, "Readline error: {}", e),
            ShellError::Io(e) => write!(f, "IO error: {}", e),
            ShellError::HomeDirNotFound => write!(f, "Home directory not found"),
            ShellError::InvalidShellPath => write!(f, "Invalid shell path"),
            ShellError::CommandNotFound(cmd) => write!(f, "command not found: {}", cmd),
            ShellError::ConfigError(path, msg) => write!(f, "Config error in {}: {}", path, msg),
            ShellError::FlagError(msg) => write!(f, "Flag error: {}", msg),
        }
    }
}

impl std::error::Error for ShellError {}
