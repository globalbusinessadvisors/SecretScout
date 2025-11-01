# SecretScout Publishing Integration Plan

**Objective**: Integrate both npm package distribution and crates.io publishing for SecretScout

**Date Created**: 2025-11-01
**Status**: Planning Phase

---

## Overview

This plan outlines the implementation of dual package distribution:
1. **npm** - For easy global installation via Node.js package manager
2. **crates.io** - For Rust developers using cargo

---

## Architecture Decision

### npm Strategy: cargo-dist with Platform-Specific Packages
- Use cargo-dist to automate multi-platform binary builds
- Publish platform-specific optional dependency packages
- Single workflow for builds, releases, and publishing

### Crates.io Strategy: Workspace Publishing
- Publish `secretscout` crate as both library and binary
- Maintain workspace structure for future expansion
- Automated publishing via GitHub Actions

---

## Phase 1: Project Restructuring

### 1.1 Update Cargo Workspace Configuration

**File**: `Cargo.toml` (root)

**Tasks**:
- Add cargo-dist configuration section
- Define supported platforms for distribution
- Configure installer types (npm, shell, PowerShell)
- Set GitHub release integration

**Changes needed**:
```toml
[workspace.metadata.dist]
# The preferred cargo-dist version to use in CI (Cargo.toml SemVer syntax)
cargo-dist-version = "0.13.0"
# CI backends to support
ci = ["github"]
# The installers to generate for each app
installers = ["npm", "shell", "powershell"]
# Target platforms to build apps for (Rust target-triple syntax)
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "x86_64-pc-windows-msvc"
]
# Publish jobs to run in CI
pr-run-mode = "plan"
# Whether to install an updater program
install-updater = false
```

**Validation**:
- [ ] Run `cargo dist init` to verify configuration
- [ ] Check generated `.github/workflows/release.yml`

---

### 1.2 Prepare secretscout Crate for Publishing

**File**: `secretscout/Cargo.toml`

**Tasks**:
- Add comprehensive metadata for crates.io
- Add categories and keywords for discoverability
- Ensure all dependencies specify versions correctly
- Add exclude patterns for unnecessary files

**Changes needed**:
```toml
[package]
name = "secretscout"
version.workspace = true
edition.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
homepage.workspace = true
description = "Blazingly fast Rust CLI for detecting secrets, passwords, API keys, and tokens in git repositories"
readme = "../README.md"
keywords = ["security", "secrets", "git", "cli", "scanner"]
categories = ["command-line-utilities", "development-tools"]
exclude = [
    "tests/fixtures/*",
    ".github/*",
    "*.log"
]
```

**Validation**:
- [ ] Run `cargo publish --dry-run` to test
- [ ] Verify package size is reasonable (<10 MB)
- [ ] Check that README renders correctly on crates.io preview

---

### 1.3 Add Crate Documentation

**Files**:
- `secretscout/src/lib.rs`
- `secretscout/src/main.rs`

**Tasks**:
- Add crate-level documentation comments
- Document public API (if exposing library functions)
- Add usage examples
- Document feature flags

**Required additions**:
```rust
// In lib.rs
//! # SecretScout
//!
//! A blazingly fast, memory-safe CLI tool for detecting secrets in git repositories.
//!
//! ## Features
//!
//! - High-performance secret scanning using Rust
//! - Multiple output formats (SARIF, JSON, CSV, text)
//! - Pre-commit hook support
//! - Configurable detection rules
//!
//! ## Usage as Library
//!
//! ```no_run
//! use secretscout::{Scanner, Config};
//!
//! let config = Config::default();
//! let scanner = Scanner::new(config);
//! // ... scanning logic
//! ```
```

**Validation**:
- [ ] Run `cargo doc --open` to preview documentation
- [ ] Ensure all public APIs are documented
- [ ] Check for broken doc links

---

## Phase 2: npm Package Setup

### 2.1 Install and Initialize cargo-dist

**Commands**:
```bash
# Install cargo-dist
cargo install cargo-dist

# Initialize in project
cargo dist init

