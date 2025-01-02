use crate::process::{ProcessError, ProcessExecutor};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod alias;
mod builtin;
mod cd;
mod source;

pub use alias::AliasCommand;
pub use builtin::ExitCommand;
pub use cd::CdCommand;
pub use source::SourceCommand;

#[derive(Debug)]
pub enum CommandError {
    NotFound(String),
    InvalidArguments(String),
    ExecutionError(String),
    IoError(std::io::Error),
    ProcessError(ProcessError),
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NotFound(cmd) => write!(f, "command not found: {}", cmd),
            CommandError::InvalidArguments(msg) => write!(f, "invalid arguments: {}", msg),
            CommandError::ExecutionError(msg) => write!(f, "execution error: {}", msg),
            CommandError::IoError(err) => write!(f, "IO error: {}", err),
            CommandError::ProcessError(err) => write!(f, "Process error: {}", err),
        }
    }
}

impl From<std::io::Error> for CommandError {
    fn from(err: std::io::Error) -> Self {
        CommandError::IoError(err)
    }
}

impl From<ProcessError> for CommandError {
    fn from(err: ProcessError) -> Self {
        CommandError::ProcessError(err)
    }
}

pub trait Command {
    fn execute(&self, args: &[String]) -> Result<(), CommandError>;
}

#[derive(Clone)]
enum CommandType {
    Cd(CdCommand),
    Source(SourceCommand),
    Exit(ExitCommand),
    Alias(AliasCommand),
}

impl Command for CommandType {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        match self {
            CommandType::Cd(cmd) => cmd.execute(args),
            CommandType::Source(cmd) => cmd.execute(args),
            CommandType::Exit(cmd) => cmd.execute(args),
            CommandType::Alias(cmd) => cmd.execute(args),
        }
    }
}

#[derive(Clone)]
pub struct CommandExecutor {
    commands: BTreeMap<String, CommandType>,
    process_executor: ProcessExecutor,
}

impl CommandExecutor {
    pub fn new(flags: &crate::flags::Flags) -> Result<Self, crate::process::ProcessError> {
        let mut executor = Self {
            commands: BTreeMap::new(),
            process_executor: ProcessExecutor::new(flags)?,
        };

        // Create shared aliases storage
        let aliases = Arc::new(Mutex::new(HashMap::new()));

        // Register commands
        executor
            .commands
            .insert("cd".to_string(), CommandType::Cd(CdCommand::new()));
        executor.commands.insert(
            "source".to_string(),
            CommandType::Source(SourceCommand::new(executor.clone())),
        );
        executor
            .commands
            .insert("exit".to_string(), CommandType::Exit(ExitCommand::new()));
        executor.commands.insert(
            "alias".to_string(),
            CommandType::Alias(AliasCommand::new(aliases)),
        );

        Ok(executor)
    }

    pub fn execute(&self, command: &str, args: &[String]) -> Result<(), CommandError> {
        // Convert args to String only for built-in commands
        if let Some(cmd) = self.commands.get(command) {
            cmd.execute(args)
        } else {
            // For external commands, use process executor with string slices
            let mut full_args = vec![command];
            let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            full_args.extend(args_refs);
            self.process_executor
                .spawn_process(&full_args)
                .map_err(|e| CommandError::ExecutionError(e.to_string()))
        }
    }

    pub fn is_builtin(&self, command: &str) -> bool {
        self.commands.contains_key(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_env() -> (CommandExecutor, PathBuf) {
        let executor = CommandExecutor::new(&crate::flags::Flags::default()).unwrap();
        let temp_dir = env::temp_dir();
        (executor, temp_dir)
    }

    #[test]
    fn test_execute_cd() {
        let (executor, temp_dir) = setup_test_env();
        let home_dir = env::var("HOME").unwrap();

        // Test cd without args (should go to home)
        assert!(executor.execute("cd", &[]).is_ok());
        assert_eq!(env::current_dir().unwrap().to_str().unwrap(), home_dir);

        // Test cd to temp directory
        assert!(executor
            .execute("cd", &[temp_dir.to_str().unwrap().to_string()])
            .is_ok());
        assert_eq!(env::current_dir().unwrap(), temp_dir);

        // Test cd with invalid path
        let result = executor.execute("cd", &["/path/that/does/not/exist".to_string()]);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ExecutionError(_))));
    }

    #[test]
    fn test_execute_source() -> Result<(), Box<dyn std::error::Error>> {
        let (executor, temp_dir) = setup_test_env();
        let test_file = temp_dir.join("test_commands.txt");

        // Create test script
        fs::write(&test_file, "cd ~\nexit\n")?;

        // Test source with valid file
        assert!(executor
            .execute("source", &[test_file.to_str().unwrap().to_string()])
            .is_ok());

        // Test source with invalid file
        let result = executor.execute("source", &["/invalid/path".to_string()]);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::ExecutionError(_))));

        // Test source without arguments
        let result = executor.execute("source", &[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::InvalidArguments(_))));

        fs::remove_file(test_file)?;
        Ok(())
    }

    #[test]
    fn test_execute_exit() {
        use std::panic;

        let (executor, _) = setup_test_env();

        let result = panic::catch_unwind(|| {
            executor.execute("exit", &[]).unwrap();
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_execute_unknown_command() {
        let (executor, _) = setup_test_env();

        let result = executor.execute("unknown_command", &[]);
        assert!(result.is_err());
        assert!(matches!(result, Err(CommandError::NotFound(_))));
    }

    #[test]
    fn test_command_chaining() -> Result<(), Box<dyn std::error::Error>> {
        let (executor, temp_dir) = setup_test_env();
        let test_file = temp_dir.join("test_chain.txt");

        // Create test script with multiple commands
        fs::write(&test_file, "cd ~\ncd /tmp\n")?;

        // Execute source command that runs multiple commands
        assert!(executor
            .execute("source", &[test_file.to_str().unwrap().to_string()])
            .is_ok());

        // Verify we ended up in /tmp
        assert_eq!(env::current_dir().unwrap(), PathBuf::from("/tmp"));

        fs::remove_file(test_file)?;
        Ok(())
    }

    #[test]
    fn test_builtin_command_detection() {
        let (executor, _) = setup_test_env();

        assert!(executor.is_builtin("cd"));
        assert!(executor.is_builtin("source"));
        assert!(executor.is_builtin("exit"));
        assert!(!executor.is_builtin("unknown"));
        assert!(!executor.is_builtin(""));
    }

    #[test]
    fn test_executor_clone_behavior() {
        let (executor1, temp_dir) = setup_test_env();
        let executor2 = executor1.clone();

        // Test that both executors can execute commands
        assert!(executor1
            .execute("cd", &[temp_dir.to_str().unwrap().to_string()])
            .is_ok());
        assert!(executor2.execute("cd", &[]).is_ok());

        // Verify both have the same commands registered
        for cmd in ["cd", "source", "exit"].iter() {
            assert_eq!(executor1.is_builtin(cmd), executor2.is_builtin(cmd));
        }
    }

    #[test]
    fn test_command_error_display() {
        let errors = vec![
            CommandError::NotFound("test".to_string()),
            CommandError::InvalidArguments("bad args".to_string()),
            CommandError::ExecutionError("failed".to_string()),
            CommandError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "io error",
            )),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}