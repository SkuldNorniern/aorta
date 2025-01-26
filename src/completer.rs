use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use crate::highlight::SyntaxHighlighter;

#[derive(Clone)]
pub struct ShellCompleter {
    commands: BTreeMap<Cow<'static, str>, ()>,
    aliases: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
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
        self.commands.insert(Cow::Borrowed("cd"), ());
        self.commands.insert(Cow::Borrowed("exit"), ());

        // Add commands from PATH
        if let Some(path_var) = env::var_os("PATH") {
            for path in env::split_paths(&path_var) {
                if let Ok(entries) = fs::read_dir(path) {
                    for entry in entries.filter_map(Result::ok) {
                        if let Ok(file_type) = entry.file_type() {
                            if file_type.is_file() || file_type.is_symlink() {
                                if let Some(name) = entry.file_name().to_str() {
                                    self.commands.insert(Cow::Owned(name.to_string()), ());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_aliases(&mut self, aliases: BTreeMap<Cow<'_, str>, Cow<'_, str>>) {
        self.aliases = aliases
            .into_iter()
            .map(|(k, v)| (Cow::Owned(k.into_owned()), Cow::Owned(v.into_owned())))
            .collect();
    }

    fn complete_command(&self, line: &str) -> Vec<Pair> {
        let mut matches = Vec::new();
        let input = line.trim();

        // Complete commands
        for cmd in self.commands.keys() {
            if cmd.starts_with(input) {
                matches.push(Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(), // Don't add space here
                });
            }
        }

        // Complete aliases
        for alias in self.aliases.keys() {
            if alias.starts_with(input) {
                matches.push(Pair {
                    display: format!("{} (alias)", alias),
                    replacement: alias.to_string(), // Don't add space here
                });
            }
        }

        matches
    }

    fn complete_path(&self, incomplete: &str) -> Vec<Pair> {
        let (dir_to_search, file_prefix, is_absolute) = self.parse_path_input(incomplete);
        let base_path = if is_absolute { PathBuf::from("/") } else { PathBuf::new() };
        
        self.get_path_matches(&dir_to_search, file_prefix, is_absolute, base_path)
    }

    fn parse_path_input(&self, incomplete: &str) -> (PathBuf, String, bool) {
        let path = Path::new(incomplete);
        let is_absolute = incomplete.starts_with('/');

        let (dir_to_search, file_prefix) = if incomplete.is_empty() {
            (PathBuf::from("."), String::new())
        } else if incomplete.ends_with('/') {
            (PathBuf::from(incomplete), String::new())
        } else if let Some(parent) = path.parent() {
            (
                if parent.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                },
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
            )
        } else {
            (PathBuf::from("."), incomplete.to_string())
        };

        (dir_to_search, file_prefix, is_absolute)
    }

    fn get_path_matches(
        &self,
        dir_to_search: &Path,
        file_prefix: String,
        is_absolute: bool,
        base_path: PathBuf,
    ) -> Vec<Pair> {
        let mut matches = Vec::new();

        if let Ok(entries) = fs::read_dir(dir_to_search) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&file_prefix) {
                        if let Some(pair) = self.create_completion_pair(
                            name,
                            &entry.path(),
                            dir_to_search,
                            is_absolute,
                            &base_path,
                        ) {
                            matches.push(pair);
                        }
                    }
                }
            }
        }

        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches
    }

    fn create_completion_pair(
        &self,
        name: &str,
        path: &Path,
        dir_to_search: &Path,
        is_absolute: bool,
        base_path: &Path,
    ) -> Option<Pair> {
        let is_dir = path.is_dir();
        
        let relative_path = if dir_to_search == Path::new(".") {
            name.to_string()
        } else {
            let mut full_path = if is_absolute {
                base_path.join(dir_to_search)
            } else {
                dir_to_search.to_path_buf()
            };
            full_path.push(name);
            full_path.to_string_lossy().into_owned()
        };

        let (display, replacement) = if is_dir {
            (
                format!("{}/", relative_path),
                format!("{}/", relative_path),
            )
        } else {
            (
                relative_path.clone(),
                format!("{} ", relative_path),
            )
        };

        Some(Pair { display, replacement })
    }
}

impl Helper for ShellCompleter {}
impl Highlighter for ShellCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        // Create highlighter if not already cached
        let highlighter = SyntaxHighlighter::new();
        Cow::Owned(highlighter.highlight_command(line))
    }

    fn highlight_error<'l>(&self, line: &'l str) -> Cow<'l, str> {
        let highlighter = SyntaxHighlighter::new();
        Cow::Owned(highlighter.highlight_error(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        true
    }
}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Validator for ShellCompleter {}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Get the text up to the cursor position
        let line_up_to_cursor = &line[..pos];

        // Split into words and get the word being completed
        let mut words: Vec<&str> = line_up_to_cursor.split_whitespace().collect();

        // If the line ends with a space, add an empty word
        if line_up_to_cursor.ends_with(' ') {
            words.push("");
        }

        match words.len() {
            0 => Ok((0, self.complete_command(""))),
            1 => {
                let word = words[0];
                let start = line_up_to_cursor.rfind(word).unwrap_or(0);
                Ok((start, self.complete_command(word)))
            }
            _ => {
                let last_word = words.last().unwrap_or(&"");
                let start = if last_word.is_empty() {
                    pos
                } else {
                    line_up_to_cursor.rfind(last_word).unwrap_or(pos)
                };
                Ok((start, self.complete_path(last_word)))
            }
        }
    }
}
