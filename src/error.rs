use crate::core::commands::CommandError;
use crate::core::config::ConfigError;
use crate::input::history::HistoryError;
use crate::process::ProcessError;
use crate::shell::pipeline::PipelineError;

#[derive(Debug)]
pub enum ShellError {
    Readline(rustyline::error::ReadlineError),
    Io(std::io::Error),
    HomeDirNotFound,
    InvalidShellPath,
    CommandNotFound(String),
    ProcessError(ProcessError),
    ConfigError(ConfigError),
    FlagError(String),
    CtrlC(String),
    CommandError(CommandError),
    HistoryError(HistoryError),
    PipelineError(PipelineError),
    PathError(String),
    FileReadError(String),
    IoError(String),
    ShellRegistrationError(String),
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

impl From<ctrlc::Error> for ShellError {
    fn from(err: ctrlc::Error) -> Self {
        ShellError::CtrlC(err.to_string())
    }
}

impl From<ProcessError> for ShellError {
    fn from(err: ProcessError) -> Self {
        ShellError::ProcessError(err)
    }
}

impl From<ConfigError> for ShellError {
    fn from(err: ConfigError) -> Self {
        ShellError::ConfigError(err)
    }
}

impl From<CommandError> for ShellError {
    fn from(err: CommandError) -> Self {
        ShellError::CommandError(err)
    }
}

impl From<HistoryError> for ShellError {
    fn from(err: HistoryError) -> Self {
        ShellError::HistoryError(err)
    }
}

impl From<PipelineError> for ShellError {
    fn from(err: PipelineError) -> Self {
        ShellError::PipelineError(err)
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
            ShellError::ConfigError(e) => write!(f, "Config error: {}", e),
            ShellError::FlagError(msg) => write!(f, "Flag error: {}", msg),
            ShellError::CtrlC(msg) => write!(f, "Ctrl-C error: {}", msg),
            ShellError::ProcessError(e) => write!(f, "Process error: {}", e),
            ShellError::CommandError(e) => write!(f, "Command error: {}", e),
            ShellError::HistoryError(e) => write!(f, "History error: {}", e),
            ShellError::PipelineError(e) => write!(f, "Pipeline error: {}", e),
            ShellError::PathError(e) => write!(f, "Path error: {}", e),
            ShellError::FileReadError(e) => write!(f, "File read error: {}", e),
            ShellError::IoError(e) => write!(f, "IO error: {}", e),
            ShellError::ShellRegistrationError(e) => write!(f, "Shell registration error: {}", e),
        }
    }
}

impl std::error::Error for ShellError {}
