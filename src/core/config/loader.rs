use std::{fs, path::Path, path::PathBuf};

use crate::flags::Flags;

use super::program::CompatMode;
use super::{Config, ConfigError, ConfigPaths};

pub struct ConfigLoader<'a> {
    paths: &'a ConfigPaths,
    compat_mode: CompatMode,
}

impl<'a> ConfigLoader<'a> {
    pub fn new(paths: &'a ConfigPaths) -> Self {
        Self {
            paths,
            compat_mode: CompatMode::Native,
        }
    }

    pub fn new_with_mode(paths: &'a ConfigPaths, compat_mode: CompatMode) -> Self {
        Self { paths, compat_mode }
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

        if let Some(env_path) = std::env::var_os("ENV").or_else(|| std::env::var_os("AORTA_ENV")) {
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
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "aorta"))
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
            self.process_content(&content, config)?;
        }
        Ok(())
    }

    fn process_content(&self, content: &str, config: &mut Config) -> Result<(), ConfigError> {
        let lines: Vec<&str> = content.lines().collect();
        let mut index = 0;

        while index < lines.len() {
            let line = lines[index].trim();
            if line.starts_with("case ") {
                index = if self.compat_mode == CompatMode::Compat {
                    self.process_case_block(&lines, index, config)?
                } else {
                    Self::skip_case_block(&lines, index)
                };
                continue;
            }

            self.process_line(lines[index], config)?;
            index += 1;
        }

        Ok(())
    }

    fn skip_case_block(lines: &[&str], start_index: usize) -> usize {
        let mut index = start_index + 1;
        while index < lines.len() {
            if lines[index].trim() == "esac" {
                return index + 1;
            }
            index += 1;
        }
        index
    }

    fn process_case_block(
        &self,
        lines: &[&str],
        start_index: usize,
        config: &mut Config,
    ) -> Result<usize, ConfigError> {
        let header = lines[start_index].trim();
        let Some(subject_expr) = Self::extract_case_subject(header) else {
            self.process_line(lines[start_index], config)?;
            return Ok(start_index + 1);
        };

        let expanded_subject = config.env_vars.expand_value(subject_expr);
        let expanded_subject = Self::expand_shell_vars(expanded_subject.trim());
        let subject = Self::strip_matching_quotes(expanded_subject.trim()).to_string();

        let mut clauses: Vec<(Vec<String>, Vec<String>)> = Vec::new();
        let mut current_patterns: Option<Vec<String>> = None;
        let mut current_commands: Vec<String> = Vec::new();
        let mut index = start_index + 1;
        let mut found_esac = false;

        while index < lines.len() {
            let raw = lines[index];
            let trimmed = raw.trim();

            if trimmed == "esac" {
                if let Some(patterns) = current_patterns.take() {
                    clauses.push((patterns, std::mem::take(&mut current_commands)));
                }
                found_esac = true;
                break;
            }

            if let Some(patterns) = Self::extract_case_patterns(trimmed) {
                if let Some(existing) = current_patterns.take() {
                    clauses.push((existing, std::mem::take(&mut current_commands)));
                }
                current_patterns = Some(patterns);
                index += 1;
                continue;
            }

            if trimmed == ";;" {
                if let Some(patterns) = current_patterns.take() {
                    clauses.push((patterns, std::mem::take(&mut current_commands)));
                }
                index += 1;
                continue;
            }

            if let Some(ref _patterns) = current_patterns {
                if let Some(before_terminator) = trimmed.strip_suffix(";;") {
                    let cmd = before_terminator.trim();
                    if !cmd.is_empty() {
                        current_commands.push(cmd.to_string());
                    }
                    if let Some(patterns) = current_patterns.take() {
                        clauses.push((patterns, std::mem::take(&mut current_commands)));
                    }
                } else {
                    current_commands.push(trimmed.to_string());
                }
            }

            index += 1;
        }

        if !found_esac {
            return Ok(index);
        }

        for (patterns, commands) in clauses {
            if self.case_clause_matches(&patterns, &subject, config) {
                for cmd in commands {
                    self.process_line(&cmd, config)?;
                }
                break;
            }
        }

        Ok(index + 1)
    }

    fn extract_case_subject(header: &str) -> Option<&str> {
        let body = header.strip_prefix("case ")?.trim();
        let in_index = body.rfind(" in")?;
        Some(body[..in_index].trim())
    }

    fn extract_case_patterns(line: &str) -> Option<Vec<String>> {
        let line = line.trim();
        if !line.ends_with(')') {
            return None;
        }

        let patterns = line
            .trim_end_matches(')')
            .split('|')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(String::from)
            .collect::<Vec<_>>();

        if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        }
    }

    fn case_clause_matches(&self, patterns: &[String], subject: &str, config: &Config) -> bool {
        patterns.iter().any(|pattern| {
            let expanded = config.env_vars.expand_value(pattern.trim());
            let expanded = Self::expand_shell_vars(expanded.trim());
            let pattern = Self::strip_matching_quotes(expanded.trim());
            Self::wildcard_match(pattern, subject)
        })
    }

    fn expand_shell_vars(value: &str) -> String {
        let mut out = String::with_capacity(value.len());
        let chars: Vec<char> = value.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] != '$' {
                out.push(chars[i]);
                i += 1;
                continue;
            }

            if i + 1 >= chars.len() {
                out.push('$');
                break;
            }

            if chars[i + 1] == '{' {
                let mut j = i + 2;
                while j < chars.len() && chars[j] != '}' {
                    j += 1;
                }

                if j >= chars.len() {
                    out.push('$');
                    i += 1;
                    continue;
                }

                let raw_name: String = chars[i + 2..j].iter().collect();
                let name = raw_name.strip_suffix('-').unwrap_or(&raw_name);
                out.push_str(&std::env::var(name).unwrap_or_default());
                i = j + 1;
                continue;
            }

            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }

            if j == i + 1 {
                out.push('$');
                i += 1;
                continue;
            }

            let name: String = chars[i + 1..j].iter().collect();
            out.push_str(&std::env::var(name).unwrap_or_default());
            i = j;
        }

        out
    }

    fn strip_matching_quotes(s: &str) -> &str {
        if s.len() >= 2 {
            let bytes = s.as_bytes();
            let first = bytes[0];
            let last = bytes[s.len() - 1];
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                return &s[1..s.len() - 1];
            }
        }
        s
    }

    fn wildcard_match(pattern: &str, input: &str) -> bool {
        let p = pattern.as_bytes();
        let s = input.as_bytes();

        let (mut pi, mut si) = (0usize, 0usize);
        let mut star_idx: Option<usize> = None;
        let mut match_idx = 0usize;

        while si < s.len() {
            if pi < p.len() && (p[pi] == b'?' || p[pi] == s[si]) {
                pi += 1;
                si += 1;
            } else if pi < p.len() && p[pi] == b'*' {
                star_idx = Some(pi);
                pi += 1;
                match_idx = si;
            } else if let Some(star) = star_idx {
                pi = star + 1;
                match_idx += 1;
                si = match_idx;
            } else {
                return false;
            }
        }

        while pi < p.len() && p[pi] == b'*' {
            pi += 1;
        }

        pi == p.len()
    }

    fn process_line(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return Ok(());
        }

        if self.compat_mode == CompatMode::Compat && line.starts_with('[') && line.contains("&&") {
            return self.process_short_circuit(line, config);
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

    fn process_short_circuit(&self, line: &str, config: &mut Config) -> Result<(), ConfigError> {
        let Some((condition, rhs)) = line.split_once("&&") else {
            return config.execute_command(line);
        };

        let condition = condition.trim();
        let rhs = rhs.trim();

        if !condition.starts_with('[') {
            return config.execute_command(line);
        }

        if self.evaluate_condition(condition, config)? {
            if let Some(rest) = rhs.strip_prefix("\\.") {
                let normalized = format!(".{}", rest);
                self.process_line(&normalized, config)?;
            } else {
                self.process_line(rhs, config)?;
            }
        }

        Ok(())
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
        let theme_path = self
            .paths
            .aorta_home
            .join("themes")
            .join(format!("{}.aorta", name));
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
            s if s.starts_with("[ -s ") => {
                let path = self.extract_path(s, "[ -s ", config)?;
                Ok(path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false))
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
        let mut path = line
            .trim_start_matches(". ")
            .trim_start_matches("source ")
            .trim();

        if (path.starts_with('"') && path.ends_with('"'))
            || (path.starts_with('\'') && path.ends_with('\''))
        {
            path = &path[1..path.len() - 1];
        }

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
        fs::write(
            temp.join("custom").join("mine.aorta"),
            "alias mc=echo custom",
        )
        .unwrap();
        env::set_var("HOME", env::temp_dir());
        env::set_var("AORTA_HOME", &temp);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();
        loader
            .load_configs(&mut config, &crate::flags::Flags::new())
            .unwrap();
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
    fn test_short_circuit_source_with_file_size_check() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = env::temp_dir();
        let sourced_file = temp_dir.join(format!("aorta_source_{}.aorta", nonce));
        fs::write(&sourced_file, "export SHORT_CIRCUIT=yes\n").unwrap();
        let content = format!(
            "[ -s \"{}\" ] && \\. \"{}\"\n",
            sourced_file.display(),
            sourced_file.display()
        );
        let config_file = temp_dir.join(format!("aorta_config_{}.aorta", nonce));
        fs::write(&config_file, content).unwrap();

        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new_with_mode(&paths, CompatMode::Compat);
        let mut config = setup_test_config();

        loader.source_if_exists(&config_file, &mut config).unwrap();

        assert_eq!(env::var("SHORT_CIRCUIT").unwrap(), "yes");
        fs::remove_file(sourced_file).unwrap();
        fs::remove_file(config_file).unwrap();
    }

    #[test]
    fn test_case_block_executes_matching_branch() {
        let content = r#"
            export PICK="foo"
            case "$PICK" in
                foo)
                    export CASE_RESULT="matched"
                    ;;
                *)
                    export CASE_RESULT="fallback"
                    ;;
            esac
        "#;

        let file_path = create_temp_config_file(content);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new_with_mode(&paths, CompatMode::Compat);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        assert_eq!(env::var("CASE_RESULT").unwrap(), "matched");
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_case_block_fallback_branch_updates_path() {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let content = r#"
            export TEST_CASE_PATH="/usr/bin"
            case ":$TEST_CASE_PATH:" in
                *:"$HOME/.local/bin":*)
                    ;;
                *)
                    export TEST_CASE_PATH="$HOME/.local/bin:$TEST_CASE_PATH"
                    ;;
            esac
        "#;

        let file_path = create_temp_config_file(content);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new_with_mode(&paths, CompatMode::Compat);
        let mut config = setup_test_config();

        loader.source_if_exists(&file_path, &mut config).unwrap();

        let expected_prefix = format!("{}/.local/bin:", home);
        assert!(env::var("TEST_CASE_PATH")
            .unwrap()
            .starts_with(&expected_prefix));
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_native_mode_skips_case_compat_parsing() {
        env::remove_var("CASE_NATIVE_ONLY");
        let content = r#"
            export PICK="foo"
            case "$PICK" in
                foo)
                    export CASE_NATIVE_ONLY="matched"
                    ;;
            esac
        "#;

        let file_path = create_temp_config_file(content);
        let paths = ConfigPaths::new(None).unwrap();
        let loader = ConfigLoader::new(&paths);
        let mut config = setup_test_config();

        let _ = loader.source_if_exists(&file_path, &mut config);

        assert!(env::var("CASE_NATIVE_ONLY").is_err());
        env::remove_var("CASE_NATIVE_ONLY");
        fs::remove_file(file_path).unwrap();
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