# Generate configuration and CI workflows
cargo dist generate
```

**Expected outputs**:
- `.github/workflows/release.yml` - Automated release workflow
- Updated `Cargo.toml` with dist metadata
- npm package structure in project

**Tasks**:
- [ ] Install cargo-dist globally
- [ ] Run initialization in project root
- [ ] Review generated GitHub Actions workflow
- [ ] Commit generated files

---

### 2.2 Configure npm Package Metadata

**File**: `package.json` (root)

**Tasks**:
- Update with comprehensive npm metadata
- Configure platform-specific optional dependencies
- Set up binary wrapper configuration
- Add publishing scripts

**Complete package.json structure**:
```json
{
  "name": "secretscout",
  "version": "3.1.0",
  "description": "Rust-powered secret detection for GitHub Actions - Fast, safe, and efficient CLI tool",
  "main": "index.js",
  "bin": {
    "secretscout": "./cli.js"
  },
  "scripts": {
    "test": "cargo test --all-features",
    "build": "cargo build --release --features native",
    "prepare": "cargo build --release --features native",
    "postinstall": "node scripts/postinstall.js"
  },
  "optionalDependencies": {
    "@secretscout/linux-x64": "3.1.0",
    "@secretscout/darwin-x64": "3.1.0",
    "@secretscout/darwin-arm64": "3.1.0",
    "@secretscout/win32-x64": "3.1.0"
  },
  "repository": {
    "type": "git",
    "url": "https://github.com/globalbusinessadvisors/SecretScout.git"
  },
  "homepage": "https://github.com/globalbusinessadvisors/SecretScout#readme",
  "bugs": {
    "url": "https://github.com/globalbusinessadvisors/SecretScout/issues"
  },
  "keywords": [
    "secretscout",
    "secrets",
    "security",
    "github-actions",
    "rust",
    "secret-scanning",
    "secret-detection",
    "cli",
    "git",
    "gitleaks"
  ],
  "author": "Global Business Advisors",
  "license": "MIT",
  "engines": {
    "node": ">=16.0.0"
  },
  "files": [
    "cli.js",
    "index.js",
    "scripts/",
    "README.md",
    "LICENSE"
  ]
}
```

**Validation**:
- [ ] Run `npm pack` to preview package contents
- [ ] Verify package size
- [ ] Check npm metadata with `npm view`

---

### 2.3 Create npm Wrapper Scripts

**File**: `cli.js` (root)

**Purpose**: Main entry point that finds and executes the correct platform binary

**Implementation**:
```javascript
#!/usr/bin/env node

const { spawn } = require('child_process');
const { join } = require('path');
const { platform, arch } = process;

// Map Node.js platform/arch to Rust target triples
function getPlatformBinary() {
  const platformMap = {
    'linux-x64': '@secretscout/linux-x64',
    'darwin-x64': '@secretscout/darwin-x64',
    'darwin-arm64': '@secretscout/darwin-arm64',
    'win32-x64': '@secretscout/win32-x64'
  };

  const key = `${platform}-${arch}`;
  const packageName = platformMap[key];

  if (!packageName) {
    console.error(`Unsupported platform: ${platform}-${arch}`);
    process.exit(1);
  }

  try {
    const binaryPath = require.resolve(`${packageName}/secretscout${platform === 'win32' ? '.exe' : ''}`);
    return binaryPath;
  } catch (err) {
    console.error(`Failed to find platform binary for ${key}`);
    console.error('Try running: npm install --force');
    process.exit(1);
  }
}

// Execute the binary with all arguments
const binary = getPlatformBinary();
const child = spawn(binary, process.argv.slice(2), { stdio: 'inherit' });

child.on('exit', (code) => {
  process.exit(code || 0);
});
```

**Tasks**:
- [ ] Create `cli.js` wrapper script
- [ ] Make executable: `chmod +x cli.js`
- [ ] Test with: `./cli.js --version`

---

**File**: `scripts/postinstall.js`

**Purpose**: Fallback download mechanism for unsupported platforms

**Implementation**:
```javascript
#!/usr/bin/env node

const https = require('https');
const { platform, arch } = process;
const fs = require('fs');
const path = require('path');

