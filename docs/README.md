# SecretScout Documentation

Welcome to the SecretScout documentation. SecretScout is a blazingly fast, memory-safe CLI tool and GitHub Action for detecting secrets in git repositories.

## Quick Links

### Getting Started
- [Main README](../README.md) - Quick start and basic usage
- [Installation Guide](#installation) - How to install SecretScout
- [CLI Usage Guide](CLI_USAGE.md) - Comprehensive CLI documentation
- [GitHub Actions Guide](GITHUB_ACTIONS.md) - GitHub Actions setup

### Reference
- [CHANGELOG](CHANGELOG.md) - Version history and changes
- [MIGRATION](MIGRATION.md) - Migration guide from v2 to v3
- [Architecture](ARCHITECTURE.md) - Technical architecture details
- [CLI Quick Reference](CLI_QUICK_REFERENCE.md) - Quick CLI commands reference

### Development
- [Implementation Reports](#implementation-reports) - Development phase reports
- [SPARC Methodology](#sparc-documentation) - SPARC development process

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/globalbusinessadvisors/SecretScout.git
cd SecretScout

# Build the CLI tool
cargo build --release

# Binary will be at: target/release/secretscout
```

### Install Globally

```bash
# Install from local source
cargo install --path secretscout

# Now use from anywhere
secretscout detect --source ~/projects/my-repo
```

## Usage

### CLI Mode

```bash
# Scan current repository
secretscout detect

# Scan specific repository
secretscout detect --source /path/to/repo

# Scan staged changes (pre-commit)
secretscout protect --staged

# Show version
secretscout version
```

### GitHub Actions Mode

Add to `.github/workflows/secretscout.yml`:

```yaml
name: Secret Scan
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: globalbusinessadvisors/SecretScout@v3
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## Core Documentation

### User Guides

- **[CLI Usage Guide](CLI_USAGE.md)** - Complete CLI documentation with examples
- **[CLI Quick Reference](CLI_QUICK_REFERENCE.md)** - Quick command reference
- **[GitHub Actions Guide](GITHUB_ACTIONS.md)** - GitHub Actions configuration

### Technical Documentation

- **[Architecture](ARCHITECTURE.md)** - System architecture and design
- **[CLI Transformation](CLI_TRANSFORMATION.md)** - CLI implementation details
- **[CHANGELOG](CHANGELOG.md)** - Version history and release notes
- **[MIGRATION](MIGRATION.md)** - Migration guide from v2 to v3

## Implementation Reports

These documents track the SPARC methodology development process:

### SPARC Phases

1. **Specification** - [SPARC_SPECIFICATION.md](SPARC_SPECIFICATION.md)
2. **Pseudocode** - [PSEUDOCODE.md](PSEUDOCODE.md)
3. **Architecture** - [ARCHITECTURE.md](ARCHITECTURE.md)
4. **Refinement** - [REFINEMENT_COMPLETE.md](REFINEMENT_COMPLETE.md)
5. **Completion** - [COMPLETION_PHASE_SUMMARY.md](COMPLETION_PHASE_SUMMARY.md)

### Phase Reports

- [PSEUDOCODE_PHASE_COMPLETE.md](PSEUDOCODE_PHASE_COMPLETE.md) - Pseudocode phase summary
- [REFINEMENT_PHASE_COMPLETE.md](REFINEMENT_PHASE_COMPLETE.md) - Refinement phase summary
- [COMPLETION_PHASE_SUMMARY.md](COMPLETION_PHASE_SUMMARY.md) - Final completion summary
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) - Overall implementation summary
- [IMPLEMENTATION_REPORT.md](IMPLEMENTATION_REPORT.md) - CLI implementation report

## Features

### Performance
- **10x Faster** - Rust-powered performance
- **60% Less Memory** - Efficient memory usage
- **Intelligent Caching** - Fast binary downloads

### Security
- **Memory Safe** - Rust guarantees
- **Path Traversal Prevention** - Secure file operations
- **Command Injection Protection** - Safe git operations
- **Input Validation** - Comprehensive checks

### Functionality
- **Dual Mode** - CLI and GitHub Actions
- **Pre-commit Hooks** - Protect before commit
- **Multiple Formats** - SARIF, JSON, CSV, text
- **Zero Config** - Works out of the box

## Support

- [GitHub Issues](https://github.com/globalbusinessadvisors/SecretScout/issues) - Report bugs or request features
- [GitHub Discussions](https://github.com/globalbusinessadvisors/SecretScout/discussions) - Ask questions and share ideas
- [Main README](../README.md) - Project overview and quick start

## Contributing

Contributions are welcome! Please see the main [README](../README.md#contributing) for contribution guidelines.

## License

MIT License - see [LICENSE](../LICENSE) for details.

---

*Documentation for SecretScout v3.1.0*
