# Publishing SecretScout

This guide is for maintainers who need to publish new versions of SecretScout to npm and crates.io.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Release Process](#release-process)
- [Manual Publishing](#manual-publishing)
- [Troubleshooting](#troubleshooting)
- [Rollback Procedures](#rollback-procedures)

---

## Prerequisites

### Required Tools

1. **Rust toolchain** (1.90+)
   ```bash
   rustup update
   ```

2. **cargo-dist** (for automated releases)
   ```bash
   cargo install cargo-dist --locked
   ```

3. **Node.js and npm** (16+)
   ```bash
   node --version
   npm --version
   ```

### Required Credentials

1. **npm Access Token**
   - Login to npmjs.com
   - Account Settings → Access Tokens → Generate New Token
   - Select "Automation" type
   - Copy token and add to GitHub Secrets as `NPM_TOKEN`

2. **crates.io API Token**
   - Login to crates.io with your GitHub account
   - Account Settings → API Tokens → New Token
   - Name it "SecretScout Releases"
   - Copy token and add to GitHub Secrets as `CARGO_TOKEN`

3. **GitHub Repository Access**
   - Write access to the repository
   - Ability to create tags and releases

### GitHub Secrets Setup

Navigate to: Repository → Settings → Secrets and variables → Actions

Add the following secrets:
- `NPM_TOKEN` - Your npm automation token
- `CARGO_TOKEN` - Your crates.io API token

The `GITHUB_TOKEN` is automatically provided by GitHub Actions.

---

## Release Process

SecretScout uses automated releases via cargo-dist. When you push a version tag, GitHub Actions automatically:
1. Builds binaries for all platforms
2. Runs tests
3. Creates a GitHub release
4. Publishes npm packages
5. Publishes to crates.io

### Step 1: Pre-release Checklist

Before starting a release, verify:

- [ ] All tests pass locally: `cargo test --all-features`
- [ ] Clippy has no warnings: `cargo clippy --all-features -- -D warnings`
- [ ] Code is formatted: `cargo fmt --all -- --check`
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] All features work as expected
- [ ] CHANGELOG.md is up to date
- [ ] No security vulnerabilities: `cargo audit` (install with `cargo install cargo-audit`)

### Step 2: Version Bump

Update version numbers in multiple files to keep them in sync:

```bash
# 1. Update workspace version in Cargo.toml
vim Cargo.toml
# Change: version = "3.1.0" to version = "3.2.0"

# 2. Update package.json version
npm version 3.2.0 --no-git-tag-version

# 3. Update all platform package versions
vim npm/linux-x64/package.json      # "version": "3.2.0"
vim npm/darwin-x64/package.json     # "version": "3.2.0"
vim npm/darwin-arm64/package.json   # "version": "3.2.0"
vim npm/win32-x64/package.json      # "version": "3.2.0"

# 4. Update package.json optionalDependencies versions
vim package.json
# Update all @secretscout/* dependencies to "3.2.0"

# 5. Commit version changes
git add Cargo.toml package.json package-lock.json npm/*/package.json
git commit -m "chore: bump version to v3.2.0"
```

### Step 3: Update CHANGELOG

Update `CHANGELOG.md` with release notes:

```markdown
## [Unreleased]

## [3.2.0] - 2025-11-15

### Added
- New feature X
- New command Y

### Changed
- Improved performance of Z

### Fixed
- Bug in component A
```

Commit the changes:
```bash
git add CHANGELOG.md
git commit -m "docs: update changelog for v3.2.0"
```

### Step 4: Create and Push Tag

```bash
# Create a new tag
git tag v3.2.0

# Push commits and tag
git push origin main
git push origin v3.2.0
```

### Step 5: Monitor GitHub Actions

1. Go to: https://github.com/globalbusinessadvisors/SecretScout/actions
2. Watch the "Release" workflow execution
3. Verify all jobs complete successfully:
   - Build jobs for each platform (linux, macos-intel, macos-arm, windows)
   - Test jobs
   - GitHub release creation
   - npm publish
   - crates.io publish

### Step 6: Verify Deployment

After the workflow completes, verify the release:

```bash
# Check npm
npm view secretscout version
npm view @secretscout/linux-x64 version
npm view @secretscout/darwin-x64 version
npm view @secretscout/darwin-arm64 version
npm view @secretscout/win32-x64 version

# Check crates.io
cargo search secretscout --limit 1

# Test installation from npm
npm install -g secretscout@latest
secretscout --version

# Test installation from crates.io
cargo install secretscout --force
secretscout --version
```

### Step 7: Post-Release Tasks

- [ ] Verify GitHub release page looks correct
- [ ] Test the published npm package on different platforms
- [ ] Test the published cargo crate
- [ ] Announce release on GitHub Discussions (optional)
- [ ] Update project documentation if needed
- [ ] Close any related GitHub issues/milestones

---

## Manual Publishing

If automated publishing fails or you need to publish manually:

### Publishing to npm Manually

```bash
# 1. Build binaries for all platforms (requires cross-compilation setup)
# See: https://rust-lang.github.io/rustup/cross-compilation.html

# For Linux x64
cargo build --release --target x86_64-unknown-linux-gnu
cp target/x86_64-unknown-linux-gnu/release/secretscout npm/linux-x64/

# For macOS x64
cargo build --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/secretscout npm/darwin-x64/

# For macOS ARM64
cargo build --release --target aarch64-apple-darwin
cp target/aarch64-apple-darwin/release/secretscout npm/darwin-arm64/

# For Windows x64
cargo build --release --target x86_64-pc-windows-msvc
cp target/x86_64-pc-windows-msvc/release/secretscout.exe npm/win32-x64/

# 2. Publish each platform package
cd npm/linux-x64 && npm publish --access public && cd ../..
cd npm/darwin-x64 && npm publish --access public && cd ../..
cd npm/darwin-arm64 && npm publish --access public && cd ../..
cd npm/win32-x64 && npm publish --access public && cd ../..

# 3. Publish main package
npm publish
```

### Publishing to crates.io Manually

```bash
# 1. Login (one-time setup)
cargo login

# 2. Dry run to check for issues
cd secretscout
cargo publish --dry-run

# 3. Review package contents
cargo package --list

# 4. Publish
cargo publish

# 5. Verify (may take 1-2 minutes to appear)
cargo search secretscout --limit 1
```

---

## Troubleshooting

### Build Failures

**Problem**: Compilation fails on a specific platform

**Solution**:
1. Check the GitHub Actions logs for the specific error
2. Try building locally for that target
3. Ensure all dependencies are available for that platform
4. Check for platform-specific code issues

### npm Publish Fails

**Problem**: npm publish returns authentication error

**Solution**:
```bash
# Check if NPM_TOKEN is valid
npm whoami

# Regenerate token if needed
# Update GitHub secret with new token
```

**Problem**: Version already exists

**Solution**:
- You cannot republish the same version
- Bump version number and create a new tag
- Use `npm deprecate` if the published version is broken

### crates.io Publish Fails

**Problem**: Cargo publish fails with authentication error

**Solution**:
```bash
# Re-login to crates.io
cargo login

# Check token in GitHub secrets
# Regenerate if expired
```

**Problem**: Package is too large

**Solution**:
```bash
# Check what's being included
cargo package --list

# Add exclusions to secretscout/Cargo.toml
exclude = [
    "tests/fixtures/*",
    "*.log",
    "target/*"
]
```

### cargo-dist Issues

**Problem**: cargo-dist fails to build

**Solution**:
```bash
# Update cargo-dist
cargo install cargo-dist --locked --force

# Re-initialize if configuration is outdated
cargo dist init --yes

# Generate updated CI workflows
cargo dist generate-ci github
```

### Missing Binaries

**Problem**: Binary not found after installation

**Solution**:
- Check that platform-specific package was published
- Verify binary is executable: `chmod +x`
- Check npm installation logs for errors
- Try `npm install --force` to reinstall optional dependencies

---

## Rollback Procedures

If a release has critical bugs, follow these steps:

### 1. Yank the npm Version

```bash
# Option 1: Unpublish (only within 72 hours)
npm unpublish secretscout@3.2.0

# Option 2: Deprecate (if widely used)
npm deprecate secretscout@3.2.0 "Critical bug, please use 3.1.0 instead"

# Also deprecate platform packages
npm deprecate @secretscout/linux-x64@3.2.0 "Critical bug, use 3.1.0"
npm deprecate @secretscout/darwin-x64@3.2.0 "Critical bug, use 3.1.0"
npm deprecate @secretscout/darwin-arm64@3.2.0 "Critical bug, use 3.1.0"
npm deprecate @secretscout/win32-x64@3.2.0 "Critical bug, use 3.1.0"
```

### 2. Yank the crates.io Version

```bash
cargo yank --vers 3.2.0 secretscout
```

**Note**: Yanking on crates.io doesn't delete the package, it just prevents new projects from using it. Existing projects with that version in Cargo.lock can still build.

### 3. Delete GitHub Release

1. Go to: https://github.com/globalbusinessadvisors/SecretScout/releases
2. Find the problematic release
3. Click "Delete release"
4. Delete the git tag:
   ```bash
   git tag -d v3.2.0
   git push origin :refs/tags/v3.2.0
   ```

### 4. Create Hotfix Release

```bash
# Create branch from last good version
git checkout v3.1.0
git checkout -b hotfix/3.2.1

# Apply minimal fix
# ... make changes ...

# Commit and tag
git add .
git commit -m "fix: critical bug in X"

# Update version to 3.2.1
# Follow normal release process
```

---

## Version Strategy

SecretScout follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes, incompatible API changes
- **MINOR** (x.Y.0): New features, backward compatible
- **PATCH** (x.y.Z): Bug fixes, backward compatible

### When to Bump Versions

- **Patch**: Bug fixes, security patches, documentation updates
- **Minor**: New features, new CLI commands, new output formats
- **Major**: Breaking changes, CLI interface changes, removed features

### Pre-release Versions

For testing releases before official publication:

```bash
# Alpha
git tag v3.2.0-alpha.1

# Beta
git tag v3.2.0-beta.1

# Release Candidate
git tag v3.2.0-rc.1
```

Users can install pre-releases:
```bash
npm install -g secretscout@3.2.0-beta.1
cargo install secretscout --version 3.2.0-beta.1
```

---

## Release Checklist Template

Copy this checklist for each release:

```markdown
## Release vX.Y.Z Checklist

### Pre-release
- [ ] All tests pass
- [ ] Clippy passes with no warnings
- [ ] Code is formatted
- [ ] Documentation builds
- [ ] Security audit passes
- [ ] CHANGELOG updated

### Version Bump
- [ ] Updated Cargo.toml
- [ ] Updated package.json
- [ ] Updated npm/*/package.json
- [ ] Updated package.json optionalDependencies
- [ ] Committed version changes

### Tagging
- [ ] Created git tag vX.Y.Z
- [ ] Pushed tag to origin

### Verification
- [ ] GitHub Actions workflow succeeded
- [ ] GitHub release created
- [ ] npm packages published
- [ ] crates.io published
- [ ] Tested npm installation
- [ ] Tested cargo installation

### Post-release
- [ ] Announced release
- [ ] Closed related issues
- [ ] Updated documentation
```

---

## Useful Commands Reference

```bash
# Check current version
cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version'

# List all git tags
git tag -l

# Check what will be included in crate
cargo package --list

# Check what will be included in npm package
npm pack --dry-run

# View published npm package info
npm view secretscout

# View published crate info
cargo info secretscout

# Test binary from release
curl -L https://github.com/globalbusinessadvisors/SecretScout/releases/download/v3.1.0/secretscout-x86_64-unknown-linux-gnu.tar.gz | tar xz

# Generate cargo-dist CI
cargo dist generate-ci github

# Plan a release (dry-run)
cargo dist plan

# Build release artifacts locally
cargo dist build
```

---

## Support

If you encounter issues during the release process:

1. Check the [GitHub Actions logs](https://github.com/globalbusinessadvisors/SecretScout/actions)
2. Review this documentation
3. Check [cargo-dist documentation](https://opensource.axo.dev/cargo-dist/)
4. Open an issue on GitHub

---

**Last Updated**: 2025-11-01
**Maintained By**: SecretScout Contributors
