use std::{
    borrow::Cow,
    collections::BTreeMap,
    env,
    fs,
};

use rustyline::completion::Pair;

#[derive(Clone)]
pub struct CommandCompleter {
    commands: BTreeMap<Cow<'static, str>, ()>,
    aliases: BTreeMap<Cow<'static, str>, Cow<'static, str>>,
}

impl CommandCompleter {
    pub fn new() -> Self {
        let mut completer = Self {
            commands: BTreeMap::new(),
            aliases: BTreeMap::new(),
        };
        completer.refresh_commands();
        completer
    }

    pub fn refresh_commands(&mut self) {
        self.commands.clear();
        self.add_builtin_commands();
        self.add_path_commands();
    }

    fn add_builtin_commands(&mut self) {
        self.commands.insert(Cow::Borrowed("cd"), ());
        self.commands.insert(Cow::Borrowed("exit"), ());
    }

    fn add_path_commands(&mut self) {
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

    pub fn complete_command(&self, line: &str) -> Vec<Pair> {
        let mut matches = Vec::new();
        let input = line.trim();

        self.add_command_matches(&mut matches, input);
        self.add_alias_matches(&mut matches, input);

        matches
    }

    fn add_command_matches(&self, matches: &mut Vec<Pair>, input: &str) {
        for cmd in self.commands.keys() {
            if cmd.starts_with(input) {
                matches.push(Pair {
                    display: cmd.to_string(),
                    replacement: cmd.to_string(),
                });
            }
        }
    }

    fn add_alias_matches(&self, matches: &mut Vec<Pair>, input: &str) {
        for alias in self.aliases.keys() {
            if alias.starts_with(input) {
                matches.push(Pair {
                    display: format!("{} (alias)", alias),
                    replacement: alias.to_string(),
                });
            }
        }
    }
} 