use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::path::PathExpander;
use rustyline::completion::Pair;

#[derive(Clone)]
pub struct PathCompleter {
    path_expander: PathExpander,
}

impl PathCompleter {
    pub fn new() -> Self {
        Self {
            path_expander: PathExpander::new(),
        }
    }

    pub fn complete_path(&self, incomplete: &str) -> Vec<Pair> {
        let (dir_to_search, file_prefix, is_tilde) = self.parse_path_input(incomplete);
        self.get_path_matches(&dir_to_search, &file_prefix, is_tilde)
    }

    fn parse_path_input(&self, incomplete: &str) -> (PathBuf, String, bool) {
        let is_tilde = incomplete.starts_with('~');
        let path = PathBuf::from(incomplete);

        // Handle empty input
        if incomplete.is_empty() {
            return (PathBuf::from("."), String::new(), false);
        }

        // Handle directory completion (ends with /)
        if incomplete.ends_with('/') {
            return (path, String::new(), is_tilde);
        }

        // Get parent directory and file prefix
        if let Some(parent) = path.parent() {
            let prefix = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let dir = if parent.as_os_str().is_empty() {
                PathBuf::from(".")
            } else {
                parent.to_path_buf()
            };

            (dir, prefix, is_tilde)
        } else {
            (PathBuf::from("."), incomplete.to_string(), is_tilde)
        }
    }

    fn get_path_matches(
        &self,
        dir_to_search: &Path,
        file_prefix: &str,
        is_tilde: bool,
    ) -> Vec<Pair> {
        let mut matches = Vec::new();
        let search_dir = if is_tilde {
            self.path_expander
                .expand(dir_to_search.to_str().unwrap_or(""))
                .unwrap_or_else(|_| dir_to_search.to_path_buf())
        } else {
            dir_to_search.to_path_buf()
        };

        if let Ok(entries) = fs::read_dir(&search_dir) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(file_prefix) {
                        if let Some(pair) = self.create_completion_pair(
                            name,
                            &entry.path(),
                            dir_to_search,
                            is_tilde,
                        ) {
                            matches.push(pair);
                        }
                    }
                }
            }
        }

        matches.sort_by(|a, b| a.display.cmp(&b.display));
        matches
    }

    fn create_completion_pair(
        &self,
        name: &str,
        path: &Path,
        dir_to_search: &Path,
        is_tilde: bool,
    ) -> Option<Pair> {
        let is_dir = path.is_dir();

        // Preserve the tilde in the path if it was used
        let relative_path = if is_tilde {
            let without_tilde = dir_to_search
                .strip_prefix("~")
                .unwrap_or(dir_to_search)
                .join(name);
            format!("~/{}", without_tilde.display())
        } else if dir_to_search == Path::new(".") {
            name.to_string()
        } else {
            dir_to_search.join(name).to_string_lossy().into_owned()
        };

        // Keep the original path style (relative/absolute)
        let display_path = relative_path;

        let (display, replacement) = if is_dir {
            (format!("{}/", display_path), format!("{}/", display_path))
        } else {
            (display_path.clone(), format!("{} ", display_path))
        };

        Some(Pair {
            display,
            replacement,
        })
    }
}
