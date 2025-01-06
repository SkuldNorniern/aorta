use super::{Command, CommandError};

#[derive(Clone)]
pub struct ExitCommand;

impl Default for ExitCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl ExitCommand {
    pub fn new() -> Self {
        Self
    }
}

impl Command for ExitCommand {
    fn execute(&self, _args: &[String]) -> Result<(), CommandError> {
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic;

    #[test]
    fn test_exit_command() {
        let cmd = ExitCommand::new();

        let result = panic::catch_unwind(|| {
            cmd.execute(&[]).unwrap();
        });

        assert!(result.is_err());
    }
}