// Only download if no platform-specific package was installed
const platformPackageExists = () => {
  const platformMap = {
    'linux-x64': '@secretscout/linux-x64',
    'darwin-x64': '@secretscout/darwin-x64',
    'darwin-arm64': '@secretscout/darwin-arm64',
    'win32-x64': '@secretscout/win32-x64'
  };

  const key = `${platform}-${arch}`;
  const packageName = platformMap[key];

  if (!packageName) return false;

  try {
    require.resolve(packageName);
    return true;
  } catch {
    return false;
  }
};

// If platform package exists, we're done
if (platformPackageExists()) {
  console.log('✓ Platform-specific binary installed');
  process.exit(0);
}

// Otherwise, attempt download from GitHub releases
console.log('Platform package not found, downloading from GitHub releases...');
// ... download logic here (for unsupported platforms)
```

**Tasks**:
- [ ] Create postinstall script
- [ ] Add GitHub release download logic
- [ ] Test on supported platforms
- [ ] Test fallback mechanism

---

### 2.4 Create Platform-Specific Package Templates

**Directory structure**:
```
npm/
├── linux-x64/
│   └── package.json
├── darwin-x64/
│   └── package.json
├── darwin-arm64/
│   └── package.json
└── win32-x64/
    └── package.json
```

**File**: `npm/linux-x64/package.json` (example)

```json
{
  "name": "@secretscout/linux-x64",
  "version": "3.1.0",
  "description": "SecretScout binary for Linux x64",
  "os": ["linux"],
  "cpu": ["x64"],
  "main": "index.js",
  "license": "MIT",
  "repository": {
    "type": "git",
    "url": "https://github.com/globalbusinessadvisors/SecretScout.git"
  },
  "files": [
    "secretscout"
  ]
}
```

**Note**: cargo-dist may generate these automatically. Verify after running `cargo dist init`.

**Tasks**:
- [ ] Create platform package directories
- [ ] Add package.json for each platform
- [ ] Configure OS and CPU constraints
- [ ] Set up file inclusions

---

## Phase 3: GitHub Actions Automation

### 3.1 Configure Release Workflow

**File**: `.github/workflows/release.yml`

**Purpose**: Automated building, testing, and publishing on version tags

**Key components**:
1. Trigger on version tags (v*.*.*)
2. Build binaries for all platforms
3. Run tests on each platform
4. Create GitHub release
5. Publish to npm
6. Publish to crates.io

**Generated by cargo-dist**, but customize as needed:

```yaml
name: Release

on:
  push:
    tags:
      - 'v[0-9]+.[0-9]+.[0-9]+'

jobs:
  # cargo-dist generates these jobs:
  # - build-*: Build binaries for each platform
  # - upload-artifacts: Upload to GitHub releases
  # - publish-npm: Publish npm packages
  # - publish-crates: Publish to crates.io
```

**Tasks**:
- [ ] Review generated workflow
- [ ] Add custom test jobs if needed
- [ ] Configure npm publish with NPM_TOKEN secret
- [ ] Configure crates.io publish with CARGO_TOKEN secret

---

### 3.2 Set Up GitHub Secrets

**Required secrets**:

1. **NPM_TOKEN**
   - Purpose: Publish to npm registry
   - How to get: `npm login` → Account settings → Access tokens → Generate new token (Automation)
   - Scope: Read and write

2. **CARGO_TOKEN**
   - Purpose: Publish to crates.io
   - How to get: crates.io → Account settings → API tokens → New token
   - Scope: Publish updates

**Steps**:
- [ ] Generate npm access token
- [ ] Add NPM_TOKEN to GitHub repository secrets
- [ ] Generate crates.io API token
- [ ] Add CARGO_TOKEN to GitHub repository secrets
- [ ] Verify GITHUB_TOKEN has sufficient permissions (auto-provided)

**Location**: Repository Settings → Secrets and variables → Actions → New repository secret

---

### 3.3 Add Pre-release Testing Workflow

**File**: `.github/workflows/test-release.yml`

**Purpose**: Test the release process without actually publishing

```yaml
name: Test Release

on:
  pull_request:
    branches: [main]
  push:
    branches: [main]

jobs:
  test-build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Build
        run: cargo build --release --features native
      - name: Test
        run: cargo test --all-features
      - name: Package test
        run: cargo dist build --artifacts all
