use std::{fs, path::Path, path::PathBuf};

use crate::flags::Flags;

use super::{Config, ConfigError, ConfigPaths};

pub struct ConfigLoader<'a> {
    paths: &'a ConfigPaths,
}

impl<'a> ConfigLoader<'a> {
    pub fn new(paths: &'a ConfigPaths) -> Self {
        Self { paths }
    }

    pub fn load_configs(&self, config: &mut Config, flags: &Flags) -> Result<(), ConfigError> {
        if !flags.is_set("noprofile") {
            self.source_if_exists(&self.paths.system_profile, config)?;
            self.source_if_exists(&self.paths.profile_path, config)?;
        }

        if !flags.is_set("norc") {
            self.source_if_exists(&self.paths.system_rc, config)?;
            self.source_if_exists(&self.paths.rc_path, config)?;
            self.source_custom_dir(config)?;
        }

        if let Some(env_path) = std::env::var_os("ENV")
            .or_else(|| std::env::var_os("AORTA_ENV"))
        {
            let p = Path::new(&env_path);
            if p.is_absolute() && p.exists() {
                self.source_if_exists(p, config)?;
            } else if let Some(home) = crate::path::home_dir() {
                let full = home.join(env_path);
                if full.exists() {
                    self.source_if_exists(&full, config)?;
                }
            }
        }

        Ok(())
    }

