use super::{Command, CommandError};
use crate::path::PathExpander;
use std::env;

#[derive(Clone)]
pub struct CdCommand {
    path_expander: PathExpander,
}

impl Default for CdCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl CdCommand {
    pub fn new() -> Self {
        Self {
            path_expander: PathExpander::new(),
        }
    }
}

impl Command for CdCommand {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        let path_str = args.first().map(|s| s.as_str()).unwrap_or("~");
        let expanded_path = self
            .path_expander
            .expand(path_str)
            .map_err(|e| CommandError::ExecutionError(e.to_string()))?;

        env::set_current_dir(&expanded_path)
            .map_err(|e| CommandError::ExecutionError(format!("Failed to change directory: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_cd_home() {
        let cmd = CdCommand::new();
        assert!(cmd.execute(&[]).is_ok());
        assert_eq!(
            env::current_dir().unwrap(),
            PathExpander::new().expand("~").unwrap()
        );
    }

    #[test]
    fn test_cd_temp() {
        let cmd = CdCommand::new();
        let temp_dir = env::temp_dir();
        assert!(cmd
            .execute(&[temp_dir.to_str().unwrap().to_string()])
            .is_ok());
        assert_eq!(env::current_dir().unwrap(), temp_dir);
    }

    #[test]
    fn test_cd_invalid() {
        let cmd = CdCommand::new();
        assert!(cmd.execute(&["/nonexistent/path".to_string()]).is_err());
    }
}
