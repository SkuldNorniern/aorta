use crate::error::ShellError;
use std::path::{Path, PathBuf};
use std::fs;
use std::env;
use std::collections::HashMap;

pub struct Config {
    rc_path: PathBuf,
    profile_path: PathBuf,
    aliases: HashMap<String, String>,
}

impl Config {
    pub fn new() -> Result<Self, ShellError> {
        let home_dir = dirs::home_dir()
            .ok_or(ShellError::HomeDirNotFound)?;

        Ok(Config {
            rc_path: home_dir.join(".aortarc"),
            profile_path: home_dir.join(".profile"),
            aliases: HashMap::new(),
        })
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
            let content = fs::read_to_string(path)
                .map_err(|e| ShellError::ConfigError(path.to_string_lossy().to_string(), e.to_string()))?;

            // Process each line in the config file
            for line in content.lines() {
                let line = line.trim();
                
                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                // Process different types of config lines
                if let Some((key, value)) = parse_env_var(line) {
                    env::set_var(key, value);
                } else if let Some((alias_name, alias_cmd)) = parse_alias(line) {
                    self.aliases.insert(alias_name, alias_cmd);
                }
            }
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
}

fn parse_env_var(line: &str) -> Option<(String, String)> {
    if line.starts_with("export ") {
        let parts: Vec<&str> = line["export ".len()..].splitn(2, '=').collect();
        if parts.len() == 2 {
            let key = parts[0].trim().to_string();
            // Remove quotes if they exist
            let value = parts[1].trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            return Some((key, value));
        }
    }
    None
}

fn parse_alias(line: &str) -> Option<(String, String)> {
    if line.starts_with("alias ") {
        let alias_def = line["alias ".len()..].trim();
        let parts: Vec<&str> = alias_def.splitn(2, '=').collect();
        if parts.len() == 2 {
            let name = parts[0].trim().to_string();
            let command = parts[1].trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            return Some((name, command));
        }
    }
    None
}