use crate::error::ShellError;
use super::environment::EnvironmentHandler;
use super::pipeline::Pipeline;
use std::collections::HashMap;

pub(crate) trait CommandHandler {
    fn execute_command(&mut self, command: &str) -> Result<(), ShellError>;
}

impl CommandHandler for super::Shell {
    fn execute_command(&mut self, command: &str) -> Result<(), ShellError> {
        // Skip empty commands early
        if command.trim().is_empty() {
            return Ok(());
        }

        // Record start time for duration tracking
        let start_time = std::time::Instant::now();

        // Parse pipeline with the original command
        // Let the pipeline handle alias and env var expansion
        let pipeline = Pipeline::parse(command)
            .map_err(ShellError::PipelineError)?;

        // Create environment variables HashMap
        let env_vars: HashMap<String, String> = std::env::vars().collect();

        // Execute pipeline with shell context
        let result = pipeline.execute_with_context(
            &env_vars,
            &self.config.get_aliases(),
            &self.executor,
        );

        // Calculate duration
        let duration = start_time.elapsed().as_millis() as u64;

        // Add to history with execution details
        if let Err(e) = self.history.add_with_details(
            command,  // Use original command for history
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
            Err(e) => Err(ShellError::PipelineError(e)),
        }
    }
} 