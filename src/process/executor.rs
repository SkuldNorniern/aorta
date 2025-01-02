use std::env;
use std::process::{Command, Stdio};

use super::{
    signal,
    ProcessError,
};

use crate::flags::Flags;

pub struct CommandExecutor {
    quiet_mode: bool,
}

impl CommandExecutor {
    pub fn new(flags: &Flags) -> Result<Self, ProcessError> {
        Ok(CommandExecutor {
            quiet_mode: flags.is_set("quiet"),
        })
    }

    pub fn spawn_process(&self, args: &[&str]) -> Result<(), ProcessError> {
        let child = Command::new(args[0])
            .args(&args[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env_clear()
            .envs(env::vars())
            .spawn();

        let mut child = match child {
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
