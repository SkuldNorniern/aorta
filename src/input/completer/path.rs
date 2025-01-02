use std::{
    fs,
    path::{Path, PathBuf},
};

use rustyline::completion::Pair;


#[derive(Clone)]
pub struct PathCompleter;

impl PathCompleter {
    pub fn new() -> Self {
        Self
    }

    pub fn complete_path(&self, incomplete: &str) -> Vec<Pair> {
        let (dir_to_search, file_prefix, is_absolute) = self.parse_path_input(incomplete);
        let base_path = if is_absolute { PathBuf::from("/") } else { PathBuf::new() };
        
        self.get_path_matches(&dir_to_search, file_prefix, is_absolute, base_path)
    }

    fn parse_path_input(&self, incomplete: &str) -> (PathBuf, String, bool) {
        let path = Path::new(incomplete);
        let is_absolute = incomplete.starts_with('/');

        let (dir_to_search, file_prefix) = if incomplete.is_empty() {
            (PathBuf::from("."), String::new())
        } else if incomplete.ends_with('/') {
            (PathBuf::from(incomplete), String::new())
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

        if let Ok(entries) = fs::read_dir(dir_to_search) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.starts_with(&file_prefix) {
                        if let Some(pair) = self.create_completion_pair(
                            name,
                            &entry.path(),
                            dir_to_search,
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
            let mut full_path = if is_absolute {
                base_path.join(dir_to_search)
            } else {
                dir_to_search.to_path_buf()
            };
            full_path.push(name);
            full_path.to_string_lossy().into_owned()
        };

        let (display, replacement) = if is_dir {
            (
                format!("{}/", relative_path),
                format!("{}/", relative_path),
            )
        } else {
            (
                relative_path.clone(),
                format!("{} ", relative_path),
            )
        };

        Some(Pair { display, replacement })
    }
} 