use inksac::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SyntaxHighlighter {
    color_support: ColorSupport,
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        let support = check_color_support().unwrap_or(ColorSupport::NoColor);
        Self {
            color_support: support,
        }
    }

    pub fn highlight_command(&self, input: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return input.to_string();
        }

        let mut parts: Vec<String> = input.split_whitespace().map(String::from).collect();
        if parts.is_empty() {
            return input.to_string();
        }

        let command_style = Style::builder().foreground(Color::Cyan).bold().build();
        parts[0] = parts[0].clone().style(command_style).to_string();

        let flag_style = Style::builder().foreground(Color::Yellow).build();
        for part in parts.iter_mut().skip(1) {
            if part.starts_with('-') {
                *part = part.clone().style(flag_style).to_string();
            }
        }

        parts.join(" ")
    }

    pub fn highlight_error(&self, error: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return error.to_string();
        }
        let error_style = Style::builder().foreground(Color::Red).bold().build();
        error.style(error_style).to_string()
    }

    pub fn highlight_success(&self, message: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return message.to_string();
        }
        let success_style = Style::builder().foreground(Color::Green).build();
        message.style(success_style).to_string()
    }

    pub fn highlight_hint(&self, hint: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return hint.to_string();
        }
        let hint_style = Style::builder()
            .foreground(Color::RGB(128, 128, 128))
            .build();
        hint.style(hint_style).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_command_preserves_input_content() {
        let highlighter = SyntaxHighlighter::new();
        let result = highlighter.highlight_command("cmd -flag arg");
        assert!(result.contains("cmd"));
        assert!(result.contains("-flag"));
        assert!(result.contains("arg"));
    }

    #[test]
    fn test_highlight_command_empty_parts() {
        let highlighter = SyntaxHighlighter::new();
        let result = highlighter.highlight_command("   ");
        assert_eq!(result, "   ");
    }

    #[test]
    fn test_highlight_error_success_hint_return_non_empty() {
        let highlighter = SyntaxHighlighter::new();
        assert!(highlighter.highlight_error("err").contains("err"));
        assert!(highlighter.highlight_success("ok").contains("ok"));
        assert!(highlighter.highlight_hint("hint").contains("hint"));
    }
}
