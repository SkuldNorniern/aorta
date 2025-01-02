use std::{fs, path::Path};

use super::{Config, ConfigError, ConfigPaths};

pub struct ConfigLoader<'a> {
    paths: &'a ConfigPaths,
}

impl<'a> ConfigLoader<'a> {
    pub fn new(paths: &'a ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn load_configs(&self, config: &mut Config) -> Result<(), ConfigError> {
        self.source_if_exists(&self.paths.profile_path, config)?;
        self.source_if_exists(&self.paths.rc_path, config)?;
        Ok(())
    }

    fn source_if_exists(&self, path: &Path, config: &mut Config) -> Result<(), ConfigError> {
        if path.exists() {
            let content = fs::read_to_string(path)?;
            for line in content.lines() {
                self.process_line(line, config)?;
            }
        }
        Ok(())
    }

    fn process_line(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        match line {
            s if s.starts_with("export ") => self.process_env_var(&s["export ".len()..], config),
            s if s.starts_with("PATH=") => self.process_path_var(&s["PATH=".len()..], config),
            s if s.starts_with("alias ") => self.process_alias(&s["alias ".len()..], config),
            _ => Ok(()),
        }
    }

    fn process_env_var(&self, var_def: &str, config: &mut Config) -> Result<(), ConfigError> {
        if let Some((name, value)) = var_def.split_once('=') {
            let name = name.trim();
            let mut value = value.trim();

            // Remove quotes if present
            if value.starts_with('"') && value.ends_with('"') {
                value = &value[1..value.len() - 1];
            }

            // Use EnvVarManager's expand_value
            let expanded_value = config.env_vars.expand_value(value);
            config.env_vars.set(name, &expanded_value);
        }
        Ok(())
    }

    fn process_path_var(&self, value: &str, config: &mut Config) -> Result<(), ConfigError> {
        let current_path = std::env::var("PATH").unwrap_or_default();
        let mut value = value.replace("$PATH", &current_path);

        // Remove quotes if present
        if value.starts_with('"') && value.ends_with('"') {
            value = (&value[1..value.len() - 1]).to_string();
        }

        // Use EnvVarManager's expand_value for $HOME expansion
        let expanded_path = config.env_vars.expand_value(&value);
        config.env_vars.set("PATH", &expanded_path);
        Ok(())
    }

    fn process_alias(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        if let Some((name, command)) = line.split_once('=') {
            let name = name.trim();
            let mut command = command.trim();

            // Remove surrounding quotes if present
            if (command.starts_with('\'') && command.ends_with('\''))
                || (command.starts_with('"') && command.ends_with('"'))
            {
                command = &command[1..command.len() - 1];
            }

            config.aliases.add(name, command);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_config() -> Config {
        Config::new().unwrap()
    }

    fn create_temp_config_file(content: &str) -> PathBuf {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("test_config");
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_process_env_var() {
        let paths = ConfigPaths::new().unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader
            .process_env_var("TEST_VAR=\"hello world\"", &mut config)
            .unwrap();
        assert_eq!(env::var("TEST_VAR").unwrap(), "hello world");
    }

    #[test]
    fn test_process_path_var() {
        let paths = ConfigPaths::new().unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        let old_path = env::var("PATH").unwrap_or_default();
        loader
            .process_path_var("/usr/local/bin:$PATH", &mut config)
            .unwrap();

        let new_path = env::var("PATH").unwrap();
        assert!(new_path.starts_with("/usr/local/bin:"));
        assert!(new_path.contains(&old_path));
    }

    #[test]
    fn test_process_alias() {
        let paths = ConfigPaths::new().unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.process_alias("ll='ls -la'", &mut config).unwrap();
        assert_eq!(config.get_alias("ll").unwrap(), "ls -la");
    }

    #[test]
    fn test_source_if_exists() {
        let content = r#"
            export TEST_VAR="test value"
            alias ll='ls -la'
            PATH=/usr/local/bin:$PATH
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new().unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert_eq!(env::var("TEST_VAR").unwrap(), "test value");
        assert_eq!(config.get_alias("ll").unwrap(), "ls -la");
        assert!(env::var("PATH").unwrap().contains("/usr/local/bin"));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }
}
