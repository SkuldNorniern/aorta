use crate::completer::ShellCompleter;
use crate::config::Config;
use crate::error::ShellError;
use crate::flags::Flags;
use crate::history::History;
use rustyline::DefaultEditor;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct Shell {
    editor: DefaultEditor,
    current_dir: String,
    config: Config,
    completer: ShellCompleter,
    history: History,
    flags: Flags,
}

impl Shell {
    pub fn new(flags: Flags) -> Result<Self, ShellError> {
        let editor = DefaultEditor::new()?;
        let current_dir = env::current_dir()?.to_string_lossy().to_string();
        let mut config = Config::new()?;

        // Load config before setting up other components
        config.load()?;

        // After loading config, update the current process environment
        if let Some(path) = env::var_os("PATH") {
            env::set_var("PATH", path); // Refresh PATH in current process
        }

        let completer = ShellCompleter::new();

        // Setup history with default values
        let history_file = dirs::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?
            .join(".aorta_history");
        let history = History::new(history_file, 1000)?;

        // Set up initial CTRL-C handler
        // let _r = running.clone();
        ctrlc::set_handler(move || {
            // Do nothing - this prevents the shell from exiting
            println!("\nUse 'exit' to exit the shell");
        })?;

        Ok(Shell {
            editor,
            current_dir,
            config,
            completer,
            history,
            flags,
        })
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        self.register_as_shell()?;

        // Update completer with current aliases
        self.completer.update_aliases(self.config.get_aliases());

        loop {
            let prompt = format!("{} > ", self.current_dir);
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    // Add to history
                    self.history.add(line.clone())?;

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

        // Expand environment variables
        let expanded_command = self.expand_env_vars(&expanded_command);

        let args: Vec<&str> = expanded_command.split_whitespace().collect();

        if args.is_empty() {
            return Ok(());
        }

        match args[0] {
            "cd" => self.change_directory(args.get(1).copied())?,
            "exit" => std::process::exit(0),
            _ => self.spawn_process(&args)?,
        }
        Ok(())
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

    fn change_directory(&mut self, path: Option<&str>) -> Result<(), ShellError> {
        let new_path = path.unwrap_or("~");
        let path_buf: PathBuf = if new_path == "~" {
            dirs::home_dir().ok_or(ShellError::HomeDirNotFound)?
        } else {
            Path::new(new_path).to_path_buf()
        };

        env::set_current_dir(&path_buf)?;
        self.current_dir = env::current_dir()?.to_string_lossy().to_string();
        Ok(())
    }

    fn spawn_process(&self, args: &[&str]) -> Result<(), ShellError> {
        let child = Command::new(args[0])
            .args(&args[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .env_clear()
            .envs(env::vars())
            .spawn();

        // Handle the spawn result first
        let mut child = match child {
            Ok(child) => child,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    eprintln!("aorta: command not found: {}", args[0]);
                    return Ok(()); // Return early with Ok to avoid showing additional error messages
                }
                return Err(e.into());
            }
        };

        // Store the child's pid
        let _pid = child.id();

        // Set up signal handling using a different approach
        unsafe {
            // Install a signal handler for SIGINT
            libc::signal(libc::SIGINT, handle_sigint as libc::sighandler_t);
        }

        // Wait for the child process to complete
        

        match child.wait() {
            Ok(status) => {
                if !status.success() {
                    println!("Process exited with status: {}", status);
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Err(ShellError::CommandNotFound(args[0].to_string()))
                } else {
                    Err(e.into())
                }
            }
        }
    }
}

// External signal handler
extern "C" fn handle_sigint(_: i32) {
    // Do nothing, let the child process handle the signal
}