```

**Tasks**:
- [ ] Create test workflow
- [ ] Ensure it runs on PRs
- [ ] Verify builds succeed on all platforms

---

## Phase 4: crates.io Publishing

### 4.1 Verify Crate Readiness

**Checklist**:
- [ ] All metadata complete in Cargo.toml
- [ ] README.md exists and is informative
- [ ] LICENSE file exists (MIT)
- [ ] Documentation is complete
- [ ] All dependencies have versions
- [ ] No git dependencies (only crates.io or path)
- [ ] Tests pass: `cargo test --all-features`
- [ ] Clippy passes: `cargo clippy --all-features`
- [ ] Dry-run succeeds: `cargo publish --dry-run`

---

### 4.2 Initial Manual Publish

**First time publishing to crates.io**:

```bash
# Login to crates.io (one time)
cargo login

# Verify package contents
cargo package --list

# Dry run to check for issues
cargo publish --dry-run

# Publish to crates.io
cd secretscout
cargo publish

# Wait for indexing (usually 1-2 minutes)
# Verify at: https://crates.io/crates/secretscout
```

**Tasks**:
- [ ] Login to crates.io with GitHub account
- [ ] Review package contents
- [ ] Perform dry-run publish
- [ ] Publish first version manually
- [ ] Verify crate appears on crates.io
- [ ] Test installation: `cargo install secretscout`

---

### 4.3 Automate crates.io Publishing

**Integration into release workflow**:

Add to `.github/workflows/release.yml`:

```yaml
  publish-crates:
    needs: [build-binaries, run-tests]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Publish to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_TOKEN }}
        run: |
          cd secretscout
          cargo publish --token ${CARGO_REGISTRY_TOKEN}
```

**Tasks**:
- [ ] Add crates.io publish job to workflow
- [ ] Test with a pre-release version (e.g., 3.1.0-beta.1)
- [ ] Verify automation works end-to-end

---

## Phase 5: Documentation & Release Process

### 5.1 Create Publishing Documentation

**File**: `docs/PUBLISHING.md`

**Contents**:
- Prerequisites for publishing
- Version bumping process
- Manual release steps
- Automated release process
- Troubleshooting common issues
- Rollback procedures

**Template**:
```markdown
# Publishing SecretScout

## Prerequisites
- npm account with publish access
- crates.io account with publish access
- GitHub repository write access
- Secrets configured in GitHub

## Release Process

### 1. Version Bump
- Update version in `Cargo.toml` (workspace)
- Update CHANGELOG.md
- Commit: `git commit -m "chore: bump version to vX.Y.Z"`

### 2. Create Release Tag
- Tag: `git tag vX.Y.Z`
- Push: `git push origin vX.Y.Z`

### 3. Automated Steps (GitHub Actions)
- Builds binaries for all platforms
- Runs tests
- Creates GitHub release
- Publishes npm packages
- Publishes to crates.io

