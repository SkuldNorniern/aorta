use super::EnvError;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct EnvPaths {
    // Doesn't have a size known at compile-time
    home: PathBuf,
    config_dir: PathBuf,
    cache_dir: PathBuf,
}

impl EnvPaths {
    pub fn new() -> Result<Self, EnvError> {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| EnvError::HomeDirNotFound)?;

        if !home.exists() {
            return Err(EnvError::InvalidPath(home));
        }

        let config_dir = home.join(".config");
        let cache_dir = home.join(".cache");

        Ok(Self {
            home,
            config_dir,
            cache_dir,
        })
    }

    pub fn ensure_dirs(&self) -> Result<(), EnvError> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    pub fn get_config_file(&self, name: &str) -> PathBuf {
        self.config_dir.join(name)
    }

    pub fn get_cache_file(&self, name: &str) -> PathBuf {
        self.cache_dir.join(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn setup_test_env() -> Result<(), std::io::Error> {
        let temp_dir = env::temp_dir().join("env_paths_test");
        fs::create_dir_all(&temp_dir)?;
        env::set_var("HOME", temp_dir);
        Ok(())
    }

    #[test]
    fn test_env_paths() -> Result<(), EnvError> {
        setup_test_env()?;
        let paths = EnvPaths::new()?;
        paths.ensure_dirs()?;

        assert!(paths.config_dir().exists());
        assert!(paths.cache_dir().exists());

        let config_file = paths.get_config_file("test.conf");
        assert!(config_file.starts_with(paths.config_dir()));

        Ok(())
    }
}
