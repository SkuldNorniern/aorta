use std::{borrow::Cow, collections::BTreeMap, fmt};

mod aliases;
mod env_vars;
mod loader;
mod paths;

use aliases::AliasManager;
use env_vars::EnvVarManager;
use loader::ConfigLoader;
use paths::ConfigPaths;

pub struct Config {
    paths: ConfigPaths,
    aliases: AliasManager,
    env_vars: EnvVarManager,
}

impl Config {
    pub fn new() -> Result<Self, ConfigError> {
        let paths = ConfigPaths::new()?;
        let aliases = AliasManager::new();
        let env_vars = EnvVarManager::new();

        Ok(Config {
            paths,
            aliases,
            env_vars,
        })
    }

    pub fn load(&mut self) -> Result<(), ConfigError> {
        let paths = self.paths.clone();
        let loader: ConfigLoader<'_> = ConfigLoader::new(&paths);
        loader.load_configs(self)?;
        Ok(())
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
            ConfigError::EnvVarNotFound(var) => write!(f, "Environment variable not found: {}", var),
            ConfigError::ConfigFileNotFound(path) => write!(f, "Config file not found: {}", path),
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}
