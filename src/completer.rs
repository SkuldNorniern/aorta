use rustyline::completion::{Completer, Pair};
use rustyline::Context;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ShellCompleter {
    commands: BTreeMap<String, ()>,
    aliases: BTreeMap<String, String>,
}

impl Default for ShellCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellCompleter {
    pub fn new() -> Self {
        let mut completer = ShellCompleter {
            commands: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };
        completer.refresh_commands();
        completer
    }

    pub fn refresh_commands(&mut self) {
        self.commands.clear();

        // Add built-in commands
        self.commands.insert("cd".to_string(), ());
        self.commands.insert("exit".to_string(), ());

        // Add commands from PATH
        if let Some(path_var) = env::var_os("PATH") {
            for path in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.filter_map(Result::ok) {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() || file_type.is_symlink() {
                                if let Some(name) = entry.file_name().to_str() {
                                    self.commands.insert(name.to_string(), ());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_aliases(&mut self, aliases: BTreeMap<String, String>) {
        self.aliases = aliases;
    }

    fn complete_command(&self, line: &str) -> Vec<Pair> {
        let mut matches = Vec::new();

        // Complete commands
        for cmd in self.commands.keys() {
            if cmd.starts_with(line) {
                matches.push(Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                });
            }
        }

        // Complete aliases
        for alias in self.aliases.keys() {
            if alias.starts_with(line) {
                matches.push(Pair {
                    display: format!("{} (alias)", alias),
                    replacement: alias.clone(),
                });
            }
        }

        matches
    }

    fn complete_path(&self, incomplete: &str) -> Vec<Pair> {
        let mut matches = Vec::new();
        let path = Path::new(incomplete);

        let (dir_to_search, file_prefix) = if let Some(parent) = path.parent() {
            (
                parent.to_path_buf(),
                path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
            )
        } else {
            (PathBuf::from("."), incomplete)
        };

        if let Ok(entries) = fs::read_dir(dir_to_search) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_prefix) {
                        let full_path = entry.path();
                        if full_path.is_dir() {
                            name.to_string().push('/');
                        }
                        matches.push(Pair {
                            display: name.to_string(),
                            replacement: full_path.to_string_lossy().to_string(),
                        });
                    }
                }
            }
        }

        matches
    }
}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_up_to_cursor = &line[..pos];
        let words: Vec<&str> = line_up_to_cursor.split_whitespace().collect();

        let matches = if words.is_empty() {
            Vec::new()
        } else if words.len() == 1 {
            // Completing command name
            self.complete_command(words[0])
        } else {
            // Completing arguments (path completion)
            let current_word = words.last().unwrap();
            self.complete_path(current_word)
        };

        let start = if let Some(last_word) = words.last() {
            pos - last_word.len()
        } else {
            0
        };

        Ok((start, matches))
    }
}
