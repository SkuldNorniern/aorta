use super::EnvError;
use std::borrow::Cow;
use std::collections::HashMap;
use std::env;

#[derive(Clone, Debug)]
pub struct EnvVarManager {
    vars: HashMap<Box<str>, Box<str>>,
}

impl EnvVarManager {
    pub fn new() -> Result<Self, EnvError> {
        let mut manager = Self {
            vars: HashMap::new(),
        };

        for (key, value) in env::vars() {
            manager.set(&key, &value)?;
        }

        Ok(manager)
    }

    pub fn set(&mut self, name: &str, value: &str) -> Result<(), EnvError> {
        if name.is_empty() {
            return Err(EnvError::InvalidValue("Empty variable name"));
        }

        let clean_value = if name == "PATH" {
            self.sanitize_path(value)?
        } else {
            value.to_string()
        };

        self.vars.insert(name.into(), clean_value.clone().into());
        env::set_var(name, clean_value);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&str, EnvError> {
        self.vars
            .get(name)
            .map(|s| s.as_ref())
            .ok_or_else(move || EnvError::VarNotFound(name.to_string()))
    }

    fn sanitize_path(&self, path: &str) -> Result<String, EnvError> {
        if path.is_empty() {
            return Err(EnvError::InvalidValue("Empty PATH value"));
        }

        let parts: Vec<&str> = path
            .split([':', '"', '\''])
            .filter(|s| !s.is_empty())
            .collect();

        let mut seen = std::collections::HashSet::new();
        let unique_parts: Vec<&str> = parts
            .into_iter()
            .filter(|part| seen.insert(*part))
            .collect();

        Ok(unique_parts.join(":"))
    }

    pub fn expand_value<'a>(&self, value: &'a str) -> Result<Cow<'a, str>, EnvError> {
        if value.is_empty() {
            return Ok(Cow::Borrowed(value));
        }

        let mut modified = false;
        let mut result = value.to_string();

        if value.contains("$HOME") {
            let home = env::var("HOME")?;
            result = result.replace("$HOME", &home);
            modified = true;
        }

        if value.contains("$PATH") {
            let path = env::var("PATH")?;
            result = result.replace("$PATH", &path);
            modified = true;
        }

        Ok(if modified {
            Cow::Owned(result)
        } else {
            Cow::Borrowed(value)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_env() -> EnvVarManager {
        env::set_var("HOME", "/home/test");
        env::set_var("PATH", "/usr/bin");
        EnvVarManager::new().unwrap()
    }

    #[test]
    fn test_set_and_get() -> Result<(), EnvError> {
        let mut manager = setup_test_env();
        manager.set("TEST_VAR", "test value")?;
        assert_eq!(manager.get("TEST_VAR")?, "test value");
        Ok(())
    }

    #[test]
    fn test_expand_value() -> Result<(), EnvError> {
        let manager = setup_test_env();
        let value = "$HOME/bin:$PATH";
        let expanded = manager.expand_value(value)?;
        assert_eq!(expanded, "/home/test/bin:/usr/bin");
        Ok(())
    }

    #[test]
    fn test_sanitize_path() -> Result<(), EnvError> {
        let manager = setup_test_env();
        let path = "/usr/bin:/usr/local/bin:/usr/bin";
        let sanitized = manager.sanitize_path(path)?;
        assert_eq!(sanitized, "/usr/bin:/usr/local/bin");
        Ok(())
    }

    #[test]
    fn test_invalid_var_name() {
        let mut manager = setup_test_env();
        assert!(manager.set("", "value").is_err());
    }
}
