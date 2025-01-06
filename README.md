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
# Environment variables
export PATH="$HOME/.local/bin:$PATH"
export EDITOR="vim"

# Aliases
alias ll='ls -la'
alias gs='git status'

# Conditional configuration
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi
```

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
