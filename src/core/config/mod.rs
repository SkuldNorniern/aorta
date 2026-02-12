use std::{borrow::Cow, collections::BTreeMap, fmt};

mod aliases;
mod bootstrap;
mod env_vars;
mod loader;
mod paths;
pub(super) mod program;

use super::commands::{CommandError, CommandExecutor};
use crate::flags::Flags;
use aliases::AliasManager;
use env_vars::EnvVarManager;
use loader::ConfigLoader;
use paths::ConfigPaths;

pub struct Config {
    paths: ConfigPaths,
    aliases: AliasManager,
    env_vars: EnvVarManager,
    executor: Option<CommandExecutor>,
}

pub fn ensure_default_config() -> Result<(), ConfigError> {
    let home = crate::path::home_dir().ok_or(ConfigError::HomeDirNotFound)?;
    program::ProgramConfig::write_default_config()?;
    bootstrap::ensure_aortarc(&home)?;
    bootstrap::ensure_aorta_home(&home)?;
    Ok(())
}

impl Config {
    pub fn new(custom_config_path: Option<&str>) -> Result<Self, ConfigError> {
        let paths = ConfigPaths::new(custom_config_path)?;
        let aliases = AliasManager::new();
        let env_vars = EnvVarManager::new();

        Ok(Config {
            paths,
            aliases,
            env_vars,
            executor: None,
        })
    }

    pub fn with_executor(mut self, executor: CommandExecutor) -> Self {
        self.executor = Some(executor);
        self
    }

    pub fn execute_command(&self, line: &str) -> Result<(), ConfigError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        let parts: Vec<String> = line.split_whitespace().map(String::from).collect();
        if parts.is_empty() {
            return Ok(());
        }

        let command = &parts[0];
        let args = &parts[1..];

        if let Some(executor) = &self.executor {
            executor
                .execute(command, args)
                .map_err(ConfigError::CommandError)?;
        }

        Ok(())
    }

    pub fn load_with_flags(&mut self, flags: &Flags) -> Result<(), ConfigError> {
        let paths = self.paths.clone();
        let loader = ConfigLoader::new(&paths);
        loader.load_configs(self, flags)?;
        Ok(())
    }

    pub fn load(&mut self) -> Result<(), ConfigError> {
        self.load_with_flags(&Flags::new())
    }

    pub fn get_alias<'a>(&'a self, cmd: &str) -> Option<Cow<'a, str>> {
        self.aliases.get(cmd)
    }

    pub fn expand_aliases<'a>(&'a self, command: &'a str) -> Cow<'a, str> {
        self.aliases.expand_command(command)
    }

    pub fn get_aliases(&self) -> BTreeMap<Cow<'_, str>, Cow<'_, str>> {
        self.aliases.get_all()
    }
}

#[derive(Debug)]
pub enum ConfigError {
    HomeDirNotFound,
    EnvVarNotFound(String),
    ConfigFileNotFound(String),
    IoError(std::io::Error),
    CommandError(CommandError),
}

impl From<std::io::Error> for ConfigError {
    fn from(e: std::io::Error) -> Self {
        ConfigError::IoError(e)
    }
}

impl From<std::env::VarError> for ConfigError {
    fn from(_: std::env::VarError) -> Self {
        ConfigError::HomeDirNotFound
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::HomeDirNotFound => write!(f, "Home directory not found"),
            ConfigError::EnvVarNotFound(var) => {
                write!(f, "Environment variable not found: {}", var)
            }
            ConfigError::ConfigFileNotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::CommandError(e) => write!(f, "Command error: {}", e),
        }
    }
}

impl From<CommandError> for ConfigError {
    fn from(e: CommandError) -> Self {
        ConfigError::CommandError(e)
    }
}
