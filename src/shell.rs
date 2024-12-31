use crate::error::ShellError;
use rustyline::DefaultEditor;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use crate::config::Config;

pub struct Shell {
    editor: DefaultEditor,
    current_dir: String,
    config: Config,
}

impl Shell {
    pub fn new() -> Result<Self, ShellError> {
        let editor = DefaultEditor::new()?;
        let current_dir = env::current_dir()?.to_string_lossy().to_string();
        let mut config = Config::new()?;
        config.load()?;

        Ok(Shell {
            editor,
            current_dir,
            config,
        })
    }

    pub fn run(&mut self) -> Result<(), ShellError> {
        self.register_as_shell()?;

        loop {
            let prompt = format!("{} > ", self.current_dir);
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    if let Err(e) = self.editor.add_history_entry(line.as_str()) {
                        eprintln!("Warning: Couldn't add to history: {}", e);
                    }
                    
                    if let Err(e) = self.execute_command(&line) {
                        eprintln!("{}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
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
        let result = Command::new(args[0])
            .args(&args[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();

        match result {
            Ok(output) => {
                if !output.status.success() {
                    println!("Process exited with status: {}", output.status);
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
