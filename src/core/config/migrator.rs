use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::ConfigError;

#[derive(Debug, Default, Clone)]
pub struct MigrationResult {
    pub source_file: Option<PathBuf>,
    pub aliases: Vec<String>,
    pub exports: Vec<String>,
    pub sources: Vec<String>,
    pub plugins: Vec<String>,
    pub notes: Vec<String>,
}

impl MigrationResult {
    pub fn has_content(&self) -> bool {
        !self.aliases.is_empty()
            || !self.exports.is_empty()
            || !self.sources.is_empty()
            || !self.plugins.is_empty()
            || !self.notes.is_empty()
    }
}

pub fn migrate_from_shell_config(home: &Path) -> Result<Option<MigrationResult>, ConfigError> {
    let candidates = [
        home.join(".zshrc"),
        home.join(".bashrc"),
        home.join(".bash_profile"),
        home.join(".profile"),
        home.join(".config").join("fish").join("config.fish"),
    ];

    for path in candidates {
        if path.exists() {
            let content = fs::read_to_string(&path)?;
            let mut result = parse_config(&content, &path);
            result.source_file = Some(path);
            if result.has_content() {
                return Ok(Some(result));
            }
        }
    }

    Ok(None)
}

fn parse_config(content: &str, path: &Path) -> MigrationResult {
    let mut aliases = BTreeSet::new();
    let mut exports = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut plugins = BTreeSet::new();
    let mut notes = BTreeSet::new();

    let is_fish = path.ends_with("config.fish");

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.contains("starship init") {
            notes.insert(
                "Detected starship init in old config; set [defaults].prompt_preset or prompt_format in ~/.config/aorta/config.toml.".to_string(),
            );
        }

        if line.starts_with("plugins=(") && line.ends_with(')') {
            let inner = &line[9..line.len() - 1];
            for p in inner.split_whitespace() {
                let p = p.trim().trim_matches('\'').trim_matches('"');
                if !p.is_empty() {
                    plugins.insert(p.to_string());
                }
            }
            continue;
        }

        if let Some(theme) = line.strip_prefix("ZSH_THEME=") {
            let theme = theme.trim().trim_matches('\'').trim_matches('"');
            if !theme.is_empty() {
                notes.insert(format!(
                    "Detected ZSH_THEME={} (Aorta uses `theme <name>` files under ~/.aorta/themes).",
                    theme
                ));
            }
            continue;
        }

        if is_fish {
            if let Some(exported) = convert_fish_export(line) {
                exports.insert(exported);
                continue;
            }
            if let Some(alias) = convert_fish_alias(line) {
                aliases.insert(alias);
                continue;
            }
            if let Some(src) = parse_source_line(line) {
                sources.insert(src);
                continue;
            }
        } else {
            if let Some(alias) = parse_alias_line(line) {
                aliases.insert(alias);
                continue;
            }
            if let Some(exp) = parse_export_line(line) {
                exports.insert(exp);
                continue;
            }
            if let Some(src) = parse_source_line(line) {
                sources.insert(src);
                continue;
            }
        }
    }

    MigrationResult {
        source_file: None,
        aliases: aliases.into_iter().collect(),
        exports: exports.into_iter().collect(),
        sources: sources.into_iter().collect(),
        plugins: plugins.into_iter().collect(),
        notes: notes.into_iter().collect(),
    }
}

fn parse_alias_line(line: &str) -> Option<String> {
    let body = line.strip_prefix("alias ")?;
    let (name, value) = body.split_once('=')?;
    let name = name.trim();
    let value = value.trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }
    Some(format!("alias {}={}", name, value))
}

fn parse_export_line(line: &str) -> Option<String> {
    if !line.starts_with("export ") {
        return None;
    }
    let body = line.trim_start_matches("export ").trim();
    if body.is_empty() || !body.contains('=') {
        return None;
    }
    Some(format!("export {}", body))
}

fn parse_source_line(line: &str) -> Option<String> {
    if let Some(path) = line.strip_prefix("source ") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(format!("source {}", path));
        }
    }

    if let Some(path) = line.strip_prefix(". ") {
        let path = path.trim();
        if !path.is_empty() {
            return Some(format!("source {}", path));
        }
    }

    None
}

fn convert_fish_export(line: &str) -> Option<String> {
    if !line.starts_with("set -gx ") {
        return None;
    }
    let rest = line.trim_start_matches("set -gx ").trim();
    let mut parts = rest.split_whitespace();
    let name = parts.next()?;
    let values: Vec<&str> = parts.collect();
    if name.is_empty() || values.is_empty() {
        return None;
    }
    let value = values.join(" ");
    Some(format!("export {}={}", name, value))
}

fn convert_fish_alias(line: &str) -> Option<String> {
    if !line.starts_with("alias ") {
        return None;
    }

    let rest = line.trim_start_matches("alias ").trim();
    let mut name_and_value = rest.splitn(2, ' ');
    let name = name_and_value.next()?.trim();
    let value = name_and_value.next()?.trim();
    if name.is_empty() || value.is_empty() {
        return None;
    }

    Some(format!("alias {}={}", name, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bash_zsh_lines() {
        let content = r#"
            alias ll='ls -la'
            export EDITOR=nvim
            source "$HOME/.cargo/env"
            plugins=(git docker)
            ZSH_THEME="agnoster"
            eval "$(starship init zsh)"
        "#;
        let path = PathBuf::from("/tmp/.zshrc");
        let result = parse_config(content, &path);

        assert!(result.aliases.contains(&"alias ll='ls -la'".to_string()));
        assert!(result.exports.contains(&"export EDITOR=nvim".to_string()));
        assert!(result
            .sources
            .contains(&"source \"$HOME/.cargo/env\"".to_string()));
        assert!(result.plugins.contains(&"git".to_string()));
        assert!(result.plugins.contains(&"docker".to_string()));
        assert!(!result.notes.is_empty());
    }

    #[test]
    fn test_parse_fish_lines() {
        let content = r#"
            set -gx EDITOR nvim
            alias ll 'ls -la'
            source ~/.cargo/env
        "#;
        let path = PathBuf::from("/tmp/config.fish");
        let result = parse_config(content, &path);
        assert!(result.exports.contains(&"export EDITOR=nvim".to_string()));
        assert!(result.aliases.contains(&"alias ll='ls -la'".to_string()));
        assert!(result.sources.contains(&"source ~/.cargo/env".to_string()));
    }
}
