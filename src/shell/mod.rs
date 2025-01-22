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
    input::{History, ShellCompleter},
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
        let completer = ShellCompleter::new();
        let mut editor = Editor::<ShellCompleter, FileHistory>::new()?;

        editor.set_helper(Some(completer.clone()));
        editor.set_auto_add_history(true);

        let current_dir = env::current_dir()?.to_string_lossy().to_string();

        // Load config and executor
        let executor = CommandExecutor::new(&flags)?;
        let mut config = Config::new()?.with_executor(executor);
        config.load()?;

        // After loading config, update the current process environment
        if let Some(path) = env::var_os("PATH") {
            env::set_var("PATH", path.clone());
        }

        // Set up history
        let history_file = dirs::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?
            .join(".aorta_history");
        let history = History::new(history_file, 1000)?;

        // Set up ctrl-c handler
        ctrlc::set_handler(move || {
            // println!("\nUse 'exit' to exit the shell");
        })?;

        let executor = CommandExecutor::new(&flags)?;

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
            let prompt = format!("{} > ", self.current_dir);
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

    fn register_as_shell(&self) -> Result<(), ShellError> {
        let current_exe = env::current_exe()
            .map_err(|e| ShellError::PathError(e.to_string()))?;
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
            
            print!("\nWould you like Aorta to attempt automatic registration? (requires sudo) [y/N]: ");
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
            writeln!(stdin, "{}", shell_path)
                .map_err(|e| ShellError::IoError(e.to_string()))?;
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
