use std::process::Command as ProcessCommand;

use super::{Command, CommandError, CommandExecutor};

#[derive(Clone)]
pub struct EvalCommand {
    executor: CommandExecutor,
}

impl EvalCommand {
    pub fn new(executor: CommandExecutor) -> Self {
        Self { executor }
    }

    fn strip_matching_quotes<'a>(s: &'a str) -> &'a str {
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

    fn parse_command_substitution<'a>(expr: &'a str) -> Option<&'a str> {
        if !expr.starts_with("$(") || !expr.ends_with(')') {
            return None;
        }

        let inner = &expr[2..expr.len() - 1];
        if inner.trim().is_empty() {
            return None;
        }

        Some(inner)
    }

    fn run_substitution(inner: &str) -> Result<String, CommandError> {
        let output = ProcessCommand::new("sh")
            .args(["-c", inner])
            .output()
            .map_err(|e| {
                CommandError::ExecutionError(format!("eval: failed to run substitution: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CommandError::ExecutionError(format!(
                "eval: substitution failed: {}",
                stderr.trim()
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn split_statements(script: &str) -> Vec<String> {
        let mut statements = Vec::new();
        let mut current = String::new();
        let mut in_single = false;
        let mut in_double = false;
        let mut escaped = false;

        for ch in script.chars() {
            if escaped {
                current.push(ch);
                escaped = false;
                continue;
            }

            if ch == '\\' {
                current.push(ch);
                escaped = true;
                continue;
            }

            match ch {
                '\'' if !in_double => {
                    in_single = !in_single;
                    current.push(ch);
                }
                '"' if !in_single => {
                    in_double = !in_double;
                    current.push(ch);
                }
                ';' | '\n' if !in_single && !in_double => {
                    let statement = current.trim();
                    if !statement.is_empty() {
                        statements.push(statement.to_string());
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        let statement = current.trim();
        if !statement.is_empty() {
            statements.push(statement.to_string());
        }

        statements
    }

    fn execute_statement(&self, statement: &str) -> Result<(), CommandError> {
        if let Some((condition, rhs)) = statement.split_once("||") {
            let condition = condition.trim();
            if condition.starts_with('[') {
                let passed = Self::evaluate_test_condition(condition);
                if !passed {
                    return self.execute_statement(rhs.trim());
                }
                return Ok(());
            }
        }

        if let Some((condition, rhs)) = statement.split_once("&&") {
            let condition = condition.trim();
            if condition.starts_with('[') {
                let passed = Self::evaluate_test_condition(condition);
                if passed {
                    return self.execute_statement(rhs.trim());
                }
                return Ok(());
            }
        }

        if let Some(rest) = statement.strip_prefix("export ") {
            let export_args = rest.trim();
            if export_args.is_empty() {
                return Ok(());
            }

            if export_args.contains('=') {
                return self.executor.execute("export", &[export_args.to_string()]);
            }

            for name in export_args.split_whitespace() {
                if !name.is_empty() {
                    let value = std::env::var(name).unwrap_or_default();
                    std::env::set_var(name, value);
                }
            }
            return Ok(());
        }

        if let Some((name, value)) = statement.split_once('=') {
            let name = name.trim();
            if !name.is_empty() && !name.contains(char::is_whitespace) {
                let value = value.trim();
                let value = Self::strip_matching_quotes(value);
                std::env::set_var(name, value);
                return Ok(());
            }
        }

        let parts: Vec<String> = statement.split_whitespace().map(String::from).collect();
        if parts.is_empty() {
            return Ok(());
        }

        let command = &parts[0];
        let args = &parts[1..];
        if command == "eval" {
            return self.execute(args);
        }
        self.executor.execute(command, args)
    }

    fn evaluate_test_condition(condition: &str) -> bool {
        let trimmed = condition.trim();

        if let Some(value) = trimmed
            .strip_prefix("[ -z ")
            .and_then(|s| s.strip_suffix(" ]"))
        {
            return Self::resolve_test_value(value).is_empty();
        }

        if let Some(value) = trimmed
            .strip_prefix("[ -n ")
            .and_then(|s| s.strip_suffix(" ]"))
        {
            return !Self::resolve_test_value(value).is_empty();
        }

        if let Some(path) = trimmed
            .strip_prefix("[ -f ")
            .and_then(|s| s.strip_suffix(" ]"))
        {
            let path = Self::strip_matching_quotes(path.trim());
            return std::path::Path::new(path).is_file();
        }

        if let Some(path) = trimmed
            .strip_prefix("[ -d ")
            .and_then(|s| s.strip_suffix(" ]"))
        {
            let path = Self::strip_matching_quotes(path.trim());
            return std::path::Path::new(path).is_dir();
        }

        if let Some(path) = trimmed
            .strip_prefix("[ -s ")
            .and_then(|s| s.strip_suffix(" ]"))
        {
            let path = Self::strip_matching_quotes(path.trim());
            let path = std::path::Path::new(path);
            return path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false);
        }

        false
    }

    fn resolve_test_value(value: &str) -> String {
        let value = Self::strip_matching_quotes(value.trim());

        if value.starts_with("${") && value.ends_with('}') {
            let inner = &value[2..value.len() - 1];
            if let Some(name) = inner.strip_suffix('-') {
                return std::env::var(name).unwrap_or_default();
            }
            return std::env::var(inner).unwrap_or_default();
        }

        if let Some(name) = value.strip_prefix('$') {
            return std::env::var(name).unwrap_or_default();
        }

        value.to_string()
    }
}

impl Command for EvalCommand {
    fn execute(&self, args: &[String]) -> Result<(), CommandError> {
        if args.is_empty() {
            return Err(CommandError::InvalidArguments(
                "Eval syntax: eval <command>".to_string(),
            ));
        }

        let expression = args.join(" ");
        let expression = Self::strip_matching_quotes(expression.trim());
        if expression.is_empty() {
            return Ok(());
        }

        let script = if let Some(inner) = Self::parse_command_substitution(expression) {
            Self::run_substitution(inner)?
        } else {
            expression.to_string()
        };

        for statement in Self::split_statements(&script) {
            self.execute_statement(&statement)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_executor() -> CommandExecutor {
        CommandExecutor::new(&crate::flags::Flags::default()).unwrap()
    }

    #[test]
    fn test_eval_direct_export() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        cmd.execute(&["export EVAL_DIRECT=ok".to_string()])?;
        assert_eq!(std::env::var("EVAL_DIRECT").unwrap(), "ok");
        Ok(())
    }

    #[test]
    fn test_eval_command_substitution() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        cmd.execute(&["$(printf 'export EVAL_SUB=works\\n')".to_string()])?;
        assert_eq!(std::env::var("EVAL_SUB").unwrap(), "works");
        Ok(())
    }

    #[test]
    fn test_eval_multi_arg_substitution_shape() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        cmd.execute(&[
            "\"$(printf".to_string(),
            "'export EVAL_MULTI=ok')\"".to_string(),
        ])?;
        assert_eq!(std::env::var("EVAL_MULTI").unwrap(), "ok");
        Ok(())
    }

    #[test]
    fn test_eval_assignment_then_export() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        cmd.execute(&["VAR_FROM_EVAL=one; export VAR_FROM_EVAL".to_string()])?;
        assert_eq!(std::env::var("VAR_FROM_EVAL").unwrap(), "one");
        Ok(())
    }

    #[test]
    fn test_eval_nested_eval_statement() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        cmd.execute(&["eval \"export NESTED_EVAL=ok\"".to_string()])?;
        assert_eq!(std::env::var("NESTED_EVAL").unwrap(), "ok");
        Ok(())
    }

    #[test]
    fn test_eval_test_condition_or_short_circuit() -> Result<(), CommandError> {
        let cmd = EvalCommand::new(setup_executor());
        std::env::remove_var("MANPATH");
        cmd.execute(&["[ -z \"${MANPATH-}\" ] || export SHOULD_NOT_EXIST=yes".to_string()])?;
        assert!(std::env::var("SHOULD_NOT_EXIST").is_err());
        Ok(())
    }
}
