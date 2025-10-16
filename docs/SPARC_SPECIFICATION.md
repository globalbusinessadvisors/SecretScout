# SPARC SPECIFICATION: Gitleaks-Action Rust Port

**Project:** SecretScout - Rust Port of gitleaks-action
**Methodology:** SPARC (Specification, Pseudocode, Architecture, Refinement, Completion)
**Phase:** SPECIFICATION ONLY
**Target Deployment:** Rust crates + WASM
**Date:** October 15, 2025
**Version:** 1.0

---

## TABLE OF CONTENTS

1. [Executive Summary](#1-executive-summary)
2. [Project Overview](#2-project-overview)
3. [Functional Requirements](#3-functional-requirements)
4. [Technical Requirements](#4-technical-requirements)
5. [System Inputs](#5-system-inputs)
6. [System Outputs](#6-system-outputs)
7. [Behavioral Specifications](#7-behavioral-specifications)
8. [Integration Requirements](#8-integration-requirements)
9. [Performance Requirements](#9-performance-requirements)
10. [Security Requirements](#10-security-requirements)
11. [Deployment Requirements](#11-deployment-requirements)
12. [Constraints and Limitations](#12-constraints-and-limitations)
13. [Success Criteria](#13-success-criteria)
14. [Out of Scope](#14-out-of-scope)

---

## 1. EXECUTIVE SUMMARY

### 1.1 Project Vision

Create a Rust-based implementation of gitleaks-action that:
- Maintains 100% functional parity with the original Node.js implementation
- Compiles to both native binaries (crates) and WebAssembly (WASM)
- Provides superior performance, smaller binary size, and enhanced security
- Integrates seamlessly with GitHub Actions workflows

### 1.2 Key Objectives

1. **Functional Parity**: Replicate all features of gitleaks-action v2.x
2. **Multi-Target Deployment**: Support both crate distribution and WASM compilation
3. **Performance**: Achieve <2 minute build times (cached) and <100ms runtime overhead
4. **Size Optimization**: Target <500KB for WASM binary (optimized)
5. **Security**: Leverage Rust's memory safety and WASM sandboxing

### 1.3 Source Analysis Summary

**Original Implementation:**
- Language: JavaScript (Node.js 20)
- Source Code: 706 lines across 4 modules
- Distribution: 140,797 lines (bundled with dependencies)
- Build System: Vercel NCC

**Core Modules:**
1. `index.js` (205 lines) - Main orchestrator
2. `gitleaks.js` (282 lines) - Binary management and scanning
3. `keygen.js` (163 lines) - License validation
4. `summary.js` (56 lines) - Job summary generation

---

## 2. PROJECT OVERVIEW

### 2.1 What is Gitleaks-Action?

A GitHub Action that wraps the gitleaks CLI tool to detect hardcoded secrets, passwords, API keys, and tokens in git repositories. It provides:
- Automated secret scanning on push, pull request, and scheduled events
- SARIF-formatted security reports
- Inline PR comments for detected secrets
- GitHub Actions job summaries with detailed findings
- License-based access control for organizations

### 2.2 Why Rust + WASM?

**Rust Benefits:**
- Memory safety without garbage collection
- Zero-cost abstractions
- Strong type system prevents runtime errors
- Excellent tooling ecosystem (cargo, clippy, rustfmt)
- Native performance

**WASM Benefits:**
- Universal binary (no cross-compilation needed)
- Sandboxed execution (enhanced security)
- Fast startup times (vs Docker containers)
- Cross-platform compatibility (Linux, macOS, Windows)
- Smaller distribution size than Docker images

**Combined Value:**
- Rust provides the implementation language
- WASM provides the deployment target
- Crates provide modular, reusable components
- npm package distribution for easy consumption

### 2.3 Target Architecture

```
┌─────────────────────────────────────────────────────────┐
│           GitHub Actions Runtime (Node.js 20/24)        │
└────────────────────┬────────────────────────────────────┘
                     │
            ┌────────┴────────┐
            │  index.js       │ (JavaScript wrapper)
            │  - Input parsing│
            │  - GitHub API   │
            │  - File I/O     │
            └────────┬────────┘
                     │
            ┌────────┴────────┐
            │  WASM Module    │ (Rust compiled)
            │  - Event routing│
            │  - SARIF parsing│
            │  - Fingerprints │
            │  - Validation   │
            └────────┬────────┘
                     │
            ┌────────┴────────┐
            │ Gitleaks Binary │ (External process)
            │  - Secret scan  │
            │  - SARIF output │
            └─────────────────┘
```

---

## 3. FUNCTIONAL REQUIREMENTS

### 3.1 Core Capabilities

#### FR-1: Event Type Support
**Requirement:** The system MUST support four GitHub event types.

**Specifications:**
1. **push** - Incremental scanning of pushed commits
   - Scan range: First commit to last commit in push
   - Single commit optimization: Use `--log-opts=-1`
   - Multi-commit: Use `--log-opts=--no-merges --first-parent {base}^..{head}`

2. **pull_request** - PR-specific scanning with inline comments
   - Fetch PR commits via GitHub API
   - Determine scan range from first to last PR commit
   - Post review comments on detected secrets
   - Deduplicate comments to prevent spam

3. **workflow_dispatch** - Manual full repository scan
   - No log-opts (scans entire history)
   - User-triggered execution
   - No PR comment posting

4. **schedule** - Automated periodic scanning
   - No log-opts (scans entire history)
   - Cron-triggered execution
   - Special handling for undefined repository metadata

**Acceptance Criteria:**
- Each event type triggers appropriate scan strategy
- Exit code reflects scan results (0=success, 2=secrets found, 1=error)
- Outputs are generated according to event type

#### FR-2: Binary Management
**Requirement:** The system MUST download, cache, and execute the gitleaks binary.

**Specifications:**
1. **Version Resolution**
   - Default: Version 8.24.3 (hard-coded fallback)
   - Override: `GITLEAKS_VERSION` environment variable
   - Special: "latest" fetches newest release from GitHub API

2. **Download Logic**
   - Check cache first (key: `gitleaks-cache-{version}-{platform}-{arch}`)
   - If cache miss: Download from GitHub releases
   - URL pattern: `https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{platform}_{arch}.{ext}`
   - Extract archive (tar.gz for Unix, zip for Windows)
   - Add to PATH

3. **Platform Detection**
   - Platforms: linux, darwin, windows (map from "win32")
   - Architectures: x64, arm64, arm
   - Validate platform/arch combination

4. **Execution**
   - Base arguments: `detect --redact -v --exit-code=2 --report-format=sarif --report-path=results.sarif --log-level=debug`
   - Add event-specific `--log-opts`
   - Add `--config={path}` if GITLEAKS_CONFIG set
   - Capture stdout/stderr
   - Return exit code

**Acceptance Criteria:**
- Binary downloads successfully on all supported platforms
- Cache reduces download time by 90%+
- Binary executes with correct arguments
- Exit codes propagate correctly (0, 1, 2)

#### FR-3: SARIF Processing
**Requirement:** The system MUST parse SARIF v2 output from gitleaks.

**Specifications:**
1. **SARIF Structure**
   ```json
   {
     "runs": [{
       "results": [{
         "ruleId": "string",
         "partialFingerprints": {
           "commitSha": "string",
           "author": "string",
           "email": "string",
           "date": "string"
         },
         "locations": [{
           "physicalLocation": {
             "artifactLocation": {"uri": "string"},
             "region": {"startLine": number}
           }
         }]
       }]
     }]
   }
   ```

2. **Extraction Requirements**
   - Extract all results from `runs[0].results[]`
   - Parse `ruleId` (detection rule name)
   - Parse `partialFingerprints` (custom gitleaks extension)
   - Parse `locations[0].physicalLocation` (file path and line)
   - Handle missing optional fields gracefully

3. **Fingerprint Generation**
   - Format: `{commitSha}:{filePath}:{ruleId}:{startLine}`
   - Example: `abc123def:src/config.js:aws-access-token:42`
   - Used for .gitleaksignore file

**Acceptance Criteria:**
- Parse valid SARIF without errors
- Extract all required fields
- Generate correct fingerprint format
- Handle malformed SARIF gracefully (error message)

#### FR-4: PR Comment Creation
**Requirement:** The system MUST post inline review comments on pull requests.

**Specifications:**
1. **Comment Content**
   - Emoji indicator: 🛑
   - Rule ID that triggered detection
   - Commit SHA containing the secret
   - Fingerprint for .gitleaksignore
   - Optional user mentions (GITLEAKS_NOTIFY_USER_LIST)

2. **Comment Placement**
   - File: From SARIF `locations[0].physicalLocation.artifactLocation.uri`
   - Line: From SARIF `locations[0].physicalLocation.region.startLine`
   - Commit: From SARIF `partialFingerprints.commitSha`
   - Side: "RIGHT" (new code)

3. **Deduplication**
   - Fetch existing PR comments
   - Compare: body, path, line number
   - Skip if identical comment exists
   - Prevents spam on re-runs

4. **Error Handling**
   - Large diffs may prevent commenting (GitHub API limitation)
   - Log warning, continue execution
   - Secrets still reported in summary and artifacts

**Acceptance Criteria:**
- Comments appear on correct line in PR diff
- Fingerprints match format specification
- Duplicate comments not posted
- Feature can be disabled via GITLEAKS_ENABLE_COMMENTS=false

#### FR-5: Job Summary Generation
**Requirement:** The system MUST generate GitHub Actions job summaries.

**Specifications:**
1. **No Secrets Detected (Exit Code 0)**
   ```markdown
   ## No leaks detected ✅
   ```

2. **Secrets Detected (Exit Code 2)**
   - Heading: "🛑 Gitleaks detected secrets 🛑"
   - HTML table with columns:
     - Rule ID
     - Commit (hyperlink, first 7 chars)
     - Secret URL (hyperlink to file:line)
     - Start Line
     - Author
     - Date
     - Email
     - File (hyperlink)
   - One row per detected secret

3. **Error (Exit Code 1)**
   ```markdown
   ## ❌ Gitleaks exited with error. Exit code [1]
   ```

4. **Table Format**
   - GitHub-flavored Markdown
   - HTML links to repository URLs
   - Commit link: `{repo_url}/commit/{commitSha}`
   - Secret link: `{repo_url}/blob/{commitSha}/{filePath}#L{startLine}`

**Acceptance Criteria:**
- Summary appears on GitHub Actions run page
- Links navigate to correct locations
- Table formatting is correct
- Feature can be disabled via GITLEAKS_ENABLE_SUMMARY=false

#### FR-6: Artifact Upload
**Requirement:** The system MUST upload SARIF results as workflow artifacts.

**Specifications:**
1. **Artifact Details**
   - Name: "gitleaks-results.sarif"
   - Content: Copy of results.sarif file
   - Uploaded when: Secrets detected AND GITLEAKS_ENABLE_UPLOAD_ARTIFACT=true

2. **Upload Logic**
   - Use GitHub Actions artifact API
   - Include: results.sarif file
   - Root directory: GITHUB_WORKSPACE
   - Option: continueOnError=true

**Acceptance Criteria:**
- Artifact appears in workflow run artifacts
- File is downloadable
- Contains valid SARIF
- Feature can be disabled

#### FR-7: License Validation
**Requirement:** The system MUST validate licenses for organization accounts.

**Specifications:**
1. **Account Type Detection**
   - Call GitHub API: `GET /users/{username}`
   - Extract `type` field ("Organization" or "User")
   - Fallback: Assume organization (require license)

2. **License Requirement**
   - Organizations: GITLEAKS_LICENSE environment variable REQUIRED
   - Personal accounts: License NOT required

3. **Keygen.sh Integration**
   - Endpoint: `POST /v1/accounts/{account}/licenses/actions/validate-key`
   - Request: License key + repository fingerprint
   - Responses:
     - VALID → Proceed
     - TOO_MANY_MACHINES → Error (license limit)
     - NO_MACHINE → Attempt activation
   - Activation: `POST /v1/accounts/{account}/machines`

4. **Current Status**
   - Feature is disabled in current implementation (lines 124-130 commented)
   - Reason: Payment processing issues with Keygen service
   - Specification retained for future re-enablement

**Acceptance Criteria:**
- Personal accounts run without license
- Organizations fail without GITLEAKS_LICENSE
- Valid licenses allow execution
- Invalid licenses show clear error message

---

### 3.2 Configuration Management

#### FR-8: Environment Variable Configuration
**Requirement:** The system MUST accept configuration via environment variables.

**Specifications:**

| Variable | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| GITHUB_TOKEN | string | Yes (PR events) | - | GitHub API authentication |
| GITLEAKS_LICENSE | string | Conditional | - | License key (orgs only) |
| GITLEAKS_VERSION | string | No | "8.24.3" | Gitleaks version to use |
| GITLEAKS_CONFIG | path | No | Auto-detect | Path to config file |
| GITLEAKS_ENABLE_SUMMARY | boolean | No | true | Enable job summary |
| GITLEAKS_ENABLE_UPLOAD_ARTIFACT | boolean | No | true | Enable artifact upload |
| GITLEAKS_ENABLE_COMMENTS | boolean | No | true | Enable PR comments |
| GITLEAKS_NOTIFY_USER_LIST | string | No | - | Comma-separated @mentions |
| BASE_REF | string | No | Auto-detect | Override base git ref |

**Boolean Parsing:**
- False values: "false", "0"
- True values: All other values (including "true", "1", empty string)

**Acceptance Criteria:**
- All variables parsed correctly
- Boolean logic matches original implementation
- Missing optional variables use defaults
- Missing required variables cause clear error

#### FR-9: Configuration File Discovery
**Requirement:** The system MUST support custom gitleaks configuration files.

**Specifications:**
1. **Priority Order**
   - Explicit: GITLEAKS_CONFIG environment variable
   - Auto-detect: gitleaks.toml in repository root
   - Default: Gitleaks built-in configuration

2. **File Format**
   - TOML syntax
   - Contents: gitleaks-specific rules, exclusions, etc.
   - Validation: Performed by gitleaks binary

3. **Argument Passing**
   - If found: Add `--config={path}` to gitleaks arguments
   - If not found: Omit argument (use gitleaks defaults)

**Acceptance Criteria:**
- GITLEAKS_CONFIG path is respected
- Auto-detection finds gitleaks.toml if present
- Missing config does not cause error
- Invalid config produces clear error from gitleaks

---

## 4. TECHNICAL REQUIREMENTS

### 4.1 Programming Language

**Requirement:** Rust 2021 edition or later.

**Rationale:**
- Memory safety without garbage collection
- Strong type system
- Excellent WASM support
- Mature ecosystem

### 4.2 Compilation Targets

**Requirement:** Support multiple compilation targets.

**Primary Targets:**
1. **wasm32-unknown-unknown** - Universal WASM binary
2. **x86_64-unknown-linux-gnu** - Linux native (GitHub runners)
3. **x86_64-apple-darwin** - macOS Intel native
4. **aarch64-apple-darwin** - macOS Apple Silicon native
5. **x86_64-pc-windows-msvc** - Windows native

**Note:** WASM target provides universal compatibility; native targets are optional for performance optimization.

### 4.3 Dependency Requirements

**Core Dependencies:**

```toml
[dependencies]
# WASM bindings
wasm-bindgen = "0.2.104"
serde = { version = "1.0", features = ["derive"], default-features = false }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
serde-wasm-bindgen = "0.6"

# GitHub API client
octocrab = { version = "0.38", default-features = false, features = ["rustls-tls"] }

# HTTP client
reqwest = { version = "0.12", features = ["json", "rustls-tls"], default-features = false }

# Async runtime (conditional)
tokio = { version = "1.40", features = ["rt", "macros"], optional = true }

# Error handling
thiserror = "1.0"
anyhow = "1.0"

# Archive extraction (native only)
tar = { version = "0.4", optional = true }
flate2 = { version = "1.0", optional = true }
zip = { version = "0.6", optional = true }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.40", features = ["full"] }
tar = "0.4"
flate2 = "1.0"
zip = "0.6"

[dev-dependencies]
wasm-bindgen-test = "0.3"
tempfile = "3.12"
mockito = "1.5"
```

**Rationale:**
- Minimal dependencies for WASM (size constraints)
- Conditional compilation for native-only features
- `default-features = false` to avoid bloat
- `rustls-tls` instead of OpenSSL (smaller, cross-platform)

### 4.4 Crate Structure

**Requirement:** Single crate with library and WASM targets.

**Recommended Structure:**
```
secretscout/
├── Cargo.toml
├── Cargo.lock
├── action.yml
├── src/
│   ├── lib.rs              # Library entry point
│   ├── wasm.rs             # WASM bindings (feature-gated)
│   ├── event.rs            # Event parsing
│   ├── scanner.rs          # Gitleaks execution
│   ├── sarif.rs            # SARIF parsing
│   ├── github.rs           # GitHub API client
│   ├── summary.rs          # Job summary generation
│   ├── license.rs          # License validation
│   └── config.rs           # Configuration management
├── dist/
│   ├── index.js            # JavaScript wrapper
│   ├── secretscout_bg.wasm # Compiled WASM
│   └── secretscout.js      # wasm-bindgen glue
└── tests/
    ├── integration/
    └── fixtures/
```

**Cargo.toml Configuration:**
```toml
[package]
name = "secretscout"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[profile.release]
opt-level = 'z'
lto = true
codegen-units = 1
strip = true
panic = 'abort'

[features]
default = []
wasm = ["wasm-bindgen", "serde-wasm-bindgen"]
native = ["tokio", "tar", "flate2", "zip"]
```

---

## 5. SYSTEM INPUTS

### 5.1 Required Inputs

#### IN-1: GitHub Event Payload
- **Source:** File at path specified by `GITHUB_EVENT_PATH` environment variable
- **Format:** JSON
- **Content:** Event-specific metadata (repository, commits, PR details, etc.)
- **Validation:** Must be valid JSON, must contain required fields per event type

#### IN-2: GitHub Token
- **Source:** `GITHUB_TOKEN` environment variable
- **Format:** String (GitHub personal access token or workflow token)
- **Required For:** Pull request events, API calls
- **Validation:** Non-empty string

#### IN-3: Event Type
- **Source:** `GITHUB_EVENT_NAME` environment variable
- **Format:** String enum
- **Valid Values:** "push", "pull_request", "workflow_dispatch", "schedule"
- **Validation:** Must be one of the supported values

### 5.2 Conditional Inputs

#### IN-4: License Key
- **Source:** `GITLEAKS_LICENSE` environment variable
- **Format:** String (opaque token)
- **Required For:** Organization accounts only
- **Validation:** Non-empty string (format validated by Keygen API)

### 5.3 Optional Inputs

#### IN-5: Gitleaks Version
- **Source:** `GITLEAKS_VERSION` environment variable
- **Format:** String (version number or "latest")
- **Default:** "8.24.3"
- **Examples:** "8.15.3", "latest"

#### IN-6: Configuration File Path
- **Source:** `GITLEAKS_CONFIG` environment variable
- **Format:** File path (absolute or relative to GITHUB_WORKSPACE)
- **Default:** Auto-detect "gitleaks.toml"
- **Validation:** File must exist and be readable

#### IN-7: Feature Toggles
- **GITLEAKS_ENABLE_SUMMARY** (boolean, default: true)
- **GITLEAKS_ENABLE_UPLOAD_ARTIFACT** (boolean, default: true)
- **GITLEAKS_ENABLE_COMMENTS** (boolean, default: true)

#### IN-8: User Notification List
- **Source:** `GITLEAKS_NOTIFY_USER_LIST` environment variable
- **Format:** Comma-separated GitHub usernames with @ prefix
- **Example:** "@user1,@user2,@user3"
- **Validation:** Optional, whitespace-tolerant

#### IN-9: Base Reference Override
- **Source:** `BASE_REF` environment variable
- **Format:** Git reference (commit SHA, branch name, tag)
- **Purpose:** Override auto-detected base ref for custom scan ranges
- **Validation:** Must be valid git reference

---

## 6. SYSTEM OUTPUTS

### 6.1 Exit Codes

**Requirement:** The system MUST exit with specific codes indicating status.

| Exit Code | Meaning | GitHub Actions Status |
|-----------|---------|----------------------|
| 0 | No secrets detected | Success ✅ |
| 1 | Error occurred | Failure ❌ |
| 2 | Secrets detected | Failure ❌ (AFTER processing results) |

**Critical Note:** Exit code 2 must process results (comments, summary, artifacts) BEFORE exiting with failure status.

### 6.2 File Outputs

#### OUT-1: SARIF Report
- **Path:** `{GITHUB_WORKSPACE}/results.sarif`
- **Format:** SARIF v2 JSON
- **Content:** Detected secrets with metadata
- **Generated By:** Gitleaks binary
- **Consumed By:** SARIF parser, artifact uploader, summary generator

#### OUT-2: Workflow Artifact
- **Name:** "gitleaks-results.sarif"
- **Content:** Copy of results.sarif
- **Condition:** Secrets detected AND GITLEAKS_ENABLE_UPLOAD_ARTIFACT=true
- **Accessible From:** GitHub Actions run page (Artifacts section)

### 6.3 GitHub Actions Outputs

#### OUT-3: Action Output
- **Name:** "exit-code"
- **Type:** Number (0, 1, or 2)
- **Set Via:** `core.setOutput("exit-code", exitCode)` in JavaScript wrapper
- **Usage:** Subsequent workflow steps can access via `${{ steps.scan.outputs.exit-code }}`

### 6.4 GitHub API Outputs

#### OUT-4: Pull Request Comments
- **Type:** Inline review comments
- **Posted To:** Specific lines in PR diff
- **Format:** Markdown with structured content
- **Condition:** PR event AND secrets detected AND GITLEAKS_ENABLE_COMMENTS=true
- **Content:** Rule ID, commit SHA, fingerprint, optional mentions

#### OUT-5: Job Summary
- **Type:** Markdown/HTML
- **Displayed On:** GitHub Actions run summary page
- **Format:** Heading + table (if secrets) or status message (if clean/error)
- **Condition:** GITLEAKS_ENABLE_SUMMARY=true

### 6.5 Logging Outputs

#### OUT-6: Console Logs
- **Levels:** INFO, DEBUG, WARNING, ERROR
- **Format:** GitHub Actions log commands (`::info::`, `::error::`, etc.)
- **Content:** Execution progress, errors, warnings
- **Destination:** GitHub Actions run logs

---

## 7. BEHAVIORAL SPECIFICATIONS

### 7.1 Push Event Behavior

**Trigger:** Code pushed to repository

**Flow:**
1. Parse event JSON from GITHUB_EVENT_PATH
2. Extract commits array
3. If commits.length == 0: Log info, exit 0
4. Determine base ref: commits[0].id
5. Determine head ref: commits[commits.length - 1].id
6. If BASE_REF env var set: Override base ref
7. If baseRef == headRef: Use `--log-opts=-1` (single commit)
8. Else: Use `--log-opts=--no-merges --first-parent {baseRef}^..{headRef}`
9. Execute gitleaks with arguments
10. Parse SARIF output
11. Generate job summary (if enabled)
12. Upload artifact (if enabled and secrets found)
13. Exit with gitleaks exit code

**Outputs:**
- Job summary: Yes (if enabled)
- Artifacts: Yes (if enabled and secrets found)
- PR comments: No

### 7.2 Pull Request Event Behavior

**Trigger:** PR opened, updated, or synchronized

**Flow:**
1. Parse event JSON from GITHUB_EVENT_PATH
2. Extract pull_request.number
3. Fetch PR commits via GitHub API: `GET /repos/{owner}/{repo}/pulls/{number}/commits`
4. Determine base ref: First commit SHA
5. Determine head ref: Last commit SHA
6. If BASE_REF env var set: Override base ref
7. Use `--log-opts=--no-merges --first-parent {baseRef}^..{headRef}`
8. Execute gitleaks with arguments
9. Parse SARIF output
10. If GITLEAKS_ENABLE_COMMENTS=true:
    a. Fetch existing PR comments
    b. For each detected secret:
       - Check if duplicate comment exists
       - If not: Post review comment at file:line
       - Include user mentions if configured
11. Generate job summary (if enabled)
12. Upload artifact (if enabled and secrets found)
13. Exit with gitleaks exit code

**Outputs:**
- Job summary: Yes (if enabled)
- Artifacts: Yes (if enabled and secrets found)
- PR comments: Yes (if enabled and secrets found)

### 7.3 Workflow Dispatch Event Behavior

**Trigger:** Manual workflow execution

**Flow:**
1. Parse event JSON from GITHUB_EVENT_PATH
2. No log-opts (scan entire repository)
3. Execute gitleaks: `detect --redact -v --exit-code=2 --report-format=sarif --report-path=results.sarif --log-level=debug`
4. Parse SARIF output
5. Generate job summary (if enabled)
6. Upload artifact (if enabled and secrets found)
7. Exit with gitleaks exit code

**Outputs:**
- Job summary: Yes (if enabled)
- Artifacts: Yes (if enabled and secrets found)
- PR comments: No

### 7.4 Schedule Event Behavior

**Trigger:** Cron schedule

**Flow:**
1. Parse event JSON from GITHUB_EVENT_PATH
2. Special handling: eventJSON.repository may be undefined
3. If undefined: Construct repository object from environment variables
   - owner.login = GITHUB_REPOSITORY_OWNER
   - full_name = GITHUB_REPOSITORY
   - name = GITHUB_REPOSITORY minus owner prefix
4. No log-opts (scan entire repository)
5. Execute gitleaks
6. Parse SARIF output
7. Generate job summary (if enabled)
8. Upload artifact (if enabled and secrets found)
9. Exit with gitleaks exit code

**Outputs:**
- Job summary: Yes (if enabled)
- Artifacts: Yes (if enabled and secrets found)
- PR comments: No

### 7.5 License Validation Behavior (When Enabled)

**Flow:**
1. Determine repository owner type:
   - Call GitHub API: `GET /users/{owner}`
   - Parse response.type
2. If type == "User": Skip license validation
3. If type == "Organization" or unknown:
   - Check for GITLEAKS_LICENSE environment variable
   - If missing: Error and exit 1
   - If present: Validate via Keygen API
4. Validation steps:
   - POST to Keygen: `/v1/accounts/{account}/licenses/actions/validate-key`
   - Include license key and repository fingerprint
   - Handle responses:
     - VALID: Continue
     - TOO_MANY_MACHINES: Error (license limit exceeded)
     - NO_MACHINE/NO_MACHINES/FINGERPRINT_SCOPE_MISMATCH: Attempt activation
5. Activation steps:
   - POST to Keygen: `/v1/accounts/{account}/machines`
   - Include repository fingerprint, platform, name
   - If success: Continue
   - If failure: Error and exit 1

**Note:** Currently disabled in implementation but retained in specification.

### 7.6 Error Handling Behavior

#### Fatal Errors (Exit Immediately)
- Unsupported event type → Exit 1
- Missing GITHUB_TOKEN (PR events) → Exit 1
- Missing GITLEAKS_LICENSE (organizations) → Exit 1
- License validation failure → Exit 1
- Gitleaks exit code 1 → Exit 1
- Unexpected gitleaks exit code → Exit with that code

#### Non-Fatal Errors (Log Warning, Continue)
- GitHub API user lookup failure → Assume organization, require license
- Cache operation failure → Download fresh binary
- PR comment creation failure → Log warning, continue (secrets still in summary/artifacts)
- Archive extraction failure → Log error, continue (may fail later)

#### Special Cases
- Empty commit list (push events) → Exit 0 (success)
- Gitleaks exit code 2 → Process results, THEN exit 1 (failure)

---

## 8. INTEGRATION REQUIREMENTS

### 8.1 GitHub Actions Platform Integration

**Requirement:** Integrate with GitHub Actions runtime and APIs.

**action.yml Configuration:**
```yaml
name: 'SecretScout'
author: 'Gitleaks LLC'
description: 'Rust/WASM implementation of gitleaks-action for secret detection'
branding:
  icon: 'shield'
  color: 'red'

runs:
  using: 'node20'
  main: 'dist/index.js'

inputs:
  config-path:
    description: 'Path to gitleaks configuration file'
    required: false
  version:
    description: 'Gitleaks version to use (default: 8.24.3, or "latest")'
    required: false
    default: '8.24.3'
  enable-summary:
    description: 'Enable job summary generation'
    required: false
    default: 'true'
  enable-upload-artifact:
    description: 'Enable SARIF artifact upload'
    required: false
    default: 'true'
  enable-comments:
    description: 'Enable PR review comments'
    required: false
    default: 'true'
  notify-user-list:
    description: 'Comma-separated list of GitHub users to notify (@user1,@user2)'
    required: false

outputs:
  exit-code:
    description: 'Gitleaks exit code (0=no leaks, 1=error, 2=leaks found)'

env:
  GITHUB_TOKEN:
    description: 'GitHub token for API access'
    required: true
  GITLEAKS_LICENSE:
    description: 'License key (required for organizations)'
    required: false
```

**JavaScript Wrapper (dist/index.js):**
- Parse action inputs
- Set environment variables for WASM module
- Load WASM module
- Call WASM entry point
- Handle WASM errors
- Set action outputs
- Write to GITHUB_STEP_SUMMARY file
- Exit with appropriate code

### 8.2 GitHub REST API Integration

**Requirement:** Interact with GitHub REST API for metadata and comments.

**Required Endpoints:**

1. **GET /users/{username}**
   - Purpose: Determine account type
   - Authentication: GITHUB_TOKEN
   - Response: `{ "type": "Organization" | "User" }`

2. **GET /repos/zricethezav/gitleaks/releases/latest**
   - Purpose: Fetch latest gitleaks version
   - Authentication: Optional (public endpoint)
   - Response: `{ "tag_name": "v8.24.3" }`

3. **GET /repos/{owner}/{repo}/pulls/{number}/commits**
   - Purpose: Fetch PR commits for scan range
   - Authentication: GITHUB_TOKEN
   - Response: Array of commit objects with SHAs

4. **GET /repos/{owner}/{repo}/pulls/{number}/comments**
   - Purpose: Fetch existing PR comments (deduplication)
   - Authentication: GITHUB_TOKEN
   - Response: Array of review comment objects

5. **POST /repos/{owner}/{repo}/pulls/{number}/comments**
   - Purpose: Create inline PR review comment
   - Authentication: GITHUB_TOKEN
   - Body: `{ body, commit_id, path, side: "RIGHT", line }`

**Error Handling:**
- Retry transient failures (429, 5xx) with exponential backoff
- Log and continue on non-critical failures (comment posting)
- Exit on critical failures (user lookup, commit fetching)

### 8.3 Keygen.sh API Integration

**Requirement:** Validate licenses via Keygen.sh API (when feature is enabled).

**Required Endpoints:**

1. **POST /v1/accounts/{account}/licenses/actions/validate-key**
   - Purpose: Validate license key for repository
   - Headers: `Content-Type: application/vnd.api+json`
   - Body:
     ```json
     {
       "meta": {
         "key": "{GITLEAKS_LICENSE}",
         "scope": {
           "fingerprint": "{owner/repo}"
         }
       }
     }
     ```
   - Response constants: VALID, TOO_MANY_MACHINES, NO_MACHINE, etc.

2. **POST /v1/accounts/{account}/machines**
   - Purpose: Activate repository (associate with license)
   - Headers: `Authorization: License {GITLEAKS_LICENSE}`
   - Body:
     ```json
     {
       "data": {
         "type": "machines",
         "attributes": {
           "fingerprint": "{owner/repo}",
           "platform": "github-actions",
           "name": "{owner/repo}"
         },
         "relationships": {
           "license": {
             "data": {
               "type": "licenses",
               "id": "{license_id}"
             }
           }
         }
       }
     }
     ```
   - Response: 201 on success

**Error Handling:**
- Log all validation failures with detailed error messages
- Exit 1 on license validation failures (when feature enabled)

### 8.4 Gitleaks Binary Integration

**Requirement:** Execute gitleaks as external process and consume output.

**Execution Pattern:**
1. Construct argument array
2. Execute via process spawn
3. Capture stdout/stderr
4. Capture exit code
5. Parse SARIF output file

**Standard Arguments:**
- `detect` - Detection mode
- `--redact` - Redact secret values in output
- `-v` - Verbose logging
- `--exit-code=2` - Use exit code 2 for leaks
- `--report-format=sarif` - SARIF v2 output
- `--report-path=results.sarif` - Output file path
- `--log-level=debug` - Detailed logging

**Event-Specific Arguments:**
- Push (single commit): `--log-opts=-1`
- Push (range): `--log-opts=--no-merges --first-parent {base}^..{head}`
- PR: `--log-opts=--no-merges --first-parent {base}^..{head}`
- Workflow/Schedule: No log-opts

**Configuration Argument:**
- If GITLEAKS_CONFIG set: `--config={path}`

**Exit Code Handling:**
- 0: No leaks → Success
- 1: Error → Failure
- 2: Leaks → Process results, then failure

---

## 9. PERFORMANCE REQUIREMENTS

### 9.1 Build Performance

**Requirement:** Optimize build times for CI/CD pipelines.

**Specifications:**
- **Cold Build** (no cache): ≤ 5 minutes on GitHub Actions runners
- **Cached Build** (dependencies cached): ≤ 2 minutes
- **Incremental Build** (source changes only): ≤ 1 minute

**Optimization Strategies:**
- Use Swatinem/rust-cache for dependency caching
- Use sccache for compilation result caching
- Disable incremental compilation in CI (`CARGO_INCREMENTAL=0`)
- Use jetli/wasm-pack-action for fast toolchain installation
- Parallel compilation: `CARGO_BUILD_JOBS=$(nproc)`

### 9.2 Runtime Performance

**Requirement:** Minimize action execution overhead.

**Specifications:**
- **WASM Module Load**: ≤ 50ms (initialization)
- **Event Parsing**: ≤ 10ms (JSON deserialization)
- **SARIF Parsing**: ≤ 100ms (typical report with 10 findings)
- **GitHub API Calls**: ≤ 500ms per request (network dependent)
- **Total Overhead** (excluding gitleaks scan): ≤ 2 seconds

**Note:** Gitleaks scan time dominates (seconds to minutes depending on repo size). Action overhead should be negligible.

### 9.3 Binary Size

**Requirement:** Optimize WASM binary size for fast distribution.

**Specifications:**
- **Target Size**: ≤ 500 KB (uncompressed)
- **Gzip Compressed**: ≤ 200 KB (for npm distribution)
- **Debug Build**: ≤ 2 MB (for development)

**Optimization Techniques:**
- `opt-level = 'z'` (optimize for size)
- `lto = true` (link-time optimization)
- `codegen-units = 1` (maximum optimization)
- `strip = true` (remove debug symbols)
- `wasm-opt -Oz` (post-process optimization)
- Minimal dependencies with `default-features = false`

### 9.4 Memory Usage

**Requirement:** Operate within GitHub Actions runner memory limits.

**Specifications:**
- **WASM Module Heap**: ≤ 32 MB (typical)
- **Total Process Memory**: ≤ 100 MB (including Node.js runtime)
- **Peak Memory** (large SARIF): ≤ 200 MB

**Note:** GitHub Actions runners have 7 GB RAM. Memory constraints are not strict but efficiency is valued.

---

## 10. SECURITY REQUIREMENTS

### 10.1 Input Validation

**Requirement:** Validate all external inputs to prevent injection attacks.

**Specifications:**

1. **Path Validation**
   - Reject paths containing `..` (path traversal)
   - Validate paths exist and are readable
   - Constrain paths to GITHUB_WORKSPACE directory

2. **Git Reference Validation**
   - Validate commit SHAs (40-character hex)
   - Validate branch/tag names (no shell metacharacters)
   - Sanitize before passing to gitleaks

3. **Environment Variable Validation**
   - Check for SQL injection patterns (if constructing queries)
   - Validate boolean values (only "true", "false", "0", "1")
   - Validate version strings (semantic versioning format)

4. **JSON Validation**
   - Parse event JSON with schema validation
   - Reject malformed JSON
   - Validate required fields exist

### 10.2 Secrets Management

**Requirement:** Prevent secret exposure in logs and outputs.

**Specifications:**

1. **Input Secrets**
   - Never log GITHUB_TOKEN value
   - Never log GITLEAKS_LICENSE value
   - Mask secrets in error messages

2. **Detected Secrets**
   - Always use gitleaks `--redact` flag
   - SARIF output must not contain actual secret values
   - PR comments must not include secret values
   - Job summaries must not include secret values

3. **GitHub Actions Secret Masking**
   - Use `::add-mask::` for any secrets (if handled in JS wrapper)
   - Ensure WASM module doesn't log sensitive data

### 10.3 WASM Sandboxing

**Requirement:** Leverage WASM security model for isolation.

**Specifications:**

1. **Memory Isolation**
   - WASM runs in isolated linear memory
   - No access to process memory outside WASM heap
   - No buffer overflow vulnerabilities affecting host

2. **Capability-Based Security**
   - WASM cannot access file system directly
   - WASM cannot make network requests directly
   - WASM cannot spawn processes directly
   - All system interactions via explicit JavaScript bindings

3. **Control Flow Integrity**
   - WASM enforces type safety at module boundary
   - No arbitrary jumps or code execution
   - Stack overflow protection

### 10.4 Dependency Security

**Requirement:** Ensure all dependencies are secure and up-to-date.

**Specifications:**

1. **Vulnerability Scanning**
   - Run `cargo audit` in CI on every build
   - Fail build on high/critical vulnerabilities
   - Document accepted low/medium vulnerabilities

2. **Supply Chain Integrity**
   - Commit Cargo.lock to repository
   - Use `cargo verify-project` to check integrity
   - Pin major versions, allow minor/patch updates

3. **License Compliance**
   - Run `cargo deny` to check licenses
   - Ensure all dependencies use permissive licenses
   - Document any GPL/copyleft dependencies

4. **SBOM Generation**
   - Generate Software Bill of Materials (SBOM)
   - Include in releases for transparency
   - Use `cargo sbom` tool

### 10.5 Code Security

**Requirement:** Follow Rust security best practices.

**Specifications:**

1. **Unsafe Code**
   - Minimize use of `unsafe` blocks
   - Document all `unsafe` usage with safety proofs
   - Review all `unsafe` code in security audits

2. **Error Handling**
   - Never use `unwrap()` or `expect()` on external inputs
   - Use `Result` and `Option` types appropriately
   - Provide context in error messages (without leaking secrets)

3. **Integer Overflow**
   - Enable overflow checks in release builds (or accept risk)
   - Use checked arithmetic for critical calculations
   - Document assumptions about integer ranges

### 10.6 Self-Hosted Runner Security

**Requirement:** Provide guidance for secure self-hosted runner usage.

**Specifications:**

1. **Recommendations**
   - Use ephemeral runners (tear down after each job)
   - Isolate runners in separate VMs/containers
   - Limit network access from runners
   - Monitor for resource exhaustion attacks

2. **Warnings**
   - Document that WASM provides additional isolation
   - Warn against running untrusted PRs on persistent runners
   - Recommend GitHub-hosted runners for public repositories

---

## 11. DEPLOYMENT REQUIREMENTS

### 11.1 Distribution Model

**Requirement:** Distribute as GitHub Action with WASM core.

**Primary Distribution:**
- **Type:** JavaScript Action
- **Runtime:** Node.js 20 (migrating to Node.js 24 in fall 2025)
- **Entry Point:** `dist/index.js`
- **WASM Module:** `dist/secretscout_bg.wasm`
- **Glue Code:** `dist/secretscout.js` (wasm-bindgen generated)

**Alternative Distributions:**
- **Rust Crate:** Publish to crates.io for reuse in other Rust projects
- **npm Package:** Publish WASM module for standalone usage
- **Docker Image:** Optional, for users requiring containerization

### 11.2 Build Process

**Requirement:** Automate building and packaging for distribution.

**Build Steps:**
1. **Rust Compilation**
   ```bash
   wasm-pack build --target nodejs --out-dir dist --release
   ```

2. **JavaScript Wrapper**
   - Write `dist/index.js` wrapper
   - Parse action inputs
   - Load WASM module
   - Call WASM entry points
   - Handle errors and outputs

3. **Optimization**
   ```bash
   wasm-opt -Oz dist/secretscout_bg.wasm -o dist/secretscout_bg.wasm
   ```

4. **Verification**
   - Test WASM module loads
   - Test all event types
   - Test error handling
   - Test on all supported platforms

5. **Distribution**
   - Commit `dist/` directory to repository
   - Tag release (e.g., v3.0.0)
   - Create GitHub release with notes

### 11.3 Platform Support

**Requirement:** Support all GitHub-hosted runner platforms.

**Supported Platforms:**
- ✅ Ubuntu 22.04, 24.04 (linux/x64)
- ✅ macOS 13, 14 (darwin/x64, darwin/arm64)
- ✅ Windows Server 2022 (windows/x64)

**Node.js Versions:**
- ✅ Node.js 20 (current standard)
- ✅ Node.js 24 (future, fall 2025)

**WASM Compatibility:**
- ✅ All platforms via wasm32-unknown-unknown target
- ✅ No platform-specific code in WASM module
- ✅ Platform differences handled in JavaScript wrapper

### 11.4 Versioning Strategy

**Requirement:** Follow semantic versioning for releases.

**Version Format:** MAJOR.MINOR.PATCH

- **MAJOR:** Breaking changes (e.g., v2 → v3)
- **MINOR:** New features, backward compatible
- **PATCH:** Bug fixes, backward compatible

**Release Branches:**
- `main` - Latest stable release
- `v3` - Major version branch (for v3.x.x releases)
- `develop` - Development branch

**GitHub Action Pinning:**
- Recommend: `gitleaks/gitleaks-action@v3` (major version)
- Alternative: `gitleaks/gitleaks-action@v3.1.2` (exact version)
- Avoid: `gitleaks/gitleaks-action@main` (unpredictable)

### 11.5 Backward Compatibility

**Requirement:** Maintain compatibility with existing workflows.

**Specifications:**

1. **Environment Variables**
   - Support all existing environment variables
   - Maintain same default values
   - Support same boolean parsing logic

2. **Outputs**
   - Maintain same output format
   - Same exit codes (0, 1, 2)
   - Same SARIF structure
   - Same PR comment format
   - Same job summary format

3. **Behavior**
   - Same event type handling
   - Same scan strategies
   - Same error messages (where practical)

4. **Migration Path**
   - No workflow changes required for v2 → v3 upgrade
   - Document any breaking changes in release notes
   - Provide migration guide if needed

---

## 12. CONSTRAINTS AND LIMITATIONS

### 12.1 WASM-Specific Constraints

**Limitation:** WASM cannot directly access file system, network, or spawn processes.

**Mitigation:** Use JavaScript wrapper for:
- File I/O (reading event JSON, SARIF files)
- Network requests (GitHub API, Keygen API, gitleaks download)
- Process execution (running gitleaks binary)

**WASM Module Responsibilities:**
- Event data parsing and routing
- SARIF parsing and transformation
- Fingerprint generation
- PR comment content generation
- Job summary content generation
- Validation logic

### 12.2 GitHub Actions Constraints

**Limitation:** GitHub Actions has resource limits.

**Limits:**
- Job execution time: 6 hours (default), 360 hours (max with configuration)
- File size: 100 MB per artifact, 10 GB total per workflow
- API rate limits: 1,000 requests per hour (GITHUB_TOKEN)

**Mitigation:**
- Optimize scan times with incremental scanning
- Compress artifacts if large
- Implement exponential backoff for API calls
- Cache gitleaks binary to reduce download time

### 12.3 Gitleaks Dependency

**Limitation:** Depends on external gitleaks binary.

**Implications:**
- Gitleaks must be available for platform/architecture
- Gitleaks updates require action updates (if new features needed)
- Gitleaks bugs affect action functionality

**Mitigation:**
- Pin default version for stability
- Allow version override for flexibility
- Test with multiple gitleaks versions
- Document minimum gitleaks version requirement

### 12.4 License Validation (When Enabled)

**Limitation:** Depends on third-party Keygen.sh service.

**Implications:**
- Service downtime affects action availability
- Network latency affects execution time
- API changes require action updates

**Mitigation:**
- Currently disabled (avoids dependency)
- If re-enabled: Implement retry logic
- If re-enabled: Cache validation results (with expiration)
- If re-enabled: Provide fallback/bypass mechanism

### 12.5 API Rate Limits

**Limitation:** GitHub API and Keygen API have rate limits.

**GitHub API:**
- 1,000 requests/hour with GITHUB_TOKEN
- 5,000 requests/hour with personal access token

**Keygen API:**
- Undocumented limits (assume conservative)

**Mitigation:**
- Minimize API calls (batch operations)
- Implement exponential backoff on 429 errors
- Use conditional requests (ETags) where possible
- Document rate limit risks in README

---

## 13. SUCCESS CRITERIA

### 13.1 Functional Completeness

**Criteria:** All FR-1 through FR-9 requirements implemented and tested.

**Validation:**
- ✅ All 4 event types supported
- ✅ Binary download and execution working
- ✅ SARIF parsing accurate
- ✅ PR comments posted correctly
- ✅ Job summaries generated correctly
- ✅ Artifacts uploaded successfully
- ✅ License validation functional (when enabled)
- ✅ Environment variables parsed correctly
- ✅ Configuration files discovered and used

### 13.2 Performance Benchmarks

**Criteria:** Meet or exceed performance requirements.

**Validation:**
- ✅ Build time ≤ 2 minutes (cached)
- ✅ WASM binary size ≤ 500 KB
- ✅ Runtime overhead ≤ 2 seconds
- ✅ Memory usage ≤ 200 MB (peak)

### 13.3 Security Validation

**Criteria:** Pass all security requirements.

**Validation:**
- ✅ `cargo audit` passes (no high/critical vulnerabilities)
- ✅ Input validation prevents injection attacks
- ✅ Secrets never logged or exposed
- ✅ WASM sandboxing verified
- ✅ Dependencies reviewed and approved

### 13.4 Platform Compatibility

**Criteria:** Run successfully on all supported platforms.

**Validation:**
- ✅ Ubuntu 22.04: Integration tests pass
- ✅ Ubuntu 24.04: Integration tests pass
- ✅ macOS 13 (Intel): Integration tests pass
- ✅ macOS 14 (Apple Silicon): Integration tests pass
- ✅ Windows Server 2022: Integration tests pass

### 13.5 User Experience

**Criteria:** Easy to use and understand.

**Validation:**
- ✅ Clear error messages guide users to solutions
- ✅ Documentation covers common use cases
- ✅ Examples provided for all event types
- ✅ Migration guide from v2 to v3 available
- ✅ Changelog documents all changes

### 13.6 Backward Compatibility

**Criteria:** Existing workflows work without modification.

**Validation:**
- ✅ All v2 environment variables supported
- ✅ Output format unchanged
- ✅ Exit codes unchanged
- ✅ SARIF structure unchanged
- ✅ PR comment format unchanged

---

## 14. OUT OF SCOPE

### 14.1 Not Included in This Phase

The following are explicitly **NOT** included in the SPECIFICATION phase:

1. ❌ **Pseudocode** - Implementation algorithms and logic flows
2. ❌ **Architecture** - Detailed system design and component interactions
3. ❌ **Refinement** - Iterative improvements and optimizations
4. ❌ **Completion** - Final implementation, testing, and deployment

These phases are part of the full SPARC methodology but are excluded per project requirements.

### 14.2 Features Not Implemented

The following features are **NOT** in scope for the initial Rust port:

1. ❌ **Custom Secret Detection** - Gitleaks handles detection; action is a wrapper
2. ❌ **Alternative Binary Support** - Only gitleaks binary supported (not trufflehog, etc.)
3. ❌ **GUI/Web Interface** - Command-line/GitHub Actions only
4. ❌ **Real-Time Scanning** - Only triggered by GitHub events
5. ❌ **Multi-Repository Scanning** - One repository per action run
6. ❌ **Historical Reporting** - No database or persistent storage
7. ❌ **Advanced Analytics** - Basic detection and reporting only
8. ❌ **Custom Notification Channels** - GitHub comments/summaries only (no Slack, email, etc.)

### 14.3 Deferred Features

The following features may be considered in future versions:

1. 🔄 **License Validation** - Currently disabled; may re-enable if Keygen service restored
2. 🔄 **Custom Report Formats** - Currently SARIF only; could add JSON, CSV, etc.
3. 🔄 **Incremental Caching** - Currently scans full range; could optimize with result caching
4. 🔄 **Parallel Scanning** - Currently sequential; could parallelize for large repos
5. 🔄 **Advanced Filtering** - Currently basic; could add regex filters, severity levels, etc.

---

## APPENDICES

### A. Glossary

- **SARIF**: Static Analysis Results Interchange Format (JSON-based security report format)
- **WASM**: WebAssembly (portable binary instruction format)
- **PR**: Pull Request
- **SHA**: Secure Hash Algorithm (Git commit identifier)
- **Fingerprint**: Unique identifier for a detected secret
- **Crate**: Rust package (library or binary)
- **cdylib**: C-compatible dynamic library (for WASM)
- **rlib**: Rust library (for native compilation)

### B. References

1. **Original gitleaks-action**: https://github.com/gitleaks/gitleaks-action
2. **Gitleaks CLI**: https://github.com/zricethezav/gitleaks
3. **SARIF Specification**: https://docs.oasis-open.org/sarif/sarif/v2.1.0/sarif-v2.1.0.html
4. **GitHub Actions Documentation**: https://docs.github.com/en/actions
5. **wasm-bindgen Guide**: https://rustwasm.github.io/wasm-bindgen/
6. **Rust WASM Book**: https://rustwasm.github.io/docs/book/
7. **Keygen API**: https://keygen.sh/docs/api/

### C. Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-15 | Claude Code Swarm | Initial specification document |

---

**END OF SPECIFICATION PHASE**

This document completes the SPARC Specification phase for the gitleaks-action Rust port. No pseudocode, architecture, refinement, or completion phases are included per project requirements.

**Next Steps (Not Included in This Document):**
1. Review and approve specification
2. Proceed to implementation planning (if desired)
3. Set up Rust project structure
4. Begin development based on this specification

**Specification Status:** ✅ COMPLETE
