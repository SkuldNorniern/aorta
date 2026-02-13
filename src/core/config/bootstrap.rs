use std::fs;

use super::migrator::MigrationResult;
use super::program::BootstrapConfig;
use super::ConfigError;

fn build_aortarc_template(
    defaults: &BootstrapConfig,
    migration: Option<&MigrationResult>,
) -> String {
    let migration_plugins = migration
        .map(|m| m.plugins.join(" "))
        .unwrap_or_default()
        .trim()
        .to_string();
    let plugins = if !migration_plugins.is_empty() {
        migration_plugins
    } else if defaults.plugins.is_empty() {
        "git".to_string()
    } else {
        defaults.plugins.join(" ")
    };

    let prompt = defaults.resolved_prompt();
    let migration_note = if let Some(m) = migration {
        if let Some(path) = &m.source_file {
            format!(
                "# Migrated from existing shell config: {}\n# Parsed aliases/exports/source lines were copied below.\n\n",
                path.display()
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let migrated_blocks = migration
        .map(build_migrated_blocks)
        .unwrap_or_else(|| "# No legacy shell config found to migrate.\n".to_string());

    format!(
        "# Oh My Zsh-style Aorta config\n\
# This file is intentionally simple and safe to edit.\n\
# Sections:\n\
#   1) Theme\n\
#   2) Plugins\n\
#   3) Prompt (starship-inspired presets)\n\
#   4) Personal customization\n\
\n\
{}\
# Theme (visual style profile): minimal, default, compact, classic, developer\n\
theme {}\n\
\n\
# Plugins (space separated): load from ~/.aorta/plugins/<name>/<name>.aorta\n\
plugins {}\n\
\n\
# Prompt:\n\
#   Presets are configured in ~/.config/aorta/config.toml [defaults].\n\
#   Tokens you can use: %u=user %h=host %~=short cwd %c=full cwd\n\
export AORTA_PROMPT=\"{}\"\n\
\n\
# Personal customization\n\
export EDITOR={}\n\
alias ll=ls -la\n\
\n\
# Examples:\n\
# alias gs=git status\n\
# export PATH=\"$HOME/.local/bin:$PATH\"\n\
# source \"$HOME/.cargo/env\"\n\
\n\
{}",
        migration_note, defaults.theme, plugins, prompt, defaults.editor, migrated_blocks
    )
}

fn build_migrated_blocks(migration: &MigrationResult) -> String {
    let mut out = String::new();
    out.push_str("# Migrated entries\n");

    if !migration.notes.is_empty() {
        out.push_str("\n# Notes from migration\n");
        for note in &migration.notes {
            out.push_str(&format!("# - {}\n", note));
        }
    }

    if !migration.exports.is_empty() {
        out.push_str("\n# Imported exports\n");
        for line in &migration.exports {
            out.push_str(line);
            out.push('\n');
        }
    }

    if !migration.aliases.is_empty() {
        out.push_str("\n# Imported aliases\n");
        for line in &migration.aliases {
            out.push_str(line);
            out.push('\n');
        }
    }

    if !migration.sources.is_empty() {
        out.push_str("\n# Imported source lines\n");
        for line in &migration.sources {
            out.push_str(line);
            out.push('\n');
        }
    }

    if out.trim() == "# Migrated entries" {
        out.push_str("\n# No compatible alias/export/source lines were found.\n");
    }

    out
}

pub fn ensure_aortarc(
    home: &std::path::Path,
    defaults: &BootstrapConfig,
) -> Result<(), ConfigError> {
    let rc_path = home.join(".aortarc");
    if rc_path.exists() {
        return Ok(());
    }

    let migration = super::migrator::migrate_from_shell_config(home)?;
    let content = build_aortarc_template(defaults, migration.as_ref());

    fs::write(&rc_path, content)?;
    Ok(())
}

pub fn ensure_aorta_home(home: &std::path::Path) -> Result<std::path::PathBuf, ConfigError> {
    let aorta_home = std::env::var_os("AORTA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".aorta"));

    fs::create_dir_all(&aorta_home)?;
    let themes = aorta_home.join("themes");
    let plugins = aorta_home.join("plugins");
    let custom = aorta_home.join("custom");
    fs::create_dir_all(&themes)?;
    fs::create_dir_all(&plugins)?;
    fs::create_dir_all(&custom)?;

    let minimal_theme = themes.join("minimal.aorta");
    if !minimal_theme.exists() {
        fs::write(minimal_theme, "# Minimal theme (no color overrides)\n")?;
    }

    let default_theme = themes.join("default.aorta");
    if !default_theme.exists() {
        fs::write(
            default_theme,
            "export AORTA_STYLE_USER=\"bright_cyan\"\nexport AORTA_STYLE_HOST=\"bright_blue\"\nexport AORTA_STYLE_PATH=\"cyan\"\n",
        )?;
    }

    let compact_theme = themes.join("compact.aorta");
    if !compact_theme.exists() {
        fs::write(
            compact_theme,
            "export AORTA_STYLE_USER=\"gray\"\nexport AORTA_STYLE_HOST=\"bright_black\"\nexport AORTA_STYLE_PATH=\"white\"\n",
        )?;
    }

    let classic_theme = themes.join("classic.aorta");
    if !classic_theme.exists() {
        fs::write(
            classic_theme,
            "export AORTA_STYLE_USER=\"green\"\nexport AORTA_STYLE_HOST=\"yellow\"\nexport AORTA_STYLE_PATH=\"bright_green\"\n",
        )?;
    }

    let developer_theme = themes.join("developer.aorta");
    if !developer_theme.exists() {
        fs::write(
            developer_theme,
            "export AORTA_STYLE_USER=\"bright_magenta\"\nexport AORTA_STYLE_HOST=\"bright_cyan\"\nexport AORTA_STYLE_PATH=\"bright_yellow\"\n",
        )?;
    }

    let git_plugin_dir = plugins.join("git");
    fs::create_dir_all(&git_plugin_dir)?;
    let git_plugin = git_plugin_dir.join("git.aorta");
    if !git_plugin.exists() {
        fs::write(
            git_plugin,
            "# Git plugin - useful aliases\nalias gst=git status\nalias gco=git checkout\nalias gcm=git commit -m\nalias gp=git push\nalias gl=git pull\n",
        )?;
    }

    Ok(aorta_home)
}
