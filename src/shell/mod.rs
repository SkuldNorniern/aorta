use std::env;
use rustyline::{config::Configurer, history::FileHistory, Editor};

mod executor;
mod environment;

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

        // Set up history
        let history_file = dirs::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?
            .join(".aorta_history");
        let history = History::new(history_file, 1000)?;

        // Set up ctrl-c handler
        ctrlc::set_handler(move || {
            println!("\nUse 'exit' to exit the shell");
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
        let current_exe = env::current_exe()?;

        // Check if the shell is in /etc/shells
        let shells = std::fs::read_to_string("/etc/shells")?;
        let shell_path = current_exe.to_string_lossy();

        if !shells.lines().any(|line| line == shell_path) {
            println!("Warning: This shell is not registered in /etc/shells");
            println!("To register, add the following line to /etc/shells:");
            println!("{}", shell_path);
        }
        Ok(())
    }
} 
