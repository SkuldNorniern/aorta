use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    collections::{HashMap, BTreeMap},
    borrow::Cow,
};

use crate::{
    core::commands::{CommandExecutor, CommandError},
};

#[derive(Debug)]
pub enum PipelineOperator {
    Pipe,           // |
    And,           // &&
    Or,            // ||
    Sequence,      // ;
    Redirect,      // >
}

#[derive(Debug)]
pub struct PipelineStage {
    pub command: String,
    pub args: Vec<String>,
    pub operator: Option<PipelineOperator>,
}

#[derive(Debug)]
pub enum PipelineError {
    IoError(std::io::Error),
    CommandError(CommandError),
    ParseError(String),
    ExecutionError(String),
}

impl std::fmt::Display for PipelineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "IO error: {}", err),
            Self::CommandError(err) => write!(f, "Command error: {}", err),
            Self::ParseError(msg) => write!(f, "Parse error: {}", msg),
            Self::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
        }
    }
}

impl From<std::io::Error> for PipelineError {
    fn from(err: std::io::Error) -> Self {
        PipelineError::IoError(err)
    }
}

impl From<CommandError> for PipelineError {
    fn from(err: CommandError) -> Self {
        PipelineError::CommandError(err)
    }
}

pub struct Pipeline {
    stages: Vec<PipelineStage>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
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
                            return Err(PipelineError::ParseError(
                                "Incomplete pipeline: missing command after |".to_string()
                            ));
                        }
                        Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::Pipe))?;
                    }
                    current_command.clear();
                }
                '&' if chars.peek() == Some(&'&') => {
                    chars.next(); // consume second '&'
                    // Check if there's any non-whitespace content after &&
                    let remaining: String = chars.clone().collect();
                    if remaining.trim().is_empty() {
                        return Err(PipelineError::ParseError(
                            "Incomplete command: missing command after &&".to_string()
                        ));
                    }
                    Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::And))?;
                    current_command.clear();
                }
                ';' => {
                    Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::Sequence))?;
                    current_command.clear();
                }
                '>' => {
                    Self::add_stage(&mut stages, &current_command, Some(PipelineOperator::Redirect))?;
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
            return Err(PipelineError::ParseError("Empty pipeline".to_string()));
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
            return Err(PipelineError::ParseError("Empty command".to_string()));
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            return Err(PipelineError::ParseError("Empty command".to_string()));
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
        env_vars: &HashMap<String, String>,
        aliases: &BTreeMap<Cow<'_, str>, Cow<'_, str>>,
        executor: &CommandExecutor
    ) -> Result<(), PipelineError> {
        let mut previous_output: Option<Vec<u8>> = None;

        for (index, stage) in self.stages.iter().enumerate() {
            // First expand aliases and split into parts
            let expanded_parts = if let Some(alias) = aliases.get(stage.command.as_str()) {
                alias.split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
            } else {
                vec![stage.command.clone()]
            };

            let command = expanded_parts[0].clone();
            let mut args = expanded_parts[1..].to_vec();
            args.extend(stage.args.clone());

            match &stage.operator {
                Some(PipelineOperator::Pipe) => {
                    if command == "grep" {
                        if args.is_empty() {
                            return Err(PipelineError::ExecutionError(
                                "grep: no pattern specified".to_string()
                            ));
                        }

                        // Create a temp file for grep input
                        let temp_input = format!("/tmp/aorta_input_{}", std::process::id());
                        
                        // Write previous output or empty string to temp file
                        if let Some(prev_out) = previous_output.take() {
                            std::fs::write(&temp_input, prev_out)?;
                        } else {
                            std::fs::write(&temp_input, "")?;
                        }

                        // Create a temp file for grep output
                        let temp_output = format!("/tmp/aorta_output_{}", std::process::id());

                        // Keep the pattern and any options, add temp file as last argument
                        let mut grep_args = args;
                        grep_args.push(temp_input.clone());

                        // Execute grep through executor
                        executor.execute(&command, &grep_args)
                            .map_err(|e| PipelineError::ExecutionError(e.to_string()))?;

                        // Read the output if it exists
                        if let Ok(output) = std::fs::read(&temp_output) {
                            previous_output = Some(output);
                        } else {
                            // If no output file, try reading from stdout capture
                            let mut cmd = Command::new("grep");
                            cmd.args(&grep_args)
                                .stdout(Stdio::piped())
                                .stderr(Stdio::inherit());

                            let output = cmd.output()
                                .map_err(|e| PipelineError::ExecutionError(e.to_string()))?;
                            previous_output = Some(output.stdout);
                        }

                        // Clean up temp files
                        let _ = std::fs::remove_file(temp_input);
                        let _ = std::fs::remove_file(temp_output);
                    } else {
                        // For other commands (including ls)
                        let mut cmd = Command::new(&command);
                        cmd.args(&args)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::inherit());

                        let output = cmd.output()
                            .map_err(|e| PipelineError::ExecutionError(e.to_string()))?;
                        previous_output = Some(output.stdout);
                    }
                }
                Some(PipelineOperator::And) | Some(PipelineOperator::Or) | Some(PipelineOperator::Sequence) | None => {
                    executor.execute(&command, &args)
                        .map_err(|e| PipelineError::ExecutionError(e.to_string()))?;
                    previous_output = None;
                }
                Some(PipelineOperator::Redirect) => {
                    if let Some(next_stage) = self.stages.get(index + 1) {
                        if let Some(output) = previous_output.take() {
                            std::fs::write(&next_stage.command, output)?;
                        } else {
                            let mut cmd = Command::new(&command);
                            cmd.args(&args)
                                .stdout(Stdio::piped())
                                .stderr(Stdio::inherit());

                            let output = cmd.output()
                                .map_err(|e| PipelineError::ExecutionError(e.to_string()))?;
                            std::fs::write(&next_stage.command, output.stdout)?;
                        }
                        break;
                    } else {
                        return Err(PipelineError::ExecutionError(
                            "Redirect operator requires a file path".to_string()
                        ));
                    }
                }
            }
        }

        // Print remaining output if any
        if let Some(output) = previous_output {
            if !output.is_empty() {
                if let Ok(s) = String::from_utf8(output) {
                    print!("{}", s);
                }
            }
        }

        Ok(())
    }

    fn expand_env_vars(&self, input: &str, env_vars: &HashMap<String, String>) -> String {
        let mut result = input.to_string();
        
        // Handle $VAR style variables
        while let Some(dollar_pos) = result.find('$') {
            if dollar_pos + 1 >= result.len() {
                break;
            }

            // Find the end of the variable name
            let var_end = result[dollar_pos + 1..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map_or(result.len(), |pos| pos + dollar_pos + 1);

            let var_name = &result[dollar_pos + 1..var_end];

            // Get the value from environment
            if let Some(value) = env_vars.get(var_name) {
                result.replace_range(dollar_pos..var_end, value);
            } else {
                // If variable not found, replace with empty string
                result.replace_range(dollar_pos..var_end, "");
            }
        }

        result
    }
} 