### 4. Verification
- Check GitHub release page
- Verify npm: `npm view secretscout`
- Verify crates.io: `cargo search secretscout`
- Test install: `npm install -g secretscout`
- Test install: `cargo install secretscout`
```

**Tasks**:
- [ ] Create comprehensive PUBLISHING.md
- [ ] Document all manual steps
- [ ] Add troubleshooting section
- [ ] Include rollback procedures

---

### 5.2 Update Main README

**File**: `README.md`

**Updates needed**:
1. Installation section - clarify npm vs cargo
2. Add badges for npm and crates.io
3. Link to publishing docs for maintainers

**Additions**:
```markdown
[![npm version](https://img.shields.io/npm/v/secretscout.svg)](https://www.npmjs.com/package/secretscout)
[![crates.io](https://img.shields.io/crates/v/secretscout.svg)](https://crates.io/crates/secretscout)

## Installation

### Via npm (Recommended for most users)
```bash
npm install -g secretscout
secretscout --version
```

### Via cargo (For Rust developers)
```bash
cargo install secretscout
secretscout --version
```

### From source
```bash
git clone https://github.com/globalbusinessadvisors/SecretScout.git
cd SecretScout
cargo build --release
./target/release/secretscout --version
```
```

**Tasks**:
- [ ] Add npm and crates.io badges
- [ ] Update installation instructions
- [ ] Clarify different installation methods
- [ ] Add troubleshooting for each method

---

### 5.3 Create CHANGELOG Structure

**File**: `CHANGELOG.md`

**Format**: Follow Keep a Changelog format

**Template**:
```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- npm package distribution
- crates.io publishing
- cargo-dist integration

## [3.1.0] - 2025-XX-XX

### Added
- Initial npm support
- Multi-platform binaries

## [3.0.1] - Previous release
...
```

**Tasks**:
- [ ] Create or update CHANGELOG.md
- [ ] Document new features
- [ ] Follow semantic versioning
- [ ] Update on each release

---

## Phase 6: Testing & Validation

### 6.1 Test npm Installation Flow

**Platforms to test**:
- [ ] Linux x64 (Ubuntu 20.04, 22.04)
- [ ] macOS x64 (Intel)
- [ ] macOS ARM64 (M1/M2)
- [ ] Windows x64

**Test scenarios**:
```bash
# Global install
npm install -g secretscout
secretscout --version
secretscout detect --help

# Local install
mkdir test-project
cd test-project
npm init -y
npm install secretscout
npx secretscout --version

# From tarball
npm pack
npm install -g secretscout-3.1.0.tgz
```

**Validation**:
- [ ] Binary executes successfully
- [ ] All platforms work
- [ ] Error messages are helpful
- [ ] Performance is acceptable

---

### 6.2 Test cargo Installation Flow

**Test scenarios**:
```bash
# Install from crates.io
cargo install secretscout

# Verify installation
which secretscout
secretscout --version

# Run tests
secretscout detect --source .

# Uninstall
cargo uninstall secretscout
```

**Validation**:
- [ ] Installation succeeds
- [ ] Binary is in PATH
- [ ] All features work
- [ ] Documentation is accessible: `cargo doc --open`

---

### 6.3 Test Release Workflow

**Pre-release testing**:

1. Create test tag in a fork:
   ```bash
   git tag v3.1.0-test
   git push origin v3.1.0-test
   ```

2. Observe GitHub Actions:
   - [ ] All platform builds succeed
   - [ ] Tests pass on all platforms
   - [ ] Artifacts are created
   - [ ] GitHub release is created

3. Download and test artifacts:
   - [ ] Each platform binary runs
   - [ ] Version is correct
   - [ ] No missing dependencies

**Tasks**:
- [ ] Test in fork or test repository first
- [ ] Verify all automation works
- [ ] Fix any issues before production release

---

## Phase 7: Production Release

### 7.1 Pre-release Checklist

**Before creating a release tag**:

- [ ] All tests pass locally
- [ ] Documentation is up to date
- [ ] CHANGELOG.md is updated
- [ ] Version numbers are consistent
- [ ] All dependencies are up to date
- [ ] Security audit passes: `cargo audit`
- [ ] Clippy has no warnings: `cargo clippy --all-features`
- [ ] Code is formatted: `cargo fmt --all`
- [ ] README reflects current version
- [ ] All TODO items are resolved

---

### 7.2 Release Steps

**Execute in order**:

1. **Version Bump**
   ```bash
   # Update Cargo.toml workspace version
   sed -i 's/version = "3.1.0"/version = "3.2.0"/' Cargo.toml

   # Update package.json
   npm version 3.2.0 --no-git-tag-version

   # Commit
   git add Cargo.toml package.json package-lock.json
   git commit -m "chore: bump version to v3.2.0"
   ```

2. **Update CHANGELOG**
   ```bash
   # Add release date to [Unreleased] section
   # Create new [Unreleased] section
   git add CHANGELOG.md
   git commit -m "docs: update changelog for v3.2.0"
   ```

3. **Create and Push Tag**
   ```bash
   git tag v3.2.0
   git push origin main
   git push origin v3.2.0
   ```

4. **Monitor GitHub Actions**
   - Watch release workflow execution
   - Verify all jobs complete successfully
   - Check GitHub releases page

5. **Verify Deployments**
   ```bash
   # Check npm
   npm view secretscout version

   # Check crates.io
   cargo search secretscout

   # Test installations
   npm install -g secretscout@latest
   cargo install secretscout
   ```

6. **Post-release**
   - [ ] Update version to next development version
   - [ ] Announce on GitHub Discussions
   - [ ] Update project website if applicable

---

### 7.3 Rollback Procedures

**If release fails or has critical bugs**:

1. **Yank npm version**
   ```bash
   npm unpublish secretscout@3.2.0
   # Or deprecate if already widely used:
   npm deprecate secretscout@3.2.0 "Critical bug, use 3.1.0 instead"
   ```

2. **Yank crates.io version**
   ```bash
   cargo yank --vers 3.2.0 secretscout
   # Note: This doesn't delete, just marks as yanked
   ```

3. **Delete GitHub release**
   - Go to Releases page
   - Delete the problematic release
   - Delete the git tag:
     ```bash
     git tag -d v3.2.0
     git push origin :refs/tags/v3.2.0
     ```

4. **Hotfix release**
   - Create branch from previous good version
   - Apply minimal fix
   - Release as v3.2.1

---

## Success Criteria

### npm Integration Complete
- [ ] Users can install with `npm install -g secretscout`
- [ ] Works on Linux, macOS (Intel & ARM), Windows
- [ ] Automatic updates via npm
- [ ] Package size < 10 MB per platform
- [ ] Listed on npmjs.com with good README

### crates.io Integration Complete
- [ ] Users can install with `cargo install secretscout`
- [ ] Published with all metadata
- [ ] Documentation renders correctly
- [ ] Listed on crates.io with examples
- [ ] Keywords and categories set appropriately

### Automation Complete
- [ ] GitHub Actions builds all platforms
- [ ] Releases triggered by version tags
- [ ] npm publish is automatic
- [ ] crates.io publish is automatic
- [ ] Release notes auto-generated
- [ ] Artifacts uploaded to GitHub releases

### Documentation Complete
- [ ] README has install instructions
- [ ] PUBLISHING.md for maintainers
- [ ] CHANGELOG follows standard format
- [ ] API documentation is complete
- [ ] Troubleshooting guides exist

---

## Timeline Estimate

| Phase | Estimated Time | Dependencies |
|-------|----------------|--------------|
| Phase 1: Restructuring | 2-3 hours | None |
| Phase 2: npm Setup | 4-5 hours | Phase 1 |
| Phase 3: GitHub Actions | 2-3 hours | Phase 1, 2 |
| Phase 4: crates.io | 1-2 hours | Phase 1 |
| Phase 5: Documentation | 2-3 hours | All phases |
| Phase 6: Testing | 3-4 hours | All phases |
| Phase 7: Production Release | 1 hour | All phases |
| **Total** | **15-21 hours** | |

---

## Resources & References

### Documentation
- [cargo-dist book](https://opensource.axo.dev/cargo-dist/)
- [crates.io publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
- [npm publishing guide](https://docs.npmjs.com/packages-and-modules/contributing-packages-to-the-registry)
- [Semantic Versioning](https://semver.org/)

### Example Projects Using Similar Setup
- [ripgrep](https://github.com/BurntSushi/ripgrep) - Rust CLI with cargo-dist
- [swc](https://github.com/swc-project/swc) - npm + Rust binaries
- [esbuild](https://github.com/evanw/esbuild) - Platform-specific npm packages

### Tools
- [cargo-dist](https://github.com/axodotdev/cargo-dist)
- [cargo-release](https://github.com/crate-ci/cargo-release)
- [cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)

---

## Notes

### Important Considerations
1. **Version Synchronization**: Keep npm and cargo versions in sync
2. **Binary Size**: Optimize for size in release builds (already configured)
3. **License Compliance**: Ensure all dependencies are MIT compatible
4. **Security**: Run `cargo audit` before each release
5. **Breaking Changes**: Follow semantic versioning strictly

### Future Enhancements
- [ ] Add homebrew formula for macOS
- [ ] Add Debian/Ubuntu APT repository
- [ ] Add Chocolatey package for Windows
- [ ] Consider Docker image distribution
- [ ] Add update notification mechanism

---

**Plan Status**: Ready for Implementation
**Next Action**: Begin Phase 1 - Project Restructuring
