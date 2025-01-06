pub(crate) trait EnvironmentHandler {
    fn expand_env_vars(&self, input: &str) -> String;
}

impl EnvironmentHandler for super::Shell {
    fn expand_env_vars(&self, input: &str) -> String {
        let mut result = input.to_string();

        while let Some(dollar_pos) = result.find('$') {
            if dollar_pos + 1 >= result.len() {
                break;
            }

            let var_end = result[dollar_pos + 1..]
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .map_or(result.len(), |pos| pos + dollar_pos + 1);

            let var_name = &result[dollar_pos + 1..var_end];

            if let Ok(value) = std::env::var(var_name) {
                result.replace_range(dollar_pos..var_end, &value);
            } else {
                result.replace_range(dollar_pos..var_end, "");
            }
        }

        result
    }
}