    fn source_custom_dir(&self, config: &mut Config) -> Result<(), ConfigError> {
        let custom_dir = self.paths.aorta_home.join("custom");
        if !custom_dir.is_dir() {
            return Ok(());
        }
        let mut entries: Vec<PathBuf> = fs::read_dir(&custom_dir)
            .map_err(ConfigError::from)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().is_some_and(|ext| ext == "aorta")
            })
            .map(|e| e.path())
            .collect();
        entries.sort();
        for path in entries {
            self.source_if_exists(&path, config)?;
        }
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
            "then" | "else" | "fi" => Ok(()),
            s if s.starts_with("export ") => self.process_env_var(&s["export ".len()..], config),
            s if s.starts_with("PATH=") => self.process_path_var(&s["PATH=".len()..], config),
            s if s.starts_with("alias ") => self.process_alias(&s["alias ".len()..], config),
            s if s.starts_with("theme ") => self.process_theme(&s["theme ".len()..], config),
            s if s.starts_with("plugins ") => self.process_plugins(&s["plugins ".len()..], config),
            s if s.starts_with("if ") => self.process_conditional(s, config),
            s if s.starts_with(". ") || s.starts_with("source ") => self.process_source(s, config),
            _ => config.execute_command(line),
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
        let current_path =
            std::env::var("PATH").map_err(|_| ConfigError::EnvVarNotFound("PATH".to_string()))?;

        let mut value = value.trim();

        // Remove any surrounding quotes
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = &value[1..value.len() - 1];
        }

        // Handle $PATH replacement without adding quotes
        let new_path = if value.contains("$PATH") {
            value.replace("$PATH", &current_path)
        } else {
            // If no $PATH variable, append to current path
            format!("{}:{}", value, current_path)
        };

        // Let EnvVarManager handle the sanitization
        config.env_vars.set("PATH", &new_path);
        Ok(())
    }

    fn process_theme(&self, name: &str, config: &mut Config) -> Result<(), ConfigError> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(());
        }
        let theme_path = self.paths.aorta_home.join("themes").join(format!("{}.aorta", name));
        self.source_if_exists(&theme_path, config)
    }

    fn process_plugins(&self, names: &str, config: &mut Config) -> Result<(), ConfigError> {
        for name in names.split_whitespace() {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            let plugin_path = self
                .paths
                .aorta_home
                .join("plugins")
                .join(name)
                .join(format!("{}.aorta", name));
            self.source_if_exists(&plugin_path, config)?;
        }
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

    fn evaluate_condition(&self, condition: &str, config: &Config) -> Result<bool, ConfigError> {
        match condition {
            s if s.starts_with("[ -n ") => {
                let var_name = self.extract_var_name(s, "[ -n ");
                Ok(std::env::var(var_name).is_ok())
            }
            s if s.starts_with("[ -z ") => {
                let var_name = self.extract_var_name(s, "[ -z ");
                Ok(std::env::var(var_name).is_err())
            }
            s if s.starts_with("[ -f ") => {
                let path = self.extract_path(s, "[ -f ", config)?;
                Ok(path.is_file())
            }
            s if s.starts_with("[ -d ") => {
                let path = self.extract_path(s, "[ -d ", config)?;
                Ok(path.is_dir())
            }
            s if s.contains("=") => Ok(self.check_equality(s, config)),
            _ => Ok(false),
        }
    }

    fn extract_var_name(&self, s: &str, prefix: &str) -> String {
        s.trim_start_matches(prefix)
            .trim_end_matches(" ]")
            .trim_matches('"')
            .trim_matches('$')
            .to_string()
    }

    fn extract_path(&self, s: &str, prefix: &str, config: &Config) -> Result<PathBuf, ConfigError> {
        let path = s
            .trim_start_matches(prefix)
            .trim_end_matches(" ]")
            .trim_matches('"');
        let expanded_path = config.env_vars.expand_value(path);
        Ok(PathBuf::from(expanded_path.as_ref()))
    }

    fn check_equality(&self, s: &str, config: &Config) -> bool {
        let parts: Vec<&str> = s
            .trim_start_matches("[ ")
            .trim_end_matches(" ]")
            .split('=')
            .map(|s| s.trim_matches('"').trim())
            .collect();

        if parts.len() == 2 {
            let left = config.env_vars.expand_value(parts[0]);
            let right = config.env_vars.expand_value(parts[1]);
            left == right
        } else {
            false
        }
    }

    fn process_conditional(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        let condition = line.trim_start_matches("if ").trim();
        let condition_met = self.evaluate_condition(condition, config)?;
        self.process_conditional_block(line, condition_met, config)
    }

    fn process_conditional_block(
        &self,
        line: &str,
        condition_met: bool,
        config: &mut Config,
    ) -> Result<(), ConfigError> {
        let mut in_then_block = false;
        let mut skip_until_fi = !condition_met;

        let content = fs::read_to_string(&config.paths.rc_path)?;
        let mut lines = content.lines().skip_while(|l| l.trim() != line);
        let _ = lines.next(); // Skip the 'if' line

        for current_line in lines {
            let current_line = current_line.trim();
            match current_line {
                "then" => in_then_block = true,
                "else" => skip_until_fi = !skip_until_fi,
                "fi" => break,
                _ if in_then_block && !skip_until_fi => {
                    self.process_line(current_line, config)?;
                }
                _ => continue,
            }
        }

        Ok(())
    }

    fn process_source(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        let path = line
            .trim_start_matches(". ")
            .trim_start_matches("source ")
            .trim();

        // Expand environment variables in the path
        let expanded_path = config.env_vars.expand_value(path);
        let path = Path::new(expanded_path.as_ref());

        if path.exists() {
            self.source_if_exists(path, config)?;
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
        Config::new(None).unwrap()
    }

    fn create_temp_config_file(content: &str) -> PathBuf {
        let temp_dir = env::temp_dir();
        let file_path = temp_dir.join("test_config");
        fs::write(&file_path, content).unwrap();
        file_path
    }

    #[test]
    fn test_process_env_var() {
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader
            .process_env_var("TEST_VAR=\"hello world\"", &mut config)
            .unwrap();
        assert_eq!(env::var("TEST_VAR").unwrap(), "hello world");
    }

    #[test]
    fn test_process_path_var() {
        let paths = ConfigPaths::new(None).unwrap();
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
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.process_alias("ll='ls -la'", &mut config).unwrap();
        assert_eq!(config.get_alias("ll").unwrap(), "ls -la");
    }

    #[test]
    fn test_process_theme() {
        let temp = env::temp_dir().join("aorta_theme_test");
        let _ = fs::create_dir_all(temp.join("themes"));
        fs::write(
            temp.join("themes").join("test.aorta"),
            "export AORTA_PROMPT=\"test > \"",
        )
        .unwrap();
        env::set_var("HOME", env::temp_dir());
        env::set_var("AORTA_HOME", &temp);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();
        loader.process_theme("test", &mut config).unwrap();
        assert_eq!(env::var("AORTA_PROMPT").unwrap(), "test > ");
        env::remove_var("AORTA_HOME");
    }

    #[test]
    fn test_source_custom_dir() {
        let temp = env::temp_dir().join("aorta_custom_test");
        let _ = fs::create_dir_all(temp.join("custom"));
        fs::write(temp.join("custom").join("mine.aorta"), "alias mc=echo custom")
            .unwrap();
        env::set_var("HOME", env::temp_dir());
        env::set_var("AORTA_HOME", &temp);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();
        loader.load_configs(&mut config, &crate::flags::Flags::new()).unwrap();
        assert_eq!(config.get_alias("mc").unwrap(), "echo custom");
        env::remove_var("AORTA_HOME");
    }

    #[test]
    fn test_noprofile_skips_profile() {
        let profile_content = "export NOPROFILE_TEST=loaded";
        let temp = env::temp_dir().join("aorta_noprofile_test");
        let _ = std::fs::create_dir_all(&temp);
        std::fs::write(temp.join("profile"), profile_content).unwrap();
        env::set_var("HOME", &temp);
        let mut flags = crate::flags::Flags::new();
        flags.parse(&["--noprofile".to_string()]).unwrap();

        let paths = ConfigPaths::new(None).unwrap();
        let mut config_paths = paths;
        config_paths.profile_path = temp.join("profile");

        let loader = ConfigLoader::new(&config_paths);
        let mut config = setup_test_config();
        loader.load_configs(&mut config, &flags).unwrap();

        assert!(env::var("NOPROFILE_TEST").is_err());
    }

    #[test]
    fn test_process_plugins() {
        let temp = env::temp_dir().join("aorta_plugins_test");
        let _ = fs::create_dir_all(temp.join("plugins").join("sample"));
        fs::write(
            temp.join("plugins").join("sample").join("sample.aorta"),
            "alias sp=echo sample",
        )
        .unwrap();
        env::set_var("HOME", env::temp_dir());
        env::set_var("AORTA_HOME", &temp);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();
        loader.process_plugins("sample", &mut config).unwrap();
        assert_eq!(config.get_alias("sp").unwrap(), "echo sample");
        env::remove_var("AORTA_HOME");
    }

    #[test]
    fn test_source_if_exists() {
        let content = r#"
            export TEST_VAR="test value"
            alias ll='ls -la'
            PATH=/usr/local/bin:$PATH
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert_eq!(env::var("TEST_VAR").unwrap(), "test value");
        assert_eq!(config.get_alias("ll").unwrap(), "ls -la");
        assert!(env::var("PATH").unwrap().contains("/usr/local/bin"));

        // Cleanup
        let _ = fs::remove_file(file_path);
    }

    #[test]
    fn test_conditional_blocks() {
        let content = r#"
            # This should be skipped
            if [ -n "$BASH_VERSION" ]; then
                export TEST_VAR="bash"
            fi
            
            # This should be processed
            export AFTER_IF="processed"
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert!(env::var("TEST_VAR").is_err()); // Should be skipped
        assert_eq!(env::var("AFTER_IF").unwrap(), "processed");

        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_conditional_with_env_var() {
        let content = r#"
            export TEST_VAR="hello"
            if [ -n "$TEST_VAR" ]
            then
                export CONDITION_MET="yes"
            fi
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert_eq!(env::var("CONDITION_MET").unwrap(), "yes");
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_conditional_with_file_check() {
        let test_file = create_temp_config_file("test content");
        let content = format!(
            r#"
            if [ -f "{}" ]
            then
                export FILE_EXISTS="yes"
            fi
        "#,
            test_file.display()
        );

        let config_file = create_temp_config_file(&content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&config_file, &mut config).unwrap();

        assert_eq!(env::var("FILE_EXISTS").unwrap(), "yes");
        fs::remove_file(test_file).unwrap();
        fs::remove_file(config_file).unwrap();
    }

    #[test]
    fn test_conditional_equality() {
        let content = r#"
            export TEST_VAR="value"
            if [ "$TEST_VAR" = "value" ]
            then
                export EQUAL="yes"
            fi
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert_eq!(env::var("EQUAL").unwrap(), "yes");
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_conditional_with_else() {
        let content = r#"
            if [ -n "$NONEXISTENT_VAR" ]
            then
                export THEN_BLOCK="executed"
            else
                export ELSE_BLOCK="executed"
            fi
        "#;
        let file_path = create_temp_config_file(content);

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert!(env::var("THEN_BLOCK").is_err());
        assert_eq!(env::var("ELSE_BLOCK").unwrap(), "executed");

        fs::remove_file(file_path).unwrap();
    }
}
