use std::borrow::Cow;
use std::collections::HashMap;
use std::env;

pub struct EnvVarManager {
    env_vars: HashMap<Box<str>, Box<str>>,
}

impl EnvVarManager {
    pub fn new() -> Self {
        let mut manager = Self {
            env_vars: HashMap::new(),
        };

        for (key, value) in env::vars() {
            manager.set(&key, &value);
        }

        manager
    }

    pub fn set(&mut self, name: &str, value: &str) {
        self.env_vars.insert(name.into(), value.into());
        env::set_var(name, value);
    }

    pub fn expand_value<'a>(&self, value: &'a str) -> Cow<'a, str> {
        let mut result = value.to_owned();
        let mut modified = false;

        if let Ok(home) = env::var("HOME") {
            if result.contains("$HOME") {
                result = result.replace("$HOME", &home);
                modified = true;
            }
        }

        if let Ok(path) = env::var("PATH") {
            if result.contains("$PATH") {
                result = result.replace("$PATH", &path);
                modified = true;
            }
        }

        if modified {
            Cow::Owned(result)
        } else {
            Cow::Borrowed(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get() {
        let mut manager = EnvVarManager::new();
        manager.set("TEST_VAR", "test value");
        assert_eq!(env::var("TEST_VAR").unwrap(), "test value");
    }

    #[test]
    fn test_expand_value() {
        let manager = EnvVarManager::new();
        env::set_var("HOME", "/home/user");
        env::set_var("PATH", "/usr/bin");

        let value = "$HOME/bin:$PATH";
        let expanded = manager.expand_value(value);
        assert_eq!(expanded, "/home/user/bin:/usr/bin");
    }

    #[test]
    fn test_no_expansion_needed() {
        let manager = EnvVarManager::new();
        let value = "simple value";
        let expanded = manager.expand_value(value);
        assert!(matches!(expanded, Cow::Borrowed(_)));
        assert_eq!(expanded, "simple value");
    }
}
