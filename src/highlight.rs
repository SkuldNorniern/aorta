use inksac::prelude::*;

#[derive(Debug, Clone, Copy)]
pub struct SyntaxHighlighter {
    color_support: ColorSupport,
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

        // Highlight command name in cyan
        let command_style = Style::builder()
            .foreground(Color::Cyan)
            .bold()
            .build();
        parts[0] = parts[0].clone().style(command_style).to_string();

        // Highlight flags/options in yellow
        for i in 1..parts.len() {
            if parts[i].starts_with('-') {
                let flag_style = Style::builder()
                    .foreground(Color::Yellow)
                    .build();
                parts[i] = parts[i].clone().style(flag_style).to_string();
            }
        }

        parts.join(" ")
    }

    pub fn highlight_error(&self, error: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return error.to_string();
        }

        let error_style = Style::builder()
            .foreground(Color::Red)
            .bold()
            .build();
        
        error.style(error_style).to_string()
    }

    pub fn highlight_success(&self, message: &str) -> String {
        if matches!(self.color_support, ColorSupport::NoColor) {
            return message.to_string();
        }

        let success_style = Style::builder()
            .foreground(Color::Green)
            .build();
        
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