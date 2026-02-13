use std::path::PathBuf;

use super::ConfigError;

const DEFAULT_AORTA_HOME: &str = "~/.aorta";
const DEFAULT_RC_PATH: &str = "~/.aortarc";
const DEFAULT_THEME: &str = "minimal";
const DEFAULT_EDITOR: &str = "vi";
const DEFAULT_PROMPT_PRESET: &str = "minimal";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatMode {
    Native,
    Compat,
}

impl CompatMode {
    fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "compat" => Self::Compat,
            _ => Self::Native,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Compat => "compat",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProgramConfig {
    pub aorta_home: PathBuf,
    pub rc_path: PathBuf,
    pub compat_mode: CompatMode,
    pub bootstrap: BootstrapConfig,
}

#[derive(Debug, Clone)]
pub struct BootstrapConfig {
    pub theme: String,
    pub plugins: Vec<String>,
    pub editor: String,
    pub prompt_preset: String,
    pub prompt_format: Option<String>,
}

impl Default for BootstrapConfig {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            plugins: vec!["git".to_string()],
            editor: DEFAULT_EDITOR.to_string(),
            prompt_preset: DEFAULT_PROMPT_PRESET.to_string(),
            prompt_format: None,
        }
    }
}

impl BootstrapConfig {
    pub fn resolved_prompt(&self) -> String {
        if let Some(format) = &self.prompt_format {
            return format.clone();
        }

        match self.prompt_preset.trim().to_ascii_lowercase().as_str() {
            "compact" => "%~ $ ".to_string(),
            "classic" => "%u@%h:%~$ ".to_string(),
            "developer" => "%u@%h %~ [dev]$ ".to_string(),
            _ => "%~ > ".to_string(),
        }
    }
}

