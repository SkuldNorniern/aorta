pub(crate) fn expand_env_vars_with<F>(input: &str, lookup: F) -> String
where
    F: Fn(&str) -> Option<String>,
{
    let mut result = input.to_string();

    while let Some(dollar_pos) = result.find('$') {
        if dollar_pos + 1 >= result.len() {
            break;
        }

        let next = result.as_bytes()[dollar_pos + 1];
        let (consumed_len, value) = if next == b'(' {
            if let Some((len, out)) = parse_command_substitution(&result[dollar_pos + 2..]) {
                (2 + len, out)
            } else {
                break;
            }
        } else if next == b'{' {
            let Some((var_name, consumed_len, default_val)) =
                parse_braced_var(&result[dollar_pos + 2..])
            else {
                break;
            };
            let val = match (lookup(var_name), default_val) {
                (Some(v), _) if !v.is_empty() => v,
                (_, Some(d)) => d.to_string(),
                _ => String::new(),
            };
            (2 + consumed_len, val)
        } else {
            let (var_name, consumed_len, default_val) = parse_simple_var(&result[dollar_pos + 1..]);
            let val = match (lookup(var_name), default_val) {
                (Some(v), _) if !v.is_empty() => v,
                (_, Some(d)) => d.to_string(),
                _ => String::new(),
            };
            (1 + consumed_len, val)
        };

        let replace_end = dollar_pos + consumed_len;
        result.replace_range(dollar_pos..replace_end, &value);
    }

    result
}

fn parse_command_substitution(rest: &str) -> Option<(usize, String)> {
    let mut depth = 1usize;
    for (pos, c) in rest.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    let inner = &rest[..pos];
                    let output = run_command_substitution(inner);
                    return Some((pos + 1, output));
                }
            }
            _ => {}
        }
    }
    None
}

fn run_command_substitution(cmd: &str) -> String {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return String::new();
    }
    let output = std::process::Command::new("sh").args(["-c", cmd]).output();
    match output {
        Ok(out) => {
            let s = String::from_utf8_lossy(&out.stdout);
            s.trim_end().to_string()
        }
        Err(_) => String::new(),
    }
}

fn parse_simple_var(rest: &str) -> (&str, usize, Option<&str>) {
    let var_end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(rest.len());
    let var_name = &rest[..var_end];
    (var_name, var_end, None)
}

fn parse_braced_var(rest: &str) -> Option<(&str, usize, Option<&str>)> {
    if let Some(close_brace) = rest.find('}') {
        let inner = &rest[..close_brace];
        let consumed = close_brace + 1;
        if let Some(colon_dash) = inner.find(":-") {
            let var_name = inner[..colon_dash].trim();
            let default_val = &inner[colon_dash + 2..];
            return Some((var_name, consumed, Some(default_val)));
        }
        if let Some(dash) = inner.find('-') {
            let var_name = inner[..dash].trim();
            let default_val = &inner[dash + 1..];
            return Some((var_name, consumed, Some(default_val)));
        }
        let var_name = inner.trim();
        return Some((var_name, consumed, None));
    }
    None
}

pub(crate) trait EnvironmentHandler {
    fn expand_env_vars(&self, input: &str) -> String;
}

impl EnvironmentHandler for super::Shell {
    fn expand_env_vars(&self, input: &str) -> String {
        expand_env_vars_with(input, |name| std::env::var(name).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars_basic() {
        let lookup = |s: &str| -> Option<String> {
            if s == "FOO" {
                Some("bar".into())
            } else if s == "EMPTY" {
                Some(String::new())
            } else {
                None
            }
        };
        assert_eq!(expand_env_vars_with("x$FOO y", lookup), "xbar y");
        assert_eq!(expand_env_vars_with("$EMPTY", lookup), "");
        assert_eq!(expand_env_vars_with("$MISSING", lookup), "");
        assert_eq!(expand_env_vars_with("no vars", lookup), "no vars");
    }

    #[test]
    fn test_expand_env_vars_with_env() {
        std::env::set_var("AORTA_TEST_EXPAND", "expanded");
        let result =
            expand_env_vars_with("test_${AORTA_TEST_EXPAND}_end", |n| std::env::var(n).ok());
        std::env::remove_var("AORTA_TEST_EXPAND");
        assert_eq!(result, "test_expanded_end");
    }

    #[test]
    fn test_expand_braced_var() {
        let lookup = |s: &str| if s == "X" { Some("val".into()) } else { None };
        assert_eq!(expand_env_vars_with("a${X}b", lookup), "avalb");
        assert_eq!(expand_env_vars_with("${X}_suffix", lookup), "val_suffix");
    }

    #[test]
    fn test_expand_default_value() {
        let lookup = |s: &str| if s == "SET" { Some("yes".into()) } else { None };
        assert_eq!(
            expand_env_vars_with("${SET:-no} ${UNSET:-default}", lookup),
            "yes default"
        );
        assert_eq!(
            expand_env_vars_with("${EMPTY:-fallback}", |s| {
                if s == "EMPTY" {
                    Some(String::new())
                } else {
                    None
                }
            }),
            "fallback"
        );
    }

    #[test]
    fn test_expand_braced_hyphen_syntax() {
        let lookup = |s: &str| if s == "SET" { Some("x".into()) } else { None };
        assert_eq!(expand_env_vars_with("${SET-foo}", lookup), "x");
        assert_eq!(expand_env_vars_with("${UNSET-bar}", lookup), "bar");
    }

    #[test]
    fn test_expand_command_substitution() {
        let lookup = |_: &str| None;
        let result = expand_env_vars_with("echo $(echo hello)", lookup);
        assert_eq!(result, "echo hello");
    }
}
