use std::fs;

use super::ConfigError;

const DEFAULT_AORTARC: &str = r#"# Aorta shell config
# See examples in ~/.aorta or $AORTA_HOME

theme minimal
alias ll=ls -la
export EDITOR=vi
"#;

const MIGRATION_HEADER: &str = r#"# Aorta shell config (migrated from zsh)
# Your original .zshrc is at ~/.zshrc
# Aorta uses different syntax - copy alias and export lines as needed.
# Example: alias ll=ls -la
# Example: export PATH="$HOME/bin:$PATH"

theme minimal
alias ll=ls -la
export EDITOR=vi
"#;

pub fn ensure_aortarc(home: &std::path::Path) -> Result<(), ConfigError> {
    let rc_path = home.join(".aortarc");
    if rc_path.exists() {
        return Ok(());
    }

    let content = if home.join(".zshrc").exists() {
        MIGRATION_HEADER
    } else {
        DEFAULT_AORTARC
    };

    fs::write(&rc_path, content)?;
    Ok(())
}

pub fn ensure_aorta_home(home: &std::path::Path) -> Result<std::path::PathBuf, ConfigError> {
    let aorta_home = std::env::var_os("AORTA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".aorta"));

    if !aorta_home.exists() {
        fs::create_dir_all(&aorta_home)?;
        let themes = aorta_home.join("themes");
        let plugins = aorta_home.join("plugins");
        fs::create_dir_all(&themes)?;
        fs::create_dir_all(&plugins)?;

        let default_theme = themes.join("minimal.aorta");
        if !default_theme.exists() {
            fs::write(default_theme, "export AORTA_PROMPT=\"%~ > \"\n")?;
        }
    }

    Ok(aorta_home)
}
