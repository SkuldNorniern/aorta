use super::ConfigError;
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub rc_path: PathBuf,
    pub profile_path: PathBuf,
}

impl ConfigPaths {
    pub fn new() -> Result<Self, ConfigError> {
        let home = env::var("HOME").map_err(|_| ConfigError::HomeDirNotFound)?;
        let home_path = PathBuf::from(home);

        Ok(ConfigPaths {
            rc_path: home_path.join(".aortarc"),
            profile_path: home_path.join(".profile"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_new_paths() {
        env::set_var("HOME", "/home/testuser");
        let paths = ConfigPaths::new().unwrap();

        assert_eq!(paths.rc_path, PathBuf::from("/home/testuser/.aortarc"));
        assert_eq!(paths.profile_path, PathBuf::from("/home/testuser/.profile"));
    }

    #[test]
    fn test_missing_home() {
        env::remove_var("HOME");
        assert!(matches!(
            ConfigPaths::new(),
            Err(ConfigError::HomeDirNotFound)
        ));
    }
}
