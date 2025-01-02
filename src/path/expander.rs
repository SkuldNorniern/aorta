use crate::error::ShellError;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct PathExpander;

impl Default for PathExpander {
    fn default() -> Self {
        Self::new()
    }
}

impl PathExpander {
    pub fn new() -> Self {
        Self
    }

    pub fn expand(&self, path: &str) -> Result<PathBuf, ShellError> {
        if path.starts_with('~') {
            self.expand_tilde(path)
        } else {
            Ok(Path::new(path).to_path_buf())
        }
    }

    fn expand_tilde(&self, path: &str) -> Result<PathBuf, ShellError> {
        if path.len() == 1 {
            // Just "~"
            dirs::home_dir().ok_or(ShellError::HomeDirNotFound)
        } else {
            let without_tilde = &path[1..];
            if let Some(stripped) = without_tilde.strip_prefix('/') {
                // "~/path"
                let mut home_path = dirs::home_dir().ok_or(ShellError::HomeDirNotFound)?;
                for part in stripped.split('/') {
                    if !part.is_empty() {
                        home_path.push(part);
                    }
                }
                Ok(home_path)
            } else {
                // "~username/path" - not handling this case for now
                Ok(Path::new(path).to_path_buf())
            }
        }
    }

    pub fn is_home_path(&self, path: &str) -> bool {
        path.starts_with('~')
    }

    pub fn get_home_dir(&self) -> Result<PathBuf, ShellError> {
        dirs::home_dir().ok_or(ShellError::HomeDirNotFound)
    }
}
