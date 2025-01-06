use super::{Command, CommandError};
use crate::core::env::{EnvError, EnvVarManager};
use std::borrow::Cow;

pub struct ExportCommand<'a> {
    env_vars: &'a mut EnvVarManager,
}

impl<'a> ExportCommand<'a> {
    pub fn new(env_vars: &'a mut EnvVarManager) -> ExportCommand<'a> {
        Self { env_vars }
    }

    fn parse_export<'b>(
        &self,
        args: &'b [String],
    ) -> Result<(Cow<'b, str>, Cow<'b, str>), CommandError> {
        if args.is_empty() {
            return Err(CommandError::InvalidArguments(
                "Export syntax: export NAME=VALUE".into(),
            ));
        }

        let arg = &args[0];
        let parts: Vec<&str> = arg.splitn(2, '=').collect();

        if parts.len() != 2 {
            return Err(CommandError::InvalidArguments(
                "Export syntax: export NAME=VALUE".into(),
            ));
        }

        let name = parts[0].trim();
        let value = parts[1].trim();

        // Remove quotes if present
        let value = if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            Cow::Owned(value[1..value.len() - 1].to_owned())
        } else {
            Cow::Borrowed(value)
        };

        if name.is_empty() {
            return Err(CommandError::InvalidArguments(
                "Variable name cannot be empty".into(),
            ));
        }

        Ok((Cow::Borrowed(name), value))
    }
}

impl Command for ExportCommand<'_> {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            return Err(CommandError::InvalidArguments(
                "Export syntax: export NAME=VALUE".into(),
            ));
        }

        let (name, value) = self.parse_export(args)?;

        // Since we can't use &mut self in execute, we need to handle interior mutability
        // This is safe because we're the only one modifying the environment at this point
        unsafe {
            let env_vars = &mut *(self.env_vars as *const _ as *mut EnvVarManager);
            env_vars.set(&name, &value).map_err(|e| match e {
                EnvError::InvalidValue(msg) => CommandError::InvalidArguments(msg.to_string()),
                _ => CommandError::InvalidArguments(e.to_string()),
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn setup_command() -> ExportCommand<'static> {
        let env_vars = EnvVarManager::new().unwrap();
        ExportCommand::new(Box::leak(Box::new(env_vars)))
    }

    #[test]
    fn test_export_simple() -> Result<(), CommandError> {
        let cmd = setup_command();
        cmd.execute(&["TEST_VAR=value".to_string()])?;
        assert_eq!(env::var("TEST_VAR").unwrap(), "value");
        Ok(())
    }

    #[test]
    fn test_export_quoted() -> Result<(), CommandError> {
        let cmd = setup_command();
        cmd.execute(&["TEST_VAR=\"quoted value\"".to_string()])?;
        assert_eq!(env::var("TEST_VAR").unwrap(), "quoted value");
        Ok(())
    }

    #[test]
    fn test_export_path() -> Result<(), CommandError> {
        let cmd = setup_command();
        env::set_var("PATH", "/usr/bin");
        cmd.execute(&["PATH=/usr/local/bin:$PATH".to_string()])?;
        assert!(env::var("PATH").unwrap().starts_with("/usr/local/bin:"));
        Ok(())
    }

    #[test]
    fn test_export_empty_args() {
        let cmd = setup_command();
        assert!(cmd.execute(&[]).is_err());
    }

    #[test]
    fn test_export_invalid_format() {
        let cmd = setup_command();
        assert!(cmd.execute(&["INVALID".to_string()]).is_err());
    }

    #[test]
    fn test_export_empty_name() {
        let cmd = setup_command();
        assert!(cmd.execute(&["=value".to_string()]).is_err());
    }

    #[test]
    fn test_export_with_spaces() -> Result<(), CommandError> {
        let cmd = setup_command();
        cmd.execute(&["TEST_VAR=value with spaces".to_string()])?;
        assert_eq!(env::var("TEST_VAR").unwrap(), "value with spaces");
        Ok(())
    }
}
