use std::path::PathBuf;

use super::ConfigError;

const DEFAULT_AORTA_HOME: &str = "~/.aorta";
const DEFAULT_RC_PATH: &str = "~/.aortarc";

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

        if std::env::var_os("AORTA_HOME").is_some() {
            aorta_home = std::env::var_os("AORTA_HOME")
                .map(PathBuf::from)
                .unwrap_or(aorta_home);
        }

        Ok(ProgramConfig {
            aorta_home,
            rc_path,
            compat_mode,
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
"#,
            DEFAULT_AORTA_HOME,
            DEFAULT_RC_PATH,
            CompatMode::Native.as_str(),
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
"#,
        )
        .unwrap();

        env::set_var("HOME", "/home/user");
        env::set_var("XDG_CONFIG_HOME", &base);

        let prog = ProgramConfig::load(&PathBuf::from("/home/user")).unwrap();
        assert_eq!(prog.compat_mode, CompatMode::Compat);

        let _ = std::fs::remove_dir_all(base);
        env::remove_var("XDG_CONFIG_HOME");
    }
}