impl ProgramConfig {
    pub fn xdg_config_dir() -> Option<PathBuf> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| crate::path::home_dir().map(|h| h.join(".config")))
    }

    pub fn config_toml_path() -> Option<PathBuf> {
        Self::xdg_config_dir().map(|d| d.join("aorta").join("config.toml"))
    }

    pub fn load(home: &PathBuf) -> Result<Self, ConfigError> {
        let config_path = match Self::config_toml_path() {
            Some(p) if p.exists() => p,
            _ => return Ok(Self::default_paths(home)),
        };

        let content = std::fs::read_to_string(&config_path).map_err(ConfigError::IoError)?;
        let parsed: toml::Value = content
            .parse()
            .map_err(|e| ConfigError::ConfigFileNotFound(format!("invalid TOML: {}", e)))?;

        let mut aorta_home = home.join(".aorta");
        let mut rc_path = home.join(".aortarc");
        let mut compat_mode = CompatMode::Native;
        let mut bootstrap = BootstrapConfig::default();

        if let Some(paths) = parsed.get("paths").and_then(|v| v.as_table()) {
            if let Some(v) = paths.get("aorta_home").and_then(|v| v.as_str()) {
                aorta_home = Self::expand_path(v, home);
            }
            if let Some(v) = paths.get("rc_path").and_then(|v| v.as_str()) {
                rc_path = Self::expand_path(v, home);
            }
        }

        if let Some(loader) = parsed.get("loader").and_then(|v| v.as_table()) {
            if let Some(v) = loader.get("compat_mode").and_then(|v| v.as_str()) {
                compat_mode = CompatMode::from_str(v);
            }
        }

        if let Some(config) = parsed.get("bootstrap").and_then(|v| v.as_table()) {
            // Backward-compatible legacy section name.
            if let Some(v) = config.get("theme").and_then(|v| v.as_str()) {
                bootstrap.theme = v.trim().to_string();
            }
            if let Some(v) = config.get("editor").and_then(|v| v.as_str()) {
                bootstrap.editor = v.trim().to_string();
            }
            if let Some(v) = config.get("prompt_preset").and_then(|v| v.as_str()) {
                bootstrap.prompt_preset = v.trim().to_string();
            }
            if let Some(v) = config.get("prompt_format").and_then(|v| v.as_str()) {
                bootstrap.prompt_format = Some(v.to_string());
            }
            if let Some(values) = config.get("plugins").and_then(|v| v.as_array()) {
                let plugins: Vec<String> = values
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect();
                if !plugins.is_empty() {
                    bootstrap.plugins = plugins;
                }
            }
        }

        if let Some(config) = parsed.get("defaults").and_then(|v| v.as_table()) {
            if let Some(v) = config.get("theme").and_then(|v| v.as_str()) {
                bootstrap.theme = v.trim().to_string();
            }
            if let Some(v) = config.get("editor").and_then(|v| v.as_str()) {
                bootstrap.editor = v.trim().to_string();
            }
            if let Some(v) = config.get("prompt_preset").and_then(|v| v.as_str()) {
                bootstrap.prompt_preset = v.trim().to_string();
            }
            if let Some(v) = config.get("prompt_format").and_then(|v| v.as_str()) {
                bootstrap.prompt_format = Some(v.to_string());
            }
            if let Some(values) = config.get("plugins").and_then(|v| v.as_array()) {
                let plugins: Vec<String> = values
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(ToString::to_string)
                    .collect();
                if !plugins.is_empty() {
                    bootstrap.plugins = plugins;
                }
            }
        }

        if std::env::var_os("AORTA_HOME").is_some() {
            aorta_home = std::env::var_os("AORTA_HOME")
                .map(PathBuf::from)
                .unwrap_or(aorta_home);
        }

        Ok(ProgramConfig {
            aorta_home,
            rc_path,
            compat_mode,
            bootstrap,
        })
    }

    fn expand_path(s: &str, home: &PathBuf) -> PathBuf {
        let s = s.trim();
        if s.starts_with("~/") {
            home.join(&s[2..])
        } else if s == "~" {
            home.clone()
        } else {
            PathBuf::from(s)
        }
    }

    fn default_paths(home: &PathBuf) -> Self {
        let aorta_home = std::env::var_os("AORTA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".aorta"));
        Self {
            aorta_home,
            rc_path: home.join(".aortarc"),
            compat_mode: CompatMode::Native,
            bootstrap: BootstrapConfig::default(),
        }
    }

    pub fn ensure_config_dir() -> Result<PathBuf, ConfigError> {
        let dir = Self::xdg_config_dir()
            .ok_or(ConfigError::HomeDirNotFound)?
            .join("aorta");
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        Ok(dir)
    }

    pub fn write_default_config() -> Result<(), ConfigError> {
        let dir = Self::ensure_config_dir()?;
        let path = dir.join("config.toml");
        if path.exists() {
            return Ok(());
        }
        let default = format!(
            r#"# Aorta program config
# Shell config (aliases, theme, plugins) lives in ~/.aortarc

[paths]
aorta_home = "{}"
rc_path = "{}"

[loader]
# native: Aorta syntax only (fast/default)
# compat: extra parser compatibility for zsh/bash-style profile snippets
compat_mode = "{}"

[defaults]
# Used when Aorta first generates ~/.aortarc.

# Theme (ships with: minimal, default, compact, classic, developer)
theme = "{}"

# Plugins are loaded from ~/.aorta/plugins/<name>/<name>.aorta
# Add more names in this list as you install/create plugins.
plugins = ["git"]

# Editor exported into ~/.aortarc
editor = "{}"

# Starship-inspired prompt presets: minimal, compact, classic, developer
prompt_preset = "{}"

# Optional: explicit prompt format (overrides prompt_preset)
# Tokens: %u=user %h=host %~=cwd(short) %c=cwd(full)
# prompt_format = "%u@%h:%~$ "
# prompt_format = "%~ $ "
# prompt_format = "%u@%h %~ [dev]$ "

# Legacy alias still accepted for backward compatibility:
# [bootstrap]
"#,
            DEFAULT_AORTA_HOME,
            DEFAULT_RC_PATH,
            CompatMode::Native.as_str(),
            DEFAULT_THEME,
            DEFAULT_EDITOR,
            DEFAULT_PROMPT_PRESET,
        );
        std::fs::write(&path, default)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_program_config_default() {
        env::set_var("HOME", "/home/user");
        env::remove_var("AORTA_HOME");
        env::remove_var("XDG_CONFIG_HOME");
        let home = PathBuf::from("/home/user");
        let prog = ProgramConfig::load(&home).unwrap();
        assert_eq!(prog.rc_path, PathBuf::from("/home/user/.aortarc"));
        assert_eq!(prog.aorta_home, PathBuf::from("/home/user/.aorta"));
        assert_eq!(prog.compat_mode, CompatMode::Native);
        assert_eq!(prog.bootstrap.theme, DEFAULT_THEME);
        assert_eq!(prog.bootstrap.editor, DEFAULT_EDITOR);
        assert_eq!(prog.bootstrap.plugins, vec!["git".to_string()]);
        assert_eq!(prog.bootstrap.resolved_prompt(), "%~ > ");
    }

    #[test]
    fn test_xdg_config_dir_falls_back_to_home() {
        env::set_var("HOME", "/home/user");
        env::remove_var("XDG_CONFIG_HOME");
        let dir = ProgramConfig::xdg_config_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/home/user/.config"));
    }

    #[test]
    fn test_program_config_compat_mode_from_toml() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = env::temp_dir().join(format!("aorta_progcfg_{}", nonce));
        let config_dir = base.join("aorta");
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.toml"),
            r#"[paths]
aorta_home = "~/.aorta"
rc_path = "~/.aortarc"

[loader]
compat_mode = "compat"

[defaults]
theme = "default"
plugins = ["git", "docker"]
editor = "nvim"
prompt_preset = "classic"
"#,
        )
        .unwrap();

        env::set_var("HOME", "/home/user");
        env::set_var("XDG_CONFIG_HOME", &base);

        let prog = ProgramConfig::load(&PathBuf::from("/home/user")).unwrap();
        assert_eq!(prog.compat_mode, CompatMode::Compat);
        assert_eq!(prog.bootstrap.theme, "default");
        assert_eq!(prog.bootstrap.editor, "nvim");
        assert_eq!(prog.bootstrap.plugins, vec!["git", "docker"]);
        assert_eq!(prog.bootstrap.resolved_prompt(), "%u@%h:%~$ ");

        let _ = std::fs::remove_dir_all(base);
        env::remove_var("XDG_CONFIG_HOME");
    }
}
