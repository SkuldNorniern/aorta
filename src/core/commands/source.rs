use std::fs;

use super::{Command, CommandError, CommandExecutor};
use crate::path::PathExpander;

#[derive(Clone)]
pub struct SourceCommand {
    path_expander: PathExpander,
    executor: CommandExecutor,
}

impl SourceCommand {
    pub fn new(executor: CommandExecutor) -> Self {
        Self {
            path_expander: PathExpander::new(),
            executor,
        }
    }
}

impl Command for SourceCommand {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            return Err(CommandError::InvalidArguments(
                "Source command requires a file path".to_string(),
            ));
        }

        let path = self
            .path_expander
            .expand(&args[0])
            .map_err(|e| CommandError::ExecutionError(e.to_string()))?;

        let content = fs::read_to_string(&path)
            .map_err(|e| CommandError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Execute each line from the file
        for line in content.lines() {
            let line = line.trim();
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<String> = line.split_whitespace().map(String::from).collect();
            if parts.is_empty() {
                continue;
            }

            let command = &parts[0];
            let args = &parts[1..];

            self.executor.execute(command, args).map_err(|e| {
                CommandError::ExecutionError(format!("Failed to execute '{}': {}", line, e))
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, fs, path::PathBuf};

    fn setup_test_file(content: &str) -> (PathBuf, CommandExecutor) {
        let temp_dir = env::temp_dir();
        let test_file = temp_dir.join("test_source.txt");
        fs::write(&test_file, content).unwrap();
        (
            test_file,
            CommandExecutor::new(&crate::flags::Flags::default()).unwrap(),
        )
    }

    #[test]
    fn test_source_valid_file() {
        let (test_file, executor) = setup_test_file("cd ~\n");
        let cmd = SourceCommand::new(executor);

        assert!(cmd
            .execute(&[test_file.to_str().unwrap().to_string()])
            .is_ok());
        fs::remove_file(test_file).unwrap();
    }

    #[test]
    fn test_source_invalid_file() {
        let executor = CommandExecutor::new(&crate::flags::Flags::default()).unwrap();
        let cmd = SourceCommand::new(executor);

        assert!(cmd.execute(&["/nonexistent/file".to_string()]).is_err());
    }

    #[test]
    fn test_source_empty_args() {
        let executor = CommandExecutor::new(&crate::flags::Flags::default()).unwrap();
        let cmd = SourceCommand::new(executor);

        assert!(cmd.execute(&[]).is_err());
    }

    #[test]
    fn test_source_with_comments() {
        let (test_file, executor) = setup_test_file("# This is a comment\ncd ~\n");
        let cmd = SourceCommand::new(executor);

        assert!(cmd
            .execute(&[test_file.to_str().unwrap().to_string()])
            .is_ok());
        fs::remove_file(test_file).unwrap();
    }
}
