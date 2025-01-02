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
        let (dir_to_search, file_prefix, is_absolute) = self.parse_path_input(incomplete);
        let base_path = if is_absolute {
            PathBuf::from("/")
        } else if self.path_expander.is_home_path(incomplete) {
            self.path_expander.get_home_dir().unwrap_or_default()
        } else {
            PathBuf::new()
        };

        self.get_path_matches(&dir_to_search, file_prefix, is_absolute, base_path)
    }

    fn parse_path_input(&self, incomplete: &str) -> (PathBuf, String, bool) {
        let is_absolute = incomplete.starts_with('/');

        let path = if let Ok(expanded) = self.path_expander.expand(incomplete) {
            expanded
        } else {
            PathBuf::from(incomplete)
        };

        let (dir_to_search, file_prefix) = if incomplete.is_empty() {
            (PathBuf::from("."), String::new())
        } else if incomplete.ends_with('/') {
            (path, String::new())
        } else if let Some(parent) = path.parent() {
            (
                if parent.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                },
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string(),
            )
        } else {
            (PathBuf::from("."), incomplete.to_string())
        };

        (dir_to_search, file_prefix, is_absolute)
    }

    fn get_path_matches(
        &self,
        dir_to_search: &Path,
        file_prefix: String,
        is_absolute: bool,
        base_path: PathBuf,
    ) -> Vec<Pair> {
        let mut matches = Vec::new();
        let search_dir = if dir_to_search == Path::new("~") {
            dirs::home_dir().unwrap_or_else(|| dir_to_search.to_path_buf())
        } else {
            dir_to_search.to_path_buf()
        };

        if let Ok(entries) = fs::read_dir(&search_dir) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&file_prefix) {
                        if let Some(pair) = self.create_completion_pair(
                            name,
                            &entry.path(),
                            &search_dir,
                            is_absolute,
                            &base_path,
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
        is_absolute: bool,
        base_path: &Path,
    ) -> Option<Pair> {
        let is_dir = path.is_dir();

        let relative_path = if dir_to_search == Path::new(".") {
            name.to_string()
        } else {
            let full_path = if is_absolute {
                base_path.join(dir_to_search)
            } else if dir_to_search.starts_with("~") {
                if let Some(home) = dirs::home_dir() {
                    home.join(name)
                } else {
                    dir_to_search.join(name)
                }
            } else {
                dir_to_search.join(name)
            };

            if let Ok(canonical) = full_path.canonicalize() {
                canonical.to_string_lossy().into_owned()
            } else {
                full_path.to_string_lossy().into_owned()
            }
        };

        let (display, replacement) = if is_dir {
            (format!("{}/", relative_path), format!("{}/", relative_path))
        } else {
            (relative_path.clone(), format!("{} ", relative_path))
        };

        Some(Pair {
            display,
            replacement,
        })
    }
}
