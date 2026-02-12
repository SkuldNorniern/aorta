use super::ConfigError;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub rc_path: PathBuf,
    pub profile_path: PathBuf,
    pub aorta_home: PathBuf,
}

impl ConfigPaths {
    pub fn new(custom_rc_path: Option<&str>) -> Result<Self, ConfigError> {
        let home_path =
            crate::path::home_dir().ok_or(ConfigError::HomeDirNotFound)?;

        let aorta_home = std::env::var_os("AORTA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home_path.join(".aorta"));

        let rc_path = match custom_rc_path {
            Some(p) => PathBuf::from(p),
            None => home_path.join(".aortarc"),
        };

        Ok(ConfigPaths {
            rc_path,
            profile_path: home_path.join(".profile"),
            aorta_home,
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
        let paths = ConfigPaths::new(None).unwrap();

        assert_eq!(paths.rc_path, PathBuf::from("/home/testuser/.aortarc"));
        assert_eq!(paths.profile_path, PathBuf::from("/home/testuser/.profile"));
    }

    #[test]
    fn test_custom_config_path() {
        env::set_var("HOME", "/home/testuser");
        let paths = ConfigPaths::new(Some("/etc/aorta/config")).unwrap();
        assert_eq!(paths.rc_path, PathBuf::from("/etc/aorta/config"));
    }

    #[test]
    fn test_missing_home() {
        env::remove_var("HOME");
        assert!(matches!(
            ConfigPaths::new(None),
            Err(ConfigError::HomeDirNotFound)
        ));
    }

    #[test]
    fn test_aorta_home_default() {
        env::set_var("HOME", "/home/user");
        env::remove_var("AORTA_HOME");
        let paths = ConfigPaths::new(None).unwrap();
        assert_eq!(paths.aorta_home, PathBuf::from("/home/user/.aorta"));
    }

    #[test]
    fn test_aorta_home_custom() {
        env::set_var("HOME", "/home/user");
        env::set_var("AORTA_HOME", "/opt/aorta");
        let paths = ConfigPaths::new(None).unwrap();
        assert_eq!(paths.aorta_home, PathBuf::from("/opt/aorta"));
        env::remove_var("AORTA_HOME");
    }
}
