# GitHub Actions Guide

SecretScout can run as a GitHub Action to automatically scan your repositories for secrets in CI/CD pipelines.

## Quick Start

Add this to `.github/workflows/secretscout.yml`:

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

## Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `config` | Path to gitleaks config file | No | Auto-detected |
| `version` | Gitleaks version to use | No | `8.24.3` |
| `license` | Gitleaks Pro license key | No | - |
| `enable-summary` | Enable job summary | No | `true` |
| `enable-upload-artifact` | Enable SARIF upload | No | `true` |
| `enable-comments` | Enable PR comments | No | `true` |
| `notify-user-list` | Users to notify (comma-separated) | No | - |
| `base-ref` | Base ref override | No | Auto-detected |

## Advanced Configuration

### Full Configuration Example

```yaml
name: Secret Scan
on:
  push:
    branches: [main, develop]
  pull_request:
  schedule:
    - cron: '0 0 * * 0'  # Weekly full scan

jobs:
  scan:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
      actions: write

    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history for complete scanning

      - name: Run SecretScout
        uses: globalbusinessadvisors/SecretScout@v3
        with:
          config: .github/.gitleaks.toml
          version: 8.24.3
          enable-summary: true
          enable-upload-artifact: true
          enable-comments: true
          notify-user-list: "@security-team, @devops"
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          GITLEAKS_LICENSE: ${{ secrets.GITLEAKS_LICENSE }}
```

## Event Support

### Push Events

```yaml
on: push
```

Scans commits between `base` and `head` refs.

### Pull Request Events

```yaml
on: pull_request
```

Scans PR commits and posts inline review comments.

### Workflow Dispatch

```yaml
on: workflow_dispatch
```

Performs a full repository scan.

### Scheduled Scans

```yaml
on:
  schedule:
    - cron: '0 0 * * 0'  # Every Sunday at midnight
```

Weekly full repository scan.

## Outputs

### 1. Job Summary

Rich HTML summary with:
- Total findings count
- Findings grouped by rule
- File paths and line numbers
- Commit information
- Direct links to findings

### 2. PR Comments

Inline review comments showing:
- Secret type and rule ID
- Exact location (file, line)
- Commit SHA and author
- Fingerprint for `.gitleaksignore`
- User notifications (if configured)

### 3. SARIF Reports

Standards-compliant SARIF 2.1.0 reports:
- Uploaded as workflow artifacts
- Compatible with GitHub Code Scanning
- Machine-readable format for automation

### 4. Exit Codes

- `0`: No secrets found (success)
- `1`: Secrets detected (failure)
- `2+`: Other errors

## Environment Variables

All v2 environment variables are supported:

```yaml
env:
  # Required for PR comments
  GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  # Optional: Gitleaks Pro license
  GITLEAKS_LICENSE: ${{ secrets.GITLEAKS_LICENSE }}

  # Optional: Feature toggles
  GITLEAKS_ENABLE_SUMMARY: true
  GITLEAKS_ENABLE_UPLOAD_ARTIFACT: true
  GITLEAKS_ENABLE_COMMENTS: true
  GITLEAKS_NOTIFY_USER_LIST: "@user1, @user2"

  # Optional: Version and config
  GITLEAKS_VERSION: 8.24.3
  GITLEAKS_CONFIG: .gitleaks.toml

  # Optional: Base ref override
  BASE_REF: main
```

## Permissions

Ensure your workflow has the correct permissions:

```yaml
permissions:
  contents: read          # Read repository contents
  pull-requests: write    # Post PR comments
  actions: write          # Upload artifacts
```

## Troubleshooting

### Binary Not Found

If you see "Binary not found", SecretScout will automatically build from source. First run may take 5-10 minutes.

**Solution**: Be patient. Subsequent runs use the cached binary.

### Permission Errors

Ensure your workflow has correct permissions:

```yaml
permissions:
  contents: read
  pull-requests: write  # For PR comments
  actions: write        # For artifacts
```

### Rate Limiting

If you hit GitHub API rate limits, SecretScout automatically retries with exponential backoff.

**Solution**: Wait or use a token with higher rate limits.

### Custom Config Not Found

SecretScout auto-detects configs in these locations:
1. Path specified in `config` input
2. `.gitleaks.toml` in repository root
3. `.github/.gitleaks.toml`
4. Gitleaks default config

## Migration from v2

SecretScout v3 is 100% backward compatible with v2. Simply update your workflow:

```yaml
# Before (v2)
- uses: gitleaks/gitleaks-action@v2

# After (v3)
- uses: globalbusinessadvisors/SecretScout@v3
```

No configuration changes required! See [MIGRATION.md](MIGRATION.md) for detailed migration guide.

## Examples

### Basic Scan

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

### With Custom Config

```yaml
name: Secret Scan
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: globalbusinessadvisors/SecretScout@v3
        with:
          config: .github/.gitleaks.toml
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### With Notifications

```yaml
name: Secret Scan
on: [push, pull_request]

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: globalbusinessadvisors/SecretScout@v3
        with:
          notify-user-list: "@security-team, @devops"
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### Scheduled Full Scan

```yaml
name: Weekly Secret Scan
on:
  schedule:
    - cron: '0 0 * * 0'  # Every Sunday at midnight

jobs:
  scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0  # Full history
      - uses: globalbusinessadvisors/SecretScout@v3
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

## See Also

- [CLI Usage Guide](CLI_USAGE.md) - Comprehensive CLI guide
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [MIGRATION.md](MIGRATION.md) - Migration from v2
- [Architecture](ARCHITECTURE.md) - Technical architecture
