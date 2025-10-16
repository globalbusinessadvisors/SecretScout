# SecretScout CLI Quick Reference

## Installation

```bash
cargo install --path secretscout
```

## Commands

### detect - Scan for secrets

```bash
# Basic
secretscout detect

# With options
secretscout detect \
  --source /path/to/repo \
  --config .gitleaks.toml \
  --report-format json \
  --report-path findings.json \
  --verbose
```

### protect - Scan staged changes

```bash
# Basic
secretscout protect --staged

# With options
secretscout protect \
  --source /path/to/repo \
  --config .gitleaks.toml \
  --verbose
```

### version - Show version

```bash
secretscout version
```

## Common Options

| Option | Short | Description |
|--------|-------|-------------|
| `--source <PATH>` | `-s` | Repository path |
| `--config <PATH>` | `-c` | Config file |
| `--verbose` | `-v` | Verbose output |
| `--help` | `-h` | Show help |

## detect Options

| Option | Default | Description |
|--------|---------|-------------|
| `--report-path` | `results.sarif` | Output file |
| `--report-format` | `sarif` | Format (sarif/json/csv/text) |
| `--redact` | `true` | Redact secrets |
| `--exit-code` | `2` | Exit code on leak |
| `--log-opts` | - | Git log options |

## Exit Codes

### detect
- `0` = No secrets found
- `2` = Secrets detected (default)
- Custom = Set with `--exit-code`

### protect
- `0` = No secrets in staged changes
- `1` = Secrets found

## Common Use Cases

### Local Development

```bash
# Quick scan
secretscout detect

# Full history
secretscout detect --log-opts "--all"

# Before commit
secretscout protect --staged
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit
secretscout protect --staged
exit $?
```

### CI/CD

```bash
# Scan in CI
secretscout detect --source $CI_WORKSPACE
```

### Branch Comparison

```bash
# Compare branches
secretscout detect --log-opts "main..feature"

# Recent commits
secretscout detect --log-opts "HEAD~10..HEAD"
```

## Configuration

### Auto-detection Order

1. `--config` flag
2. `gitleaks.toml`
3. `.gitleaks.toml`
4. Default config

### Sample Config

```toml
# .gitleaks.toml
title = "Custom Config"

[[rules]]
id = "aws-key"
description = "AWS Access Key"
regex = '''AKIA[0-9A-Z]{16}'''

[allowlist]
paths = ["test/", "vendor/"]
```

## Git Aliases

```bash
# Add to .gitconfig
[alias]
    scan = !secretscout detect
    scan-staged = !secretscout protect --staged
    scan-all = !secretscout detect --log-opts --all
```

## Shell Aliases

```bash
# Add to .bashrc/.zshrc
alias ss='secretscout'
alias ssd='secretscout detect'
alias ssp='secretscout protect --staged'
```

## Troubleshooting

### Binary downloads automatically
First run takes 5-10s to download gitleaks

### Permission denied
```bash
chmod +x /path/to/secretscout
```

### Config not found
```bash
ls -la .gitleaks.toml
secretscout detect --config /absolute/path/config.toml
```

### Enable debug output
```bash
secretscout detect --verbose
```

## Examples

### Basic Workflow

```bash
# 1. Scan repository
secretscout detect

# 2. Review findings
cat results.sarif

# 3. Add to allowlist if needed
echo "path/to/false-positive" >> .gitleaksignore

# 4. Rescan
secretscout detect
```

### Security Audit

```bash
# Full history scan with JSON output
secretscout detect \
  --log-opts "--all" \
  --report-format json \
  --report-path audit-$(date +%Y%m%d).json \
  --verbose
```

### Development Workflow

```bash
# Before starting work
git pull
secretscout detect

# During development
# (make changes)
git add .

# Before committing
secretscout protect --staged

# If clean, commit
git commit -m "Add feature"
```

## GitHub Actions vs CLI

### GitHub Actions Mode
```yaml
- uses: globalbusinessadvisors/SecretScout@v3
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### CLI Mode
```bash
secretscout detect --source .
```

### Auto-detection
SecretScout automatically detects mode based on environment variables:
- `GITHUB_ACTIONS=true` + `GITHUB_WORKSPACE` + `GITHUB_EVENT_PATH` = GitHub Actions mode
- Otherwise = CLI mode

## Report Formats

### SARIF (default)
```bash
secretscout detect --report-format sarif
```

### JSON
```bash
secretscout detect --report-format json
```

### CSV
```bash
secretscout detect --report-format csv
```

### Text
```bash
secretscout detect --report-format text
```

## Advanced Usage

### Multiple Repos
```bash
for repo in repo1 repo2 repo3; do
  secretscout detect --source "$repo" --report-path "$repo.sarif"
done
```

### Scheduled Scans
```bash
# crontab
0 0 * * * secretscout detect --source /path --report-path /var/log/scan-$(date +\%Y\%m\%d).sarif
```

### Custom Exit Codes
```bash
# Exit 1 instead of 2 on leak
secretscout detect --exit-code 1

# Or catch exit code
secretscout detect || echo "Secrets found: $?"
```

## Getting Help

```bash
# General help
secretscout --help

# Command help
secretscout detect --help
secretscout protect --help

# Version info
secretscout version
```

## Links

- [Full Documentation](../README.md)
- [CLI Usage Guide](CLI_USAGE.md)
- [Transformation Details](CLI_TRANSFORMATION.md)
- [GitHub Issues](https://github.com/globalbusinessadvisors/SecretScout/issues)

---

**SecretScout v3.0.0 - Fast, Safe, Reliable Secret Detection**
