use rustyline::{config::Configurer, history::FileHistory, Editor};
use std::env;
use std::io::{self, Write};

mod environment;
mod executor;
pub(crate) mod pipeline;

use crate::{
    core::{commands::CommandExecutor, config::Config},
    error::ShellError,
    flags::Flags,
    input::{History, HistoryEntry, ShellCompleter},
};

use executor::CommandHandler;

pub struct Shell {
    pub(crate) editor: Editor<ShellCompleter, FileHistory>,
    pub(crate) current_dir: String,
    pub(crate) config: Config,
    pub(crate) completer: ShellCompleter,
    pub(crate) history: History,
    pub(crate) flags: Flags,
    pub(crate) executor: CommandExecutor,
}

impl Shell {
    pub fn new(flags: Flags) -> Result<Self, ShellError> {
        crate::core::config::ensure_default_config()?;

        let completer = ShellCompleter::new();
        let mut editor = Editor::<ShellCompleter, FileHistory>::new()?;

        editor.set_helper(Some(completer.clone()));
        editor.set_auto_add_history(true);

        let current_dir = env::current_dir()?.to_string_lossy().to_string();

        let executor = CommandExecutor::new(&flags)?;
        let config_path = flags.get_value("config").map(|s| s.as_str());
        let mut config = Config::new(config_path)?.with_executor(executor.clone());
        config.load_with_flags(&flags)?;

        if let Some(path) = env::var_os("PATH") {
            env::set_var("PATH", path.clone());
        }

        let history_file = crate::path::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?
            .join(".aorta_history");
        let history = History::new(history_file.clone(), 1000)?;

        for entry in history.get_recent(1000).into_iter().rev() {
            if let HistoryEntry::Command { command, .. } = entry {
                if let Err(e) = editor.add_history_entry(command.as_ref()) {
                    if !flags.is_set("quiet") {
                        eprintln!("Warning: Failed to add history entry: {}", e);
                    }
                }
            }
        }

        ctrlc::set_handler(move || {})?;

        Ok(Shell {
            editor,
            current_dir,
            config,
            completer,
            history,
            flags,
            executor,
        })
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        self.register_as_shell()?;
        self.completer.refresh_commands();
        self.completer.update_aliases(self.config.get_aliases());

        // Implement the command loop here instead of calling run_command_loop
        loop {
            let prompt = self.format_prompt();
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    if let Err(e) = self.editor.add_history_entry(line.as_str()) {
                        if !self.flags.is_set("quiet") {
                            eprintln!("Warning: Couldn't add to history: {}", e);
                        }
                    }

                    if let Err(e) = self.execute_command(&line) {
                        if !self.flags.is_set("quiet") {
                            eprintln!("{}", e);
                        }
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    if !self.flags.is_set("quiet") {
                        println!("CTRL-C");
                    }
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    if !self.flags.is_set("quiet") {
                        println!("CTRL-D");
                    }
                    break;
                }
                Err(e) => {
                    if !self.flags.is_set("quiet") {
                        eprintln!("Error: {}", e);
                    }
                    continue;
                }
            }
        }
        Ok(())
    }

    fn format_prompt(&self) -> String {
        let home_abbrev = crate::path::home_dir()
            .and_then(|h| h.to_str().map(|s| s.to_string()))
            .unwrap_or_default();
        let cwd = if !home_abbrev.is_empty() && self.current_dir.starts_with(&home_abbrev) {
            format!("~{}", &self.current_dir[home_abbrev.len()..])
        } else {
            self.current_dir.clone()
        };
        let user = env::var("USER").unwrap_or_else(|_| "?".to_string());
        let host = env::var("HOSTNAME").unwrap_or_else(|_| "?".to_string());

        if let Ok(fmt) = env::var("AORTA_PROMPT") {
            fmt.replace("%c", &self.current_dir)
                .replace("%~", &cwd)
                .replace("%u", &user)
                .replace("%h", &host)
        } else {
            format!("{} > ", cwd)
        }
    }

    fn register_as_shell(&self) -> Result<(), ShellError> {
        if self.flags.is_set("skip-register") {
            return Ok(());
        }

        let current_exe = env::current_exe().map_err(|e| ShellError::PathError(e.to_string()))?;
        let shell_path = current_exe.to_string_lossy();

        // Check if the shell is in /etc/shells
        let shells = std::fs::read_to_string("/etc/shells")
            .map_err(|e| ShellError::FileReadError(e.to_string()))?;

        if !shells.lines().any(|line| line == shell_path) {
            println!("\nAorta Shell Registration");
            println!("------------------------");
            println!("This shell is not registered in /etc/shells");
            println!("Registration allows using Aorta as your default shell.");
            println!("\nTo register manually, add this line to /etc/shells:");
            println!("{}", shell_path);

            print!(
                "\nWould you like Aorta to attempt automatic registration? (requires sudo) [y/N]: "
            );
            io::stdout()
                .flush()
                .map_err(|e| ShellError::IoError(e.to_string()))?;

            let mut response = String::new();
            io::stdin()
                .read_line(&mut response)
                .map_err(|e| ShellError::IoError(e.to_string()))?;

            if response.trim().to_lowercase() == "y" {
                match self.perform_shell_registration(&shell_path) {
                    Ok(_) => println!("Successfully registered Aorta in /etc/shells"),
                    Err(e) => println!("Failed to register shell: {}", e),
                }
            } else {
                println!("Shell registration skipped.");
            }
        }
        Ok(())
    }

    fn perform_shell_registration(&self, shell_path: &str) -> Result<(), ShellError> {
        use std::process::Command;

        let mut status = Command::new("sudo")
            .args(["tee", "-a", "/etc/shells"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .map_err(|e| ShellError::ShellRegistrationError(e.to_string()))?;

        if let Some(ref mut stdin) = status.stdin {
            writeln!(stdin, "{}", shell_path).map_err(|e| ShellError::IoError(e.to_string()))?;
        }

        // Wait for the command to complete
        let result = status
            .wait_with_output()
            .map_err(|e| ShellError::ShellRegistrationError(e.to_string()))?;

        if !result.status.success() {
            return Err(ShellError::ShellRegistrationError(
                "Failed to register shell".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_new_fails_without_home() {
        let saved = env::var_os("HOME");
        env::remove_var("HOME");
        let flags = crate::flags::Flags::default();
        let result = Shell::new(flags);
        if let Some(h) = saved {
            env::set_var("HOME", h);
        }
        assert!(result.is_err());
    }
}
