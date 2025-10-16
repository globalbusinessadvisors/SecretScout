# SecretScout CLI Usage Guide

SecretScout v3.0.0 introduces a complete CLI interface, transforming it from a GitHub Actions-only tool into a general-purpose secret detection tool like gitleaks itself.

## Table of Contents

- [Installation](#installation)
- [Quick Start](#quick-start)
- [Commands](#commands)
  - [detect](#detect-command)
  - [protect](#protect-command)
  - [version](#version-command)
- [Configuration](#configuration)
- [Use Cases](#use-cases)
- [Integration](#integration)
- [Dual-Mode Architecture](#dual-mode-architecture)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/globalbusinessadvisors/SecretScout.git
cd SecretScout

# Build and install
cargo install --path secretscout

# Verify installation
secretscout version
```

### From Cargo (future)

```bash
cargo install secretscout
```

### Binary Download (future)

```bash
# Linux
curl -sSfL https://github.com/globalbusinessadvisors/SecretScout/releases/latest/download/secretscout-linux-x64 -o secretscout
chmod +x secretscout

# macOS
curl -sSfL https://github.com/globalbusinessadvisors/SecretScout/releases/latest/download/secretscout-darwin-x64 -o secretscout
chmod +x secretscout

# Windows
curl -sSfL https://github.com/globalbusinessadvisors/SecretScout/releases/latest/download/secretscout-windows-x64.exe -o secretscout.exe
```

## Quick Start

```bash
# Scan current directory
secretscout detect

# Scan specific repository
secretscout detect --source /path/to/repo

# Protect staged changes (pre-commit hook)
secretscout protect --staged

# Show help
secretscout --help
secretscout detect --help
secretscout protect --help
```

## Commands

### detect Command

Scan a repository for secrets in git history.

#### Syntax

```bash
secretscout detect [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --source <PATH>` | Path to git repository | `.` (current dir) |
| `-r, --report-path <PATH>` | Path to write SARIF report | `results.sarif` |
| `-f, --report-format <FORMAT>` | Report format (sarif, json, csv, text) | `sarif` |
| `--redact` | Redact secrets in output | `true` |
| `--exit-code <CODE>` | Exit code when leaks detected | `2` |
| `--log-opts <OPTS>` | Git log options | None |
| `-c, --config <PATH>` | Path to gitleaks config file | Auto-detect |
| `-v, --verbose` | Enable verbose logging | `false` |
| `-h, --help` | Print help | - |

#### Examples

**Basic scan:**
```bash
secretscout detect
```

**Scan specific directory:**
```bash
secretscout detect --source /path/to/repo
```

**Custom config:**
```bash
secretscout detect --config custom.toml
```

**JSON output:**
```bash
secretscout detect --report-format json --report-path findings.json
```

**Scan specific git range:**
```bash
# Compare two branches
secretscout detect --log-opts "main..feature-branch"

# Scan specific commits
secretscout detect --log-opts "HEAD~10..HEAD"

# Full repository scan
secretscout detect --log-opts "--all"
```

**Verbose output:**
```bash
secretscout detect --verbose
```

**Custom exit code:**
```bash
secretscout detect --exit-code 1
```

#### Exit Codes

- `0`: No secrets found
- `2`: Secrets detected (default)
- Custom: Set with `--exit-code`

### protect Command

Scan staged changes in git repository. Perfect for pre-commit hooks.

#### Syntax

```bash
secretscout protect [OPTIONS]
```

#### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-s, --source <PATH>` | Path to git repository | `.` (current dir) |
| `--staged` | Scan staged changes only | `true` |
| `-c, --config <PATH>` | Path to gitleaks config file | Auto-detect |
| `-v, --verbose` | Enable verbose logging | `false` |
| `-h, --help` | Print help | - |

#### Examples

**Scan staged changes:**
```bash
secretscout protect --staged
```

**Custom config:**
```bash
secretscout protect --config .gitleaks.toml
```

**Verbose output:**
```bash
secretscout protect --verbose
```

#### Exit Codes

- `0`: No secrets in staged changes
- `1`: Secrets found in staged changes

### version Command

Print version information.

#### Syntax

```bash
secretscout version
```

#### Output

```
secretscout 3.0.0
```

## Configuration

### Config File Detection

SecretScout automatically detects configuration files in this order:

1. Path specified via `--config` flag
2. `gitleaks.toml` in current directory
3. `.gitleaks.toml` in current directory
4. Default gitleaks configuration

### Custom Config Example

Create `.gitleaks.toml`:

```toml
title = "SecretScout Custom Config"

[[rules]]
description = "AWS Access Key"
id = "aws-access-key"
regex = '''AKIA[0-9A-Z]{16}'''

[[rules]]
description = "Generic API Key"
id = "generic-api-key"
regex = '''(?i)api[_-]?key['\"]?\s*[:=]\s*['\"]?[a-z0-9]{32,45}['\"]?'''

[[rules]]
description = "Private Key"
id = "private-key"
regex = '''-----BEGIN (RSA|DSA|EC|OPENSSH) PRIVATE KEY-----'''

[allowlist]
description = "Allowlist paths"
paths = [
  "vendor/",
  ".terraform/",
  "node_modules/",
  "*.test.js",
  "test/**",
]

[allowlist]
description = "Allowlist regexes"
regexes = [
  '''example\.com''',
  '''test[_-]key''',
]
```

### Global Options

Global options can be specified before the command:

```bash
secretscout --config custom.toml --verbose detect
secretscout -c custom.toml -v detect --source /path/to/repo
```

## Use Cases

### 1. Local Development

Scan your repository during development:

```bash
# Quick scan
secretscout detect

# Full history scan
secretscout detect --log-opts "--all"

# Scan before committing
secretscout protect --staged
```

### 2. Pre-commit Hook

Prevent secrets from being committed:

#### Manual Hook

Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
secretscout protect --staged
exit $?
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

#### Pre-commit Framework

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: secretscout
        name: SecretScout Secret Detection
        entry: secretscout protect --staged
        language: system
        pass_filenames: false
        stages: [commit]
```

### 3. CI/CD Integration

#### GitLab CI

```yaml
# .gitlab-ci.yml
secret-scan:
  stage: test
  script:
    - secretscout detect --source $CI_PROJECT_DIR
  artifacts:
    reports:
      sast: results.sarif
```

#### Jenkins

```groovy
pipeline {
    agent any
    stages {
        stage('Secret Scan') {
            steps {
                sh 'secretscout detect --source .'
            }
        }
    }
}
```

#### CircleCI

```yaml
# .circleci/config.yml
version: 2.1
jobs:
  secret-scan:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run: cargo install secretscout
      - run: secretscout detect
```

### 4. Security Audits

Perform comprehensive security audits:

```bash
# Full repository scan
secretscout detect --log-opts "--all" --verbose

# Scan specific time period
secretscout detect --log-opts "--since='2024-01-01' --until='2024-12-31'"

# Generate detailed JSON report
secretscout detect --report-format json --report-path audit-$(date +%Y%m%d).json
```

### 5. Branch Comparison

Compare branches before merging:

```bash
# Compare feature branch to main
secretscout detect --log-opts "main..feature-branch"

# Compare staging to production
secretscout detect --log-opts "production..staging"

# Scan unmerged commits
secretscout detect --log-opts "origin/main..HEAD"
```

## Integration

### Git Aliases

Add to `.gitconfig`:

```ini
[alias]
    scan = !secretscout detect
    scan-staged = !secretscout protect --staged
    scan-all = !secretscout detect --log-opts --all
    scan-branch = "!f() { secretscout detect --log-opts \"main..$1\"; }; f"
```

Usage:
```bash
git scan
git scan-staged
git scan-all
git scan-branch feature-xyz
```

### Shell Aliases

Add to `.bashrc` or `.zshrc`:

```bash
alias ss='secretscout'
alias ssd='secretscout detect'
alias ssp='secretscout protect --staged'
alias ssv='secretscout detect --verbose'
```

### Docker

Run SecretScout in Docker:

```dockerfile
FROM rust:latest as builder
WORKDIR /build
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
COPY --from=builder /build/target/release/secretscout /usr/local/bin/
RUN apt-get update && apt-get install -y git ca-certificates && rm -rf /var/lib/apt/lists/*
ENTRYPOINT ["secretscout"]
CMD ["detect"]
```

Build and run:
```bash
docker build -t secretscout .
docker run -v $(pwd):/repo secretscout detect --source /repo
```

## Dual-Mode Architecture

SecretScout automatically detects whether to run in CLI or GitHub Actions mode:

### Mode Detection

```rust
if env::var("GITHUB_ACTIONS").is_ok()
    && env::var("GITHUB_WORKSPACE").is_ok()
    && env::var("GITHUB_EVENT_PATH").is_ok()
{
    // GitHub Actions mode
    // - Parse event context
    // - Post PR comments
    // - Generate job summaries
    // - Upload SARIF artifacts
} else {
    // CLI mode
    // - Parse command-line arguments
    // - Execute gitleaks commands
    // - Output to console
}
```

### CLI Mode (Default)

When run without GitHub Actions environment variables:

```bash
secretscout detect
# Automatically uses CLI mode
```

### GitHub Actions Mode

When run in GitHub Actions environment:

```yaml
- uses: globalbusinessadvisors/SecretScout@v3
# Automatically uses GitHub Actions mode
```

### Manual Mode Testing

Test GitHub Actions mode locally:

```bash
export GITHUB_ACTIONS=true
export GITHUB_WORKSPACE=$(pwd)
export GITHUB_EVENT_PATH=/path/to/event.json
export GITHUB_EVENT_NAME=push
export GITHUB_REPOSITORY=owner/repo
export GITHUB_REPOSITORY_OWNER=owner
export GITHUB_TOKEN=ghp_xxxxx

./secretscout
# Runs in GitHub Actions mode
```

## Troubleshooting

### Gitleaks Binary Not Found

SecretScout automatically downloads gitleaks on first run:

```bash
$ secretscout detect
INFO: Downloading gitleaks v8.24.3 for linux/x64
INFO: Extracted gitleaks binary to: ~/.cache/secretscout/gitleaks/...
```

Subsequent runs use the cached binary.

### Permission Denied

Ensure git repository is accessible:

```bash
# Check repository access
cd /path/to/repo
git status

# Run with verbose output
secretscout detect --verbose
```

### Custom Config Not Found

Verify config file path:

```bash
# Check if config exists
ls -la .gitleaks.toml

# Use absolute path
secretscout detect --config /absolute/path/to/config.toml
```

### No Secrets Detected but Expected

Enable verbose logging:

```bash
secretscout detect --verbose
```

Check if files are in allowlist:

```toml
# .gitleaks.toml
[allowlist]
paths = [
  "test/",  # These files won't be scanned
  "vendor/",
]
```

## Advanced Usage

### Custom Gitleaks Version

SecretScout uses gitleaks 8.24.3 by default. To use a different version, modify the code or use environment variable (future enhancement).

### Multiple Repositories

Scan multiple repositories:

```bash
#!/bin/bash
for repo in repo1 repo2 repo3; do
    echo "Scanning $repo..."
    secretscout detect --source "$repo" --report-path "$repo-findings.sarif"
done
```

### Report Aggregation

Combine multiple SARIF reports:

```bash
# Scan multiple projects
secretscout detect --source project1 --report-path project1.sarif
secretscout detect --source project2 --report-path project2.sarif

# Process with sarif-tools or custom script
```

### Continuous Monitoring

Set up continuous monitoring:

```bash
# crontab -e
0 0 * * * secretscout detect --source /path/to/repo --report-path /var/log/secretscout-$(date +\%Y\%m\%d).sarif
```

## Best Practices

1. **Always use config files**: Customize detection rules for your needs
2. **Enable pre-commit hooks**: Prevent secrets from being committed
3. **Regular full scans**: Run `--log-opts "--all"` periodically
4. **Store reports**: Keep SARIF reports for compliance
5. **Verbose logging**: Use `--verbose` when debugging
6. **Update regularly**: Keep SecretScout updated for latest rules
7. **Review findings**: Not all detections are real secrets
8. **Use allowlists**: Exclude test files and false positives

## Support

- **Documentation**: [Full Docs](../README.md)
- **Issues**: [GitHub Issues](https://github.com/globalbusinessadvisors/SecretScout/issues)
- **Discussions**: [GitHub Discussions](https://github.com/globalbusinessadvisors/SecretScout/discussions)

---

**SecretScout v3.0.0 - Fast, Safe, Reliable Secret Detection**
