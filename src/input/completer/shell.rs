use std::{borrow::Cow, collections::BTreeMap};

use super::{command::CommandCompleter, path::PathCompleter};
use crate::highlight::SyntaxHighlighter;

use rustyline::{
    completion::{Completer, Pair},
    highlight::{Highlighter, CmdKind},
    hint::Hinter,
    validate::Validator,
    Context, Helper,
};

#[derive(Clone)]
pub struct ShellCompleter {
    command_completer: CommandCompleter,
    path_completer: PathCompleter,
    highlighter: SyntaxHighlighter,
}

impl Default for ShellCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellCompleter {
    pub fn new() -> Self {
        ShellCompleter {
            command_completer: CommandCompleter::new(),
            path_completer: PathCompleter::new(),
            highlighter: SyntaxHighlighter::new(),
        }
    }

    pub fn refresh_commands(&mut self) {
        self.command_completer.refresh_commands();
    }

    pub fn update_aliases(&mut self, aliases: BTreeMap<Cow<'_, str>, Cow<'_, str>>) {
        self.command_completer.update_aliases(aliases);
    }
}

impl Helper for ShellCompleter {}
impl Highlighter for ShellCompleter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        Cow::Owned(self.highlighter.highlight_command(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> std::borrow::Cow<'h, str> {
        Cow::Owned(self.highlighter.highlight_hint(hint))
    }
}
impl Hinter for ShellCompleter {
    type Hint = String;
}
impl Validator for ShellCompleter {}

impl Completer for ShellCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let line_up_to_cursor = &line[..pos];
        let mut words: Vec<&str> = line_up_to_cursor.split_whitespace().collect();

        if line_up_to_cursor.ends_with(' ') {
            words.push("");
        }

        let (start, matches) = match words.len() {
            0 => (0, self.command_completer.complete_command("")),
            1 => {
                let word = words[0];
                let start = line_up_to_cursor.rfind(word).unwrap_or(0);
                (start, self.command_completer.complete_command(word))
            }
            _ => {
                let last_word = words.last().unwrap_or(&"");
                let start = if last_word.is_empty() {
                    pos
                } else {
                    line_up_to_cursor.rfind(last_word).unwrap_or(pos)
                };
                (start, self.path_completer.complete_path(last_word))
            }
        };

        Ok((start, matches))
    }
}
