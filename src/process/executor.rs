use std::env;
use std::process::{Command, Stdio};

use super::{signal, ProcessError};
use crate::flags::Flags;
use crate::path::PathExpander;

#[derive(Clone)]
pub struct CommandExecutor {
    quiet_mode: bool,
    path_expander: PathExpander,
}

impl CommandExecutor {
    pub fn new(flags: &Flags) -> Result<Self, ProcessError> {
        Ok(CommandExecutor {
            quiet_mode: flags.is_set("quiet"),
            path_expander: PathExpander::new(),
        })
    }

    pub fn spawn_process(&self, args: &[&str]) -> Result<(), ProcessError> {
        let expanded_args: Vec<String> = args
            .iter()
            .map(|&arg| {
                if arg.contains('~') {
                    self.path_expander
                        .expand(arg)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_else(|_| arg.to_owned())
                } else {
                    arg.to_owned()
                }
            })
            .collect();

        let mut command = Command::new(&expanded_args[0]);
        command
            .args(&expanded_args[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env_clear()
            .envs(std::env::vars());

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    if !self.quiet_mode {
                        eprintln!("aorta: command not found: {}", args[0]);
                    }
                    return Ok(());
                }
                return Err(e.into());
            }
        };

        let _pid = child.id();
        signal::setup_signal_handlers()?;

        match child.wait() {
            Ok(status) => {
                if !status.success() && !self.quiet_mode {
                    println!("Process exited with status: {}", status);
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(ProcessError::CommandNotFound(args[0].to_string()))
                } else {
                    Err(e.into())
                }
            }
        }
    }
}
