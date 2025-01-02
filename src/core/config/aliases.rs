use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

pub struct AliasManager {
    aliases: HashMap<Box<str>, Box<str>>,
}

impl AliasManager {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: &str, command: &str) {
        self.aliases.insert(name.into(), command.into());
    }

    pub fn get<'a>(&'a self, cmd: &str) -> Option<Cow<'a, str>> {
        self.aliases.get(cmd).map(|s| Cow::Borrowed(&**s))
    }

    pub fn expand_command<'a>(&'a self, command: &'a str) -> Cow<'a, str> {
        let mut parts: Vec<&str> = command.split_whitespace().collect();
        if let Some(first_word) = parts.first() {
            if let Some(alias_value) = self.get(first_word) {
                parts[0] = &alias_value;
                return Cow::Owned(parts.join(" "));
            }
        }
        Cow::Borrowed(command)
    }

    pub fn get_all(&self) -> BTreeMap<Cow<'_, str>, Cow<'_, str>> {
        self.aliases
            .iter()
            .map(|(k, v)| (Cow::Borrowed(&**k), Cow::Borrowed(&**v)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_alias() {
        let mut manager = AliasManager::new();
        manager.add("ll", "ls -la");
        assert_eq!(manager.get("ll").unwrap(), "ls -la");
    }

    #[test]
    fn test_expand_command() {
        let mut manager = AliasManager::new();
        manager.add("ll", "ls -la");

        let expanded = manager.expand_command("ll /home");
        assert_eq!(expanded, "ls -la /home");
    }

    #[test]
    fn test_no_expansion_needed() {
        let manager = AliasManager::new();
        let command = "ls -l";
        let expanded = manager.expand_command(command);
        assert!(matches!(expanded, Cow::Borrowed(_)));
        assert_eq!(expanded, command);
    }

    #[test]
    fn test_get_all() {
        let mut manager = AliasManager::new();
        manager.add("ll", "ls -la");
        manager.add("gs", "git status");

        let all = manager.get_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all.get("ll").unwrap(), "ls -la");
        assert_eq!(all.get("gs").unwrap(), "git status");
    }
}
