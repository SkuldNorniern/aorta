use super::{Command, CommandError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AliasCommand {
    aliases: Arc<Mutex<HashMap<String, String>>>,
}

impl AliasCommand {
    pub fn new(aliases: Arc<Mutex<HashMap<String, String>>>) -> Self {
        Self { aliases }
    }
}

impl Command for AliasCommand {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            // List all aliases
            let aliases = self.aliases.lock().map_err(|e| {
                CommandError::ExecutionError(format!("Failed to access aliases: {}", e))
            })?;

            for (alias, command) in aliases.iter() {
                println!("{}='{}'", alias, command);
            }
            return Ok(());
        }

        let alias_str = args.join(" ");
        if let Some((name, value)) = alias_str.split_once('=') {
            let name = name.trim().to_string();
            let value = value
                .trim()
                .trim_matches(|c| c == '\'' || c == '"')
                .to_string();

            let mut aliases = self.aliases.lock().map_err(|e| {
                CommandError::ExecutionError(format!("Failed to access aliases: {}", e))
            })?;

            aliases.insert(name, value);
        } else {
            return Err(CommandError::InvalidArguments(
                "Usage: alias name='command'".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alias_add() {
        let aliases = Arc::new(Mutex::new(HashMap::new()));
        let cmd = AliasCommand::new(aliases);

        assert!(cmd.execute(&["ll='ls -l'".to_string()]).is_ok());
    }

    #[test]
    fn test_alias_list() {
        let aliases = Arc::new(Mutex::new(HashMap::new()));
        let cmd = AliasCommand::new(aliases);

        assert!(cmd.execute(&[]).is_ok());
    }

    #[test]
    fn test_alias_invalid() {
        let aliases = Arc::new(Mutex::new(HashMap::new()));
        let cmd = AliasCommand::new(aliases);

        assert!(cmd.execute(&["invalid_format".to_string()]).is_err());
    }
}
