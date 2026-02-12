use crate::error::ShellError;
use crate::path;
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
        let home = path::home_dir().ok_or(ShellError::HomeDirNotFound)?;

        match path {
            "~" => Ok(home),
            path if path.starts_with("~/") => {
                let remainder = &path[2..];
                Ok(home.join(remainder))
            }
            _ => Ok(Path::new(path).to_path_buf()),
        }
    }

    pub fn is_home_path(&self, path: &str) -> bool {
        path.starts_with('~')
    }

    pub fn get_home_dir(&self) -> Result<PathBuf, ShellError> {
        path::home_dir().ok_or(ShellError::HomeDirNotFound)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path;

    #[test]
    fn test_expand_tilde() {
        let expander = PathExpander::new();
        let home = path::home_dir().expect("HOME env var required for test");

        // Test single tilde
        assert_eq!(expander.expand("~").unwrap(), home);

        // Test tilde with slash
        assert_eq!(expander.expand("~/").unwrap(), home);

        // Test tilde with path
        assert_eq!(expander.expand("~/test").unwrap(), home.join("test"));

        // Test tilde with nested path
        assert_eq!(
            expander.expand("~/test/nested").unwrap(),
            home.join("test").join("nested")
        );
    }

    #[test]
    fn test_non_tilde_paths() {
        let expander = PathExpander::new();

        // Test absolute path
        assert_eq!(
            expander.expand("/usr/local").unwrap(),
            PathBuf::from("/usr/local")
        );

        // Test relative path
        assert_eq!(expander.expand("./test").unwrap(), PathBuf::from("./test"));
    }
}
