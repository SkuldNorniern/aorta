use crate::error::ShellError;
use super::environment::EnvironmentHandler;

pub(crate) trait CommandHandler {
    fn execute_command(&mut self, command: &str) -> Result<(), ShellError>;
}

impl CommandHandler for super::Shell {
    fn execute_command(&mut self, command: &str) -> Result<(), ShellError> {
        // Skip empty commands early
        if command.trim().is_empty() {
            return Ok(());
        }

        // Expand aliases and environment variables
        let expanded_command = self.config.expand_aliases(command);
        let expanded_command = EnvironmentHandler::expand_env_vars(self, &expanded_command);

        // Parse command and arguments
        let args: Vec<&str> = expanded_command.split_whitespace().collect();
        if args.is_empty() {
            return Ok(());
        }

        let command_name = args[0];
        let command_args: Vec<String> = args[1..].iter().map(|&s| s.to_string()).collect();

        // Execute and track command
        let start_time = std::time::Instant::now();
        let result = self.executor.execute(command_name, &command_args);
        let duration = start_time.elapsed().as_millis() as u64;

        // Update history
        if let Err(e) = self.history.add_with_details(
            command,
            result.is_err() as i32,
            duration,
        ) {
            if !self.flags.is_set("quiet") {
                eprintln!("Warning: Failed to add command to history: {}", e);
            }
        }

        // Update current directory on success
        if result.is_ok() {
            self.current_dir = std::env::current_dir()?.to_string_lossy().to_string();
        }

        match result {
            Ok(_) => Ok(()),
            Err(e) => Err(ShellError::CommandError(e)),
        }
    }
} 