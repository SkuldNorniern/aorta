use super::program::{CompatMode, ProgramConfig};
use super::ConfigError;
use std::path::PathBuf;

const ETC_AORTA_PROFILE: &str = "/etc/aorta/profile";
const ETC_AORTA_RC: &str = "/etc/aorta/aortarc";

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub rc_path: PathBuf,
    pub profile_path: PathBuf,
    pub aorta_home: PathBuf,
    pub system_profile: PathBuf,
    pub system_rc: PathBuf,
    pub compat_mode: CompatMode,
}

impl ConfigPaths {
    pub fn new(custom_rc_path: Option<&str>) -> Result<Self, ConfigError> {
        let home_path = crate::path::home_dir().ok_or(ConfigError::HomeDirNotFound)?;

        let prog = ProgramConfig::load(&home_path)?;

        let rc_path = match custom_rc_path {
            Some(p) => PathBuf::from(p),
            None => prog.rc_path,
        };

        Ok(ConfigPaths {
            rc_path,
            profile_path: home_path.join(".profile"),
            aorta_home: prog.aorta_home,
            system_profile: PathBuf::from(ETC_AORTA_PROFILE),
            system_rc: PathBuf::from(ETC_AORTA_RC),
            compat_mode: prog.compat_mode,
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
        assert_eq!(paths.system_profile, PathBuf::from("/etc/aorta/profile"));
        assert_eq!(paths.system_rc, PathBuf::from("/etc/aorta/aortarc"));
        assert_eq!(paths.compat_mode, CompatMode::Native);
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
