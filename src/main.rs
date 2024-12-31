use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug)]
struct Shell {
    editor: DefaultEditor,
    current_dir: String,
}

impl Shell {
    fn new() -> Self {
        let editor = DefaultEditor::new().unwrap();
        let current_dir = env::current_dir().unwrap().to_string_lossy().to_string();

        Shell {
            editor,
            current_dir,
        }
    }

    fn run(&mut self) {
        loop {
            let prompt = format!("{} > ", self.current_dir);
            match self.editor.readline(&prompt) {
                Ok(line) => {
                    self.editor.add_history_entry(line.as_str()).unwrap();
                    self.execute_command(&line);
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
    }

    fn execute_command(&mut self, command: &str) {
        let args: Vec<&str> = command.split_whitespace().collect();

        if args.is_empty() {
            return;
        }

        match args[0] {
            "cd" => self.change_directory(args.get(1).copied()),
            "exit" => std::process::exit(0),
            _ => self.spawn_process(&args),
        }
    }

    fn change_directory(&mut self, path: Option<&str>) {
        let new_path = path.unwrap_or("~");
        let path_buf = if new_path == "~" {
            dirs::home_dir().unwrap()
        } else {
            Path::new(new_path).to_path_buf()
        };

        if let Err(e) = env::set_current_dir(&path_buf) {
            println!("cd: {}: {}", new_path, e);
            return;
        }

        self.current_dir = env::current_dir().unwrap().to_string_lossy().to_string();
    }

    fn spawn_process(&self, args: &[&str]) {
        let result = Command::new(args[0])
            .args(&args[1..])
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output();

        match result {
            Ok(output) if !output.status.success() => {
                println!("Process exited with status: {}", output.status);
            }
            Err(e) => println!("Error executing command: {}", e),
            _ => {}
        }
    }
}

fn main() {
    println!("Welcome to Aorta Shell!");
    println!("Type 'exit' to quit.");

    let mut shell = Shell::new();
    shell.run();
}
