use crate::error::ShellError;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Config {
    rc_path: PathBuf,
    profile_path: PathBuf,
    aliases: HashMap<String, String>,
    env_vars: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Result<Self, ShellError> {
        let home = env::var("HOME").map_err(|_| ShellError::HomeDirNotFound)?;
        let home_path = PathBuf::from(home);

        let mut config = Config {
            rc_path: home_path.join(".aortarc"),
            profile_path: home_path.join(".profile"),
            aliases: HashMap::new(),
            env_vars: HashMap::new(),
        };

        // Initialize with current environment
        for (key, value) in std::env::vars() {
            config.env_vars.insert(key, value);
        }

        Ok(config)
    }

    pub fn get_alias(&self, cmd: &str) -> Option<&String> {
        self.aliases.get(cmd)
    }

    pub fn load(&mut self) -> Result<(), ShellError> {
        // Store paths locally to avoid self-referential borrows
        let profile_path = self.profile_path.clone();
        let rc_path = self.rc_path.clone();

        // Load .profile first (if it exists)
        self.source_if_exists(&profile_path)?;

        // Then load .aortarc (if it exists)
        self.source_if_exists(&rc_path)?;

        Ok(())
    }

    fn source_if_exists(&mut self, path: &Path) -> Result<(), ShellError> {
        if path.exists() {
            let content = fs::read_to_string(path).map_err(|e| {
                ShellError::ConfigError(path.to_string_lossy().to_string(), e.to_string())
            })?;

            // Process each line in the config file
            for line in content.lines() {
                self.process_line(line)?;
            }
        }
        Ok(())
    }

    fn process_line(&mut self, line: &str) -> Result<(), ShellError> {
        // Skip empty lines and comments
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            return Ok(());
        }

        // Handle export statements
        if let Some(var_def) = line.strip_prefix("export ") {
            let var_def = var_def.trim();
            return self.process_env_var(var_def);
        }

        // Handle direct PATH assignments
        if let Some(path_value) = line.strip_prefix("PATH=") {
            return self.process_path_var(path_value);
        }

        // Handle aliases
        if line.starts_with("alias ") {
            return self.process_alias(line);
        }

        Ok(())
    }

    fn process_env_var(&mut self, var_def: &str) -> Result<(), ShellError> {
        if let Some((name, value)) = var_def.split_once('=') {
            let name = name.trim();
            let mut value = value.trim();

            // Remove quotes if present
            if value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }

            // Expand any variables in the value
            let expanded_value = self.expand_value(value);

            // Store in our internal map and set system env var
            self.env_vars
                .insert(name.to_string(), expanded_value.clone());
            std::env::set_var(name, expanded_value);
        }
        Ok(())
    }

    fn process_path_var(&mut self, value: &str) -> Result<(), ShellError> {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut new_path = value.replace("$PATH", &current_path);

        // Expand $HOME
        if let Some(home) = std::env::var_os("HOME") {
            new_path = new_path.replace("$HOME", home.to_str().unwrap());
        }

        // Store in our internal map and set system PATH
        self.env_vars.insert("PATH".to_string(), new_path.clone());
        std::env::set_var("PATH", new_path);
        Ok(())
    }

    fn process_alias(&mut self, line: &str) -> Result<(), ShellError> {
        if let Some((name, command)) = line["alias ".len()..].split_once('=') {
            let name = name.trim();
            let mut command = command.trim();

            // Remove surrounding quotes if present
            if (command.starts_with('\'') && command.ends_with('\''))
                || (command.starts_with('"') && command.ends_with('"'))
            {
                command = &command[1..command.len() - 1];
            }

            self.aliases.insert(name.to_string(), command.to_string());
        }
        Ok(())
    }

    pub fn expand_aliases(&self, command: &str) -> String {
        let mut parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(first_word) = parts.first() {
            if let Some(alias_value) = self.get_alias(first_word) {
                // Replace the first word with the alias value
                parts[0] = alias_value;
            }
        }
        parts.join(" ")
    }

    pub fn get_aliases(&self) -> std::collections::BTreeMap<String, String> {
        self.aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    fn expand_value(&self, value: &str) -> String {
        let mut result = value.to_string();

        // Expand $HOME
        if let Ok(home) = std::env::var("HOME") {
            result = result.replace("$HOME", &home);
        }

        // Expand $PATH
        if let Ok(path) = std::env::var("PATH") {
            result = result.replace("$PATH", &path);
        }

        result
    }
}
