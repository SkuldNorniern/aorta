# Aorta

A modern, feature-rich shell for Linux written in Rust.

## Features

- **Intelligent Command Completion**
  - Context-aware command suggestions
  - Path completion with tilde expansion
  - Alias completion with descriptive hints

- **Advanced Configuration**
  - Comprehensive environment variable management
  - Conditional configuration blocks
  - Shell alias support
  - Path sanitization and expansion

- **History Management**
  - Persistent command history
  - History search with multiple modes
  - Duplicate entry prevention
  - Configurable history size

- **Shell Capabilities**
  - Pipeline execution
  - Built-in commands (cd, exit, source, etc.)
  - Error handling with detailed messages
  - Command chaining

## Installation

```bash
cargo install aorta
```

## Configuration

Create `~/.aortarc` to customize your shell:

```bash
# Theme: minimal, default, or custom
theme minimal

# Plugins: load from ~/.aorta/plugins/<name>/<name>.aorta
plugins git

# Customization
alias ll=ls -la
export EDITOR=vim
```

Theme vs prompt (important):

- `theme` controls visual styling (highlight colors for prompt parts).
- `AORTA_PROMPT` / `prompt_preset` controls prompt structure/content (`%u`, `%h`, `%~`, etc.).
- This separation avoids duplication: shape comes from prompt format, style comes from theme.

Program-level settings live in `~/.config/aorta/config.toml`:

```toml
[paths]
aorta_home = "~/.aorta"
rc_path = "~/.aortarc"

[loader]
compat_mode = "native" # or "compat" for zsh/bash-like profile snippets

[defaults]
# Used when Aorta first generates ~/.aortarc
# Theme choices: minimal, default, compact, classic, developer
theme = "minimal"
# Plugins load from ~/.aorta/plugins/<name>/<name>.aorta
plugins = ["git"]
editor = "vi"

# Starship-inspired presets: minimal, compact, classic, developer
prompt_preset = "minimal"

# Optional explicit prompt format override
# prompt_format = "%u@%h:%~$ "
# prompt_format = "%~ $ "
# prompt_format = "%u@%h %~ [dev]$ "

# Legacy alias still accepted:
# [bootstrap]
```

### Prompt Presets

Use `prompt_preset` in `~/.config/aorta/config.toml` to choose a default style for generated `~/.aortarc`.

- `minimal` -> `%~ > `
- `compact` -> `%~ $ `
- `classic` -> `%u@%h:%~$ `
- `developer` -> `%u@%h %~ [dev]$ `

If you want full control, set `prompt_format` instead of `prompt_preset`.

```toml
[defaults]
prompt_format = "[%h] %~ -> "
```

Prompt tokens:

- `%u` username
- `%h` hostname
- `%~` cwd (home shortened)
- `%c` cwd (full path)

### Theme Examples

Themes are loaded from `~/.aorta/themes/<name>.aorta` and selected in `~/.aortarc`.

Built-in bootstrap themes:

- `minimal`
- `default`
- `compact`
- `classic`
- `developer`

Theme highlight variables:

- `AORTA_STYLE_USER`
- `AORTA_STYLE_HOST`
- `AORTA_STYLE_PATH`

Supported color names include: `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`,
`bright_red`, `bright_green`, `bright_yellow`, `bright_blue`, `bright_magenta`, `bright_cyan`,
`bright_white`, `gray`.

Example custom theme file:

```bash
# ~/.aorta/themes/work.aorta
export AORTA_STYLE_USER="bright_cyan"
export AORTA_STYLE_HOST="bright_blue"
export AORTA_STYLE_PATH="cyan"
```

Then enable it:

```bash
theme work
```

### Plugin Examples

Plugins are loaded by name from `~/.aorta/plugins/<name>/<name>.aorta`.

Example:

```bash
# ~/.aorta/plugins/docker/docker.aorta
alias dps='docker ps'
alias dcu='docker compose up'
alias dcd='docker compose down'
```

Enable it in `~/.aortarc`:

```bash
plugins git docker
```

Migration notes:

- On first run, if `~/.aortarc` does not exist, Aorta scans common shell configs (`.zshrc`, `.bashrc`, `.bash_profile`, `.profile`, `~/.config/fish/config.fish`).
- It imports compatible lines into the generated `~/.aortarc` (`alias`, `export`, `source`) and adds migration notes for unsupported patterns.
- This keeps startup lightweight while helping you move from zsh/bash/fish incrementally.

## Development

```bash
# Clone the repository
git clone https://github.com/yourusername/aorta.git
cd aorta

# Build
cargo build

# Run tests
cargo test
```

## Architecture

- **Core Components**
  - Command execution engine
  - Configuration management
  - Environment variable handling
  - Path expansion utilities

- **Input Processing**
  - Command completion system
  - History management
  - Line editing capabilities

- **Error Handling**
  - Comprehensive error types
  - Graceful error recovery
  - Detailed error messages

## Contributing

Contributions are welcome. Please ensure your changes:
1. Include appropriate tests
2. Follow the existing code style
3. Update documentation as needed
4. Add error handling where appropriate

## License

[Mozilla Public License 2.0](LICENSE)

## Why Aorta?

Just as the aorta is the main artery carrying blood from your heart, Aorta shell aims to be the main conduit for your interaction with the Linux/Artery system - reliable, efficient, and essential.

## ⚠️ Important Disclaimer

**EXPERIMENTAL STATUS**: Aorta is currently in early development and is **NOT** intended for production use.

- This is an experimental shell implementation
- May contain bugs that could affect system stability
- Use in a production environment is strongly discouraged
- Testing in isolated environments is recommended

**LIABILITY**: By using Aorta, you acknowledge and agree that:
- The author(s) are not responsible for any damage or data loss
- Use of this software is entirely at your own risk
- No warranty or guarantee of fitness for any purpose is provided

## Recommended Usage

- Development and testing environments only
- Virtual machines or containers
- Non-critical systems
- Educational purposes

---
<sub>*Despite all these warnings, I actually use Aorta as my main shell. Do as I say, not as I do! 😅*</sub>
