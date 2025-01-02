use std::env;

use crate::{
    core::{commands::CommandExecutor, config::Config},
    error::ShellError,
    flags::Flags,
    input::{History, ShellCompleter},
};

use rustyline::{config::Configurer, history::FileHistory, Editor};

pub struct Shell {
    editor: Editor<ShellCompleter, FileHistory>,
    current_dir: String,
    config: Config,
    completer: ShellCompleter,
    history: History,
    flags: Flags,
    executor: CommandExecutor,
}

impl Shell {
    pub fn new(flags: Flags) -> Result<Self, ShellError> {
        let completer = ShellCompleter::new();
        let mut editor = Editor::<ShellCompleter, FileHistory>::new()?;

        // First create the editor, then set the helper
        editor.set_helper(Some(completer.clone()));
        editor.set_auto_add_history(true);

        let current_dir = env::current_dir()?.to_string_lossy().to_string();
        let mut config = Config::new()?;

        // Load config before setting up other components
        config.load()?;

        // After loading config, update the current process environment
        if let Some(path) = env::var_os("PATH") {
            env::set_var("PATH", path);
        }

        let history_file = dirs::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?
            .join(".aorta_history");
        let history = History::new(history_file, 1000)?;

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

        loop {
            let prompt = format!("{} > ", self.current_dir);
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Add to history
                    self.history.add(&line)?;

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

    fn execute_command(&mut self, command: &str) -> Result<(), ShellError> {
        // Expand aliases before processing the command
        let expanded_command = self.config.expand_aliases(command);
        let expanded_command = self.expand_env_vars(&expanded_command);

        // Convert to owned Strings for the command system
        let args: Vec<&str> = expanded_command.split_whitespace().collect();

        if args.is_empty() {
            return Ok(());
        }

        let command_name = args[0];
        let command_args: Vec<String> = args[1..].iter().map(|&s| s.to_string()).collect();

        // First try to execute as a built-in command
        match self.executor.execute(command_name, &command_args) {
            Ok(_) => {
                // Update current_dir after command execution
                self.current_dir = env::current_dir()?
                    .to_string_lossy()
                    .to_string();
                Ok(())
            }
            Err(e) => Err(ShellError::CommandError(e)),
        }
    }

    fn expand_env_vars(&self, input: &str) -> String {
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
            if let Ok(value) = std::env::var(var_name) {
                result.replace_range(dollar_pos..var_end, &value);
            } else {
                // If variable not found, replace with empty string
                result.replace_range(dollar_pos..var_end, "");
            }
        }

        result
    }
}
