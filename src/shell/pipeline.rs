use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    io::Write,
    process::{Command, Stdio},
};

use crate::core::commands::{CommandError, CommandExecutor};

#[derive(Debug)]
pub enum PipelineOperator {
    Pipe,        // |
    And,         // &&
    Or,          // ||
    Sequence,    // ;
    Redirect,    // >
    RedirectIn,  // <
}

#[derive(Debug)]
pub struct PipelineStage {
    pub command: String,
    pub args: Vec<String>,
    pub operator: Option<PipelineOperator>,
}

#[derive(Debug)]
pub enum PipelineError {
    Io(std::io::Error),
    Command(CommandError),
    Parse(String),
    Execution(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "IO error: {}", err),
            Self::Command(err) => write!(f, "Command error: {}", err),
            Self::Parse(msg) => write!(f, "Parse error: {}", msg),
            Self::Execution(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl From<std::io::Error> for PipelineError {
    fn from(err: std::io::Error) -> Self {
        PipelineError::Io(err)
    }
}

impl From<CommandError> for PipelineError {
    fn from(err: CommandError) -> Self {
        PipelineError::Command(err)
    }
}

pub struct Pipeline {
    stages: Vec<PipelineStage>,
}

impl Pipeline {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    #[cfg(test)]
    pub fn stage_count(&self) -> usize {
        self.stages.len()
    }

    #[cfg(test)]
    pub fn first_command(&self) -> Option<&str> {
        self.stages.first().map(|s| s.command.as_str())
    }

    pub fn parse(input: &str) -> Result<Self, PipelineError> {
        let mut stages = Vec::new();
        let mut current_command = String::new();
        let mut chars = input.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '|' => {
                    if chars.peek() == Some(&'|') {
                        chars.next(); // consume second '|'
                        Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::Or))?;
                    } else {
                        // Check if there's any non-whitespace content after the pipe
                        let remaining: String = chars.clone().collect();
                        if remaining.trim().is_empty() {
                            return Err(PipelineError::Parse(
                                "Incomplete pipeline: missing command after |".to_string(),
                            ));
                        }
                        Self::add_stage(
                            &mut stages,
                            &current_command,
                            Some(PipelineOperator::Pipe),
                        )?;
                    }
                    current_command.clear();
                }
                '&' if chars.peek() == Some(&'&') => {
                    chars.next(); // consume second '&'
                                  // Check if there's any non-whitespace content after &&
                    let remaining: String = chars.clone().collect();
                    if remaining.trim().is_empty() {
                        return Err(PipelineError::Parse(
                            "Incomplete command: missing command after &&".to_string(),
                        ));
                    }
                    Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::And))?;
                    current_command.clear();
                }
                ';' => {
                    Self::add_stage(
                        &mut stages,
                        &current_command,
                        Some(PipelineOperator::Sequence),
                    )?;
                    current_command.clear();
                }
                '>' => {
                    Self::add_stage(
                        &mut stages,
                        &current_command,
                        Some(PipelineOperator::Redirect),
                    )?;
                    current_command.clear();
                }
                '<' => {
                    Self::add_stage(
                        &mut stages,
                        &current_command,
                        Some(PipelineOperator::RedirectIn),
                    )?;
                    current_command.clear();
                }
                _ => current_command.push(c),
            }
        }

        // Add the last command if any
        if !current_command.trim().is_empty() {
            Self::add_stage(&mut stages, &current_command, None)?;
        }

        if stages.is_empty() {
            return Err(PipelineError::Parse("Empty pipeline".to_string()));
        }

        Ok(Self { stages })
    }

    fn add_stage(
        stages: &mut Vec<PipelineStage>,
        command_str: &str,
        operator: Option<PipelineOperator>,
    ) -> Result<(), PipelineError> {
        let trimmed = command_str.trim();
        if trimmed.is_empty() {
            return Err(PipelineError::Parse("Empty command".to_string()));
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err(PipelineError::Parse("Empty command".to_string()));
        }

        stages.push(PipelineStage {
            command: parts[0].to_string(),
            args: parts[1..].iter().map(|s| s.to_string()).collect(),
            operator,
        });

        Ok(())
    }

    pub fn execute_with_context(
        &self,
        _env_vars: &HashMap<String, String>,
        aliases: &BTreeMap<Cow<'_, str>, Cow<'_, str>>,
        executor: &CommandExecutor,
    ) -> Result<(), PipelineError> {
        let mut previous_output: Option<Vec<u8>> = None;
        let mut last_success = true;

        for (index, stage) in self.stages.iter().enumerate() {
            let (command, args) = Self::resolve_stage(stage, aliases);

            let prev_operator = index
                .checked_sub(1)
                .and_then(|i| self.stages.get(i))
                .and_then(|s| s.operator.as_ref());
            let skip = matches!(
                (prev_operator, last_success),
                (Some(PipelineOperator::And), false) | (Some(PipelineOperator::Or), true)
            );
            if skip {
                continue;
            }

            previous_output = match &stage.operator {
                Some(PipelineOperator::Pipe) => {
                    Self::run_pipe_stage(&command, &args, previous_output)?
                }
                Some(PipelineOperator::And)
                | Some(PipelineOperator::Or)
                | Some(PipelineOperator::Sequence)
                | None => {
                    let (out, success) = Self::run_sequence_stage(
                        &command,
                        &args,
                        previous_output,
                        executor,
                    )?;
                    last_success = success;
                    out
                }
                Some(PipelineOperator::Redirect) => {
                    self.run_redirect_stage(index, &command, &args, previous_output)?;
                    return Ok(());
                }
                Some(PipelineOperator::RedirectIn) => {
                    let (out, success) = self.run_redirect_in_stage(index, &command, &args)?;
                    last_success = success;
                    out
                }
            };
        }

        if let Some(output) = previous_output {
            if !output.is_empty() {
                if let Ok(s) = String::from_utf8(output) {
                    print!("{}", s);
                }
            }
        }

        Ok(())
    }

    fn resolve_stage(
        stage: &PipelineStage,
        aliases: &BTreeMap<Cow<'_, str>, Cow<'_, str>>,
    ) -> (String, Vec<String>) {
        let expanded_parts: Vec<String> = if let Some(alias) = aliases.get(stage.command.as_str()) {
            alias.split_whitespace().map(String::from).collect()
        } else {
            vec![stage.command.clone()]
        };
        let command = expanded_parts[0].clone();
        let mut args = expanded_parts[1..].to_vec();
        args.extend(stage.args.clone());
        (command, args)
    }

    fn run_pipe_stage(
        command: &str,
        args: &[String],
        previous_output: Option<Vec<u8>>,
    ) -> Result<Option<Vec<u8>>, PipelineError> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        let output = if let Some(prev_out) = previous_output {
            cmd.stdin(Stdio::piped());
            let mut child = cmd.spawn().map_err(|e| PipelineError::Execution(e.to_string()))?;
            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(&prev_out);
            }
            child
                .wait_with_output()
                .map_err(|e| PipelineError::Execution(e.to_string()))?
        } else {
            cmd.stdin(Stdio::inherit());
            cmd.output().map_err(|e| PipelineError::Execution(e.to_string()))?
        };

        Ok(Some(output.stdout))
    }

    fn run_sequence_stage(
        command: &str,
        args: &[String],
        previous_output: Option<Vec<u8>>,
        executor: &CommandExecutor,
    ) -> Result<(Option<Vec<u8>>, bool), PipelineError> {
        let success = if let Some(prev_out) = previous_output {
            if !executor.is_builtin(command) {
                let mut cmd = Command::new(command);
                cmd.args(args)
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit());
                let mut child = cmd.spawn().map_err(|e| PipelineError::Execution(e.to_string()))?;
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(&prev_out);
                }
                let output = child
                    .wait_with_output()
                    .map_err(|e| PipelineError::Execution(e.to_string()))?;
                if !output.stdout.is_empty() {
                    let s = String::from_utf8_lossy(&output.stdout);
                    print!("{}", s);
                }
                output.status.success()
            } else {
                executor
                    .execute_with_status(command, args)
                    .map_err(|e| PipelineError::Execution(e.to_string()))?
            }
        } else {
            executor
                .execute_with_status(command, args)
                .map_err(|e| PipelineError::Execution(e.to_string()))?
        };
        Ok((None, success))
    }

    fn run_redirect_stage(
        &self,
        index: usize,
        command: &str,
        args: &[String],
        previous_output: Option<Vec<u8>>,
    ) -> Result<(), PipelineError> {
        let next_stage = self
            .stages
            .get(index + 1)
            .ok_or_else(|| PipelineError::Execution("Redirect operator requires a file path".to_string()))?;

        let output = if let Some(out) = previous_output {
            out
        } else {
            let mut cmd = Command::new(command);
            cmd.args(args)
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit());
            let result = cmd.output().map_err(|e| PipelineError::Execution(e.to_string()))?;
            result.stdout
        };

        std::fs::write(&next_stage.command, output)?;
        Ok(())
    }

    fn run_redirect_in_stage(
        &self,
        index: usize,
        command: &str,
        args: &[String],
    ) -> Result<(Option<Vec<u8>>, bool), PipelineError> {
        let next_stage = self.stages.get(index + 1).ok_or_else(|| {
            PipelineError::Execution("Redirect < requires a file path".to_string())
        })?;
        let path_str = next_stage.command.trim();
        let file_path = if path_str.starts_with("~/") {
            crate::path::home_dir()
                .ok_or_else(|| PipelineError::Execution("HOME not set".to_string()))?
                .join(path_str.trim_start_matches("~/"))
        } else if path_str == "~" {
            crate::path::home_dir()
                .ok_or_else(|| PipelineError::Execution("HOME not set".to_string()))?
        } else {
            std::path::PathBuf::from(path_str)
        };
        let stdin_file = std::fs::File::open(&file_path)
            .map_err(|e| PipelineError::Execution(e.to_string()))?;
        let mut cmd = std::process::Command::new(command);
        cmd.args(args)
            .stdin(Stdio::from(stdin_file))
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let output = cmd
            .output()
            .map_err(|e| PipelineError::Execution(e.to_string()))?;
        let success = output.status.success();
        if !output.stdout.is_empty() {
            let s = String::from_utf8_lossy(&output.stdout);
            print!("{}", s);
        }
        Ok((None, success))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let pipeline = Pipeline::parse("echo hello").unwrap();
        assert_eq!(pipeline.stage_count(), 1);
        assert_eq!(pipeline.first_command(), Some("echo"));
    }

    #[test]
    fn test_parse_sequence() {
        let pipeline = Pipeline::parse("cmd1 ; cmd2").unwrap();
        assert_eq!(pipeline.stage_count(), 2);
    }

    #[test]
    fn test_parse_pipe() {
        let pipeline = Pipeline::parse("echo hi | cat").unwrap();
        assert_eq!(pipeline.stage_count(), 2);
        assert_eq!(pipeline.first_command(), Some("echo"));
    }

    #[test]
    fn test_parse_empty_fails() {
        assert!(Pipeline::parse("").is_err());
    }

    #[test]
    fn test_parse_pipe_incomplete_fails() {
        assert!(Pipeline::parse("echo |").is_err());
    }

    #[test]
    fn test_parse_redirect_in() {
        let pipeline = Pipeline::parse("cat < file").unwrap();
        assert_eq!(pipeline.stage_count(), 2);
    }

    #[test]
    fn test_pipeline_error_display() {
        let err = PipelineError::Parse("test".to_string());
        assert!(!err.to_string().is_empty());
    }
}
