use crate::error::ShellError;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Flags {
    flags: HashMap<String, Flag>,
}

#[derive(Debug, Clone)]
pub struct Flag {
    pub short: String,
    pub long: String,
    pub description: String,
    pub value: Option<String>,
}

impl Default for Flags {
    fn default() -> Self {
        Self::new()
    }
}

impl Flags {
    pub fn new() -> Self {
        let mut flags = HashMap::new();

        // Add default flags similar to neofetch style
        flags.insert(
            "help".to_string(),
            Flag {
                short: "-h".to_string(),
                long: "--help".to_string(),
                description: "Print this help message".to_string(),
                value: None,
            },
        );

        flags.insert(
            "version".to_string(),
            Flag {
                short: "-v".to_string(),
                long: "--version".to_string(),
                description: "Show version information".to_string(),
                value: None,
            },
        );

        flags.insert(
            "config".to_string(),
            Flag {
                short: "-c".to_string(),
                long: "--config".to_string(),
                description: "Specify custom config file path".to_string(),
                value: None,
            },
        );

        flags.insert(
            "quiet".to_string(),
            Flag {
                short: "-q".to_string(),
                long: "--quiet".to_string(),
                description: "Suppress output".to_string(),
                value: None,
            },
        );

        flags.insert(
            "debug".to_string(),
            Flag {
                short: "-d".to_string(),
                long: "--debug".to_string(),
                description: "Enable debug output".to_string(),
                value: None,
            },
        );

        Flags { flags }
    }

    pub fn parse(&mut self, args: &[String]) -> Result<(), ShellError> {
        let mut i = 0;
        while i < args.len() {
            let arg = &args[i];

            // Check for both short and long flags
            for flag in self.flags.values_mut() {
                if arg == &flag.short || arg == &flag.long {
                    // Check if the flag expects a value
                    if arg == "-c" || arg == "--config" {
                        if i + 1 < args.len() {
                            flag.value = Some(args[i + 1].clone());
                            i += 1;
                        } else {
                            return Err(ShellError::FlagError(format!(
                                "Flag {} requires a value",
                                arg
                            )));
                        }
                    } else {
                        flag.value = Some("true".to_string());
                    }
                }
            }
            i += 1;
        }
        Ok(())
    }

    pub fn is_set(&self, name: &str) -> bool {
        self.flags
            .get(name)
            .and_then(|f| f.value.as_ref())
            .is_some()
    }

    pub fn get_value(&self, name: &str) -> Option<&String> {
        self.flags.get(name).and_then(|f| f.value.as_ref())
    }

    pub fn print_help(&self) {
        println!("Usage: aorta [OPTIONS]");
        println!("\nOptions:");
        for flag in self.flags.values() {
            println!("  {}, {:<15} {}", flag.short, flag.long, flag.description);
        }
    }
}
