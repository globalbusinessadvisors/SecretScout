# SYSTEM ARCHITECTURE - SecretScout

**Project:** SecretScout - Rust/WASM Port of gitleaks-action
**Methodology:** SPARC (Specification, Pseudocode, Architecture, Refinement, Completion)
**Phase:** ARCHITECTURE
**Date:** October 16, 2025
**Version:** 1.0
**Status:** Ready for Review

---

## TABLE OF CONTENTS

1. [Executive Summary](#executive-summary)
2. [Architectural Overview](#architectural-overview)
3. [System Context](#system-context)
4. [Architectural Layers](#architectural-layers)
5. [Component Catalog](#component-catalog)
6. [Deployment Architecture](#deployment-architecture)
7. [Data Flow Architecture](#data-flow-architecture)
8. [Integration Architecture](#integration-architecture)
9. [Security Architecture](#security-architecture)
10. [Architectural Decisions](#architectural-decisions)
11. [Quality Attributes](#quality-attributes)
12. [Architectural Constraints](#architectural-constraints)

---

## EXECUTIVE SUMMARY

### Purpose

This document defines the high-level system architecture for SecretScout, a Rust-based reimplementation of gitleaks-action that compiles to both native binaries and WebAssembly (WASM). The architecture balances functional parity with the original Node.js implementation while leveraging Rust's performance, safety, and cross-platform capabilities.

### Key Architectural Characteristics

- **Hybrid Architecture**: JavaScript wrapper + WASM core + external process orchestration
- **Layered Design**: Presentation (GitHub Actions) → Application (WASM) → Infrastructure (JavaScript glue)
- **Event-Driven**: Responds to 4 GitHub event types with specialized handling
- **Pipeline Pattern**: Linear flow from event → scan → parse → report → exit
- **Sandboxed Execution**: WASM isolation with capability-based security

### Strategic Decisions

1. **WASM Core Logic**: All business logic in Rust/WASM for performance and safety
2. **JavaScript I/O Layer**: File system, network, and process operations in Node.js
3. **Stateless Execution**: No persistent state; each run is independent
4. **External Binary Dependency**: Delegate secret detection to gitleaks CLI
5. **GitHub Actions Native**: Optimized for GitHub's workflow environment

---

## ARCHITECTURAL OVERVIEW

### System Architecture Diagram

```
┌────────────────────────────────────────────────────────────────────────────┐
│                        GITHUB ACTIONS RUNTIME                               │
│                         (Node.js 20/24 Host)                                │
└───────────────────────────────┬────────────────────────────────────────────┘
                                │
                                │ action.yml
                                │ inputs/outputs
                                │
                    ┌───────────┴────────────┐
                    │  PRESENTATION LAYER    │
                    │   (JavaScript Wrapper) │
                    │     dist/index.js      │
                    │                        │
                    │ • Parse action inputs  │
                    │ • Load WASM module     │
                    │ • File I/O operations  │
                    │ • Process spawning     │
                    │ • HTTP requests        │
                    │ • Error marshalling    │
                    └───────────┬────────────┘
                                │
                                │ wasm-bindgen
                                │ FFI boundary
                                │
                    ┌───────────┴────────────┐
                    │   APPLICATION LAYER    │
                    │    (WASM Core Logic)   │
                    │  secretscout_bg.wasm   │
                    │                        │
                    │ ┌──────────────────┐   │
                    │ │ Event Router     │   │
                    │ └────────┬─────────┘   │
                    │          │             │
                    │    ┌─────┴─────┐       │
                    │    │           │       │
                    │ ┌──▼──┐     ┌──▼──┐    │
                    │ │SARIF│     │ PR  │    │
                    │ │Parse│     │Comm.│    │
                    │ └──┬──┘     └──┬──┘    │
                    │    │           │       │
                    │    └─────┬─────┘       │
                    │          │             │
                    │    ┌─────▼─────┐       │
                    │    │  Summary  │       │
                    │    │ Generator │       │
                    │    └───────────┘       │
                    └───────────┬────────────┘
                                │
                                │ async calls
                                │ (via JS callbacks)
                                │
        ┌───────────────────────┴────────────────────────┐
        │                                                 │
┌───────▼────────┐          ┌─────────▼─────────┐   ┌───▼──────┐
│  GITHUB API    │          │  GITLEAKS BINARY  │   │  FILE    │
│                │          │                   │   │  SYSTEM  │
│ • User lookup  │          │ • Secret scanning │   │          │
│ • PR commits   │          │ • SARIF output    │   │ • Config │
│ • PR comments  │          │ • Exit codes      │   │ • Events │
│ • Latest       │          │   0: Clean        │   │ • SARIF  │
│   releases     │          │   1: Error        │   │          │
└────────────────┘          │   2: Secrets      │   └──────────┘
                            └───────────────────┘
```

### Architecture Style Classification

**Primary Pattern**: **Pipes and Filters**
- Linear pipeline from input (event) to output (exit code)
- Each component transforms data and passes to next stage
- Clear separation of concerns between filters

**Secondary Pattern**: **Event-Driven**
- Different processing paths based on GitHub event type
- Asynchronous operations with callbacks
- Non-blocking I/O operations

**Tertiary Pattern**: **Layered**
- Clear separation between presentation, application, and infrastructure
- Dependencies flow downward (no circular dependencies)
- Each layer has distinct responsibilities

---

## SYSTEM CONTEXT

### Context Diagram

```
                                 ┌──────────────────┐
                                 │                  │
                                 │  GitHub Actions  │
                                 │     Platform     │
                                 │                  │
                                 └────────┬─────────┘
                                          │
                                          │ triggers workflow
                                          │
                        ┌─────────────────▼────────────────┐
                        │                                  │
                        │         SecretScout              │
                        │    (Rust/WASM Action)            │
                        │                                  │
                        └───┬─────────────┬────────────┬───┘
                            │             │            │
              ┌─────────────┘             │            └─────────────┐
              │                           │                          │
              │                           │                          │
    ┌─────────▼─────────┐      ┌─────────▼─────────┐     ┌─────────▼────────┐
    │                   │      │                   │     │                  │
    │   GitHub REST     │      │     Gitleaks      │     │  Repository      │
    │       API         │      │   Binary (CLI)    │     │  File System     │
    │                   │      │                   │     │                  │
    │ • User info       │      │ • Scan commits    │     │ • Source code    │
    │ • PR metadata     │      │ • Detect secrets  │     │ • Config files   │
    │ • Comments        │      │ • Generate SARIF  │     │ • Event JSON     │
    └───────────────────┘      └───────────────────┘     └──────────────────┘
```

### External Actors

1. **GitHub Actions Platform**
   - Provides: Node.js runtime, environment variables, workflow context
   - Consumes: Exit codes, job summaries, artifacts, log output
   - Interaction: Synchronous execution model

2. **GitHub REST API**
   - Provides: User metadata, PR commits, existing comments
   - Consumes: Review comments, artifact uploads
   - Interaction: Asynchronous HTTP with rate limiting

3. **Gitleaks Binary**
   - Provides: Secret detection, SARIF reports
   - Consumes: Git repository, configuration files, CLI arguments
   - Interaction: Synchronous process execution

4. **Repository File System**
   - Provides: Source code, configuration, event payloads
   - Consumes: SARIF output files
   - Interaction: Synchronous file I/O

### System Boundaries

**Inside the System:**
- Event routing and orchestration
- SARIF parsing and transformation
- PR comment content generation
- Job summary HTML generation
- Configuration management
- Binary download and caching
- Error handling and reporting

**Outside the System:**
- Secret detection algorithms (delegated to gitleaks)
- Git operations (delegated to gitleaks)
- GitHub Actions runtime (host environment)
- HTTP client implementation (Node.js fetch/reqwest)
- File system operations (Node.js fs module)

---

## ARCHITECTURAL LAYERS

### Layer 1: Presentation Layer (JavaScript)

**Technology**: Node.js 20/24, JavaScript ES2022
**Location**: `dist/index.js`
**Responsibilities**:
- Parse GitHub Actions inputs from `action.yml`
- Load and initialize WASM module
- Perform all file system operations (read event JSON, SARIF files)
- Execute HTTP requests (GitHub API, binary downloads)
- Spawn and manage child processes (gitleaks binary)
- Marshal data between JavaScript and WASM
- Write job summaries to `GITHUB_STEP_SUMMARY`
- Set action outputs via `core.setOutput()`
- Handle uncaught WASM errors
- Exit with appropriate status code

**Key Characteristics**:
- Stateless (no persistent state between calls)
- Thin layer (minimal business logic)
- Synchronous interface to GitHub Actions
- Asynchronous interface to WASM core

**Dependencies**:
- `@actions/core` - GitHub Actions toolkit
- `@actions/github` - GitHub API client
- `@actions/artifact` - Artifact upload
- `@actions/tool-cache` - Binary caching
- `secretscout.js` - WASM bindings (generated)

### Layer 2: Application Layer (WASM)

**Technology**: Rust 2021, wasm32-unknown-unknown target
**Location**: `dist/secretscout_bg.wasm`
**Responsibilities**:
- Route events to appropriate handlers (push, PR, workflow_dispatch, schedule)
- Parse and validate SARIF JSON structure
- Extract secrets, locations, and metadata from SARIF
- Generate fingerprint strings (`{commit}:{file}:{rule}:{line}`)
- Generate PR comment content (Markdown with structured data)
- Generate job summary HTML tables
- Validate configuration values
- Implement retry logic with exponential backoff
- Handle errors with detailed context

**Key Characteristics**:
- Pure functional core (no side effects)
- Memory-safe (Rust's ownership system)
- Sandboxed execution (WASM security model)
- Deterministic (same inputs → same outputs)
- Cross-platform (universal binary)

**Dependencies** (compiled into WASM):
- `serde` + `serde_json` - JSON parsing
- `wasm-bindgen` - JavaScript interop
- `thiserror` - Error types
- Minimal dependencies (size optimization)

### Layer 3: Infrastructure Layer (External Systems)

**Components**:

1. **GitHub API Client** (JavaScript + Rust types)
   - User account type lookup
   - PR commit fetching with pagination
   - PR comment posting with deduplication
   - Latest release queries
   - Rate limit tracking and backoff

2. **Binary Management** (JavaScript execution + Rust orchestration)
   - Version resolution (default, override, "latest")
   - Platform/architecture detection
   - Cache key generation
   - HTTP download with progress
   - Archive extraction (tar.gz, zip)
   - PATH modification
   - Process spawning with argument construction

3. **File System Abstraction** (JavaScript implementation)
   - Read event JSON payloads
   - Read SARIF output files
   - Write job summary files
   - Discover configuration files
   - Validate paths (security)

**Key Characteristics**:
- Platform-specific implementations
- Handles all I/O operations
- Manages external process lifecycle
- Implements retry and error recovery

---

## COMPONENT CATALOG

### Core Components

#### 1. Event Router (`event.rs` → WASM)

**Purpose**: Dispatch events to specialized handlers based on GitHub event type

**Responsibilities**:
- Parse `GITHUB_EVENT_NAME` environment variable
- Load event JSON from `GITHUB_EVENT_PATH`
- Validate event structure
- Route to: `HandlePushEvent()`, `HandlePullRequestEvent()`, `HandleWorkflowDispatchEvent()`, or `HandleScheduleEvent()`
- Return scan configuration (base ref, head ref, log-opts)

**Interfaces**:
- **Input**: Event type string, event JSON payload
- **Output**: `ScanConfiguration` struct or error

**Dependencies**: Configuration module, SARIF parser

**Key Algorithms**:
- Event type validation (enum matching)
- Commit range determination (first to last)
- BASE_REF override logic
- Single commit optimization (log-opts=-1)

**Error Handling**: Fail fast on unsupported event types or malformed JSON

---

#### 2. Binary Manager (`scanner.rs` → JavaScript + WASM orchestration)

**Purpose**: Download, cache, and execute gitleaks binary

**Responsibilities**:
- Resolve gitleaks version (default 8.24.3, override, or "latest" from API)
- Detect platform (linux, darwin, windows) and architecture (x64, arm64)
- Generate cache key: `gitleaks-cache-{version}-{platform}-{arch}`
- Download binary from GitHub releases if not cached
- Extract archive (platform-specific: tar.gz vs zip)
- Add binary to PATH
- Construct CLI arguments with event-specific log-opts
- Execute gitleaks as child process
- Capture stdout, stderr, and exit code
- Return execution results

**Interfaces**:
- **Input**: `ScanConfiguration`, gitleaks version
- **Output**: Exit code (0/1/2), SARIF file path

**Dependencies**: HTTP client (JavaScript), cache API (JavaScript), process spawner (JavaScript)

**Key Algorithms**:
- Cache hit/miss detection
- URL construction: `https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{platform}_{arch}.{ext}`
- Argument array construction based on event type
- Exit code interpretation

**Error Handling**:
- Cache failures → Download fresh (non-fatal)
- Download failures → Fatal error
- Extraction failures → Fatal error
- Gitleaks exit code 1 → Fatal error
- Gitleaks exit code 2 → Continue (secrets found)

---

#### 3. SARIF Parser (`sarif.rs` → WASM)

**Purpose**: Parse SARIF v2 JSON output from gitleaks into structured data

**Responsibilities**:
- Read SARIF file (via JavaScript callback)
- Deserialize JSON with serde
- Validate SARIF structure (runs[0].results[])
- Extract for each result:
  - `ruleId` (detection rule name)
  - `partialFingerprints` (commit, author, email, date)
  - `locations[0].physicalLocation` (file path, line number)
- Generate fingerprint string: `{commitSha}:{filePath}:{ruleId}:{startLine}`
- Return `Vec<DetectedSecret>`

**Interfaces**:
- **Input**: SARIF file path (JavaScript reads file)
- **Output**: `Vec<DetectedSecret>` or `SARIFParseError`

**Dependencies**: serde_json, file reader (JavaScript callback)

**Data Structures**:
```rust
struct DetectedSecret {
    rule_id: String,
    commit_sha: String,
    author: String,
    email: String,
    date: String,
    file_path: String,
    start_line: u32,
    fingerprint: String,
}
```

**Key Algorithms**:
- Null-safe JSON traversal
- Fingerprint generation
- Default value substitution for missing fields

**Error Handling**:
- File not found → Fatal error
- Invalid JSON → Fatal error
- Missing required fields → Use defaults where possible
- Empty results array → Valid (no secrets found)

---

#### 4. PR Comment Generator (`github.rs` → WASM + JavaScript)

**Purpose**: Create inline review comments on pull requests

**Responsibilities**:
- Generate comment body Markdown
- Determine comment placement (file, line, commit, side)
- Fetch existing PR comments via GitHub API (JavaScript)
- Build deduplication hash map
- Check for duplicate comments (same body, path, line)
- Post new comments via GitHub API (JavaScript)
- Handle API errors gracefully (non-fatal)
- Add @mentions if `GITLEAKS_NOTIFY_USER_LIST` set

**Interfaces**:
- **Input**: `Vec<DetectedSecret>`, PR number, GITHUB_TOKEN
- **Output**: Comment count (posted), errors (warnings)

**Dependencies**: GitHub API client (JavaScript), string utilities (WASM)

**Comment Format**:
```markdown
🛑 **Gitleaks detected:** {rule_id}

**Commit:** {short_sha}
**Fingerprint:** {commit}:{file}:{rule}:{line}

To ignore this secret, add the fingerprint to your .gitleaksignore file.

cc @user1, @user2
```

**Key Algorithms**:
- Markdown generation with escaping
- Deduplication map (HashMap<(body, path, line), bool>)
- Retry with exponential backoff for transient failures
- Rate limit tracking and pre-emptive backoff

**Error Handling**:
- API failures → Log warning, continue (secrets still in summary/artifacts)
- Rate limits → Retry with backoff
- 422 Large Diff → Skip comment, log info
- Line not in diff → Skip comment

---

#### 5. Job Summary Generator (`summary.rs` → WASM)

**Purpose**: Generate GitHub Actions job summary HTML/Markdown

**Responsibilities**:
- Determine summary type based on exit code:
  - Exit 0 → Success message
  - Exit 2 → HTML table with findings
  - Exit 1 → Error message
- Generate HTML table with columns: Rule ID, Commit, Secret URL, Line, Author, Date, Email, File
- Generate hyperlinks to repository (commit URLs, file URLs with line anchors)
- Escape HTML entities (XSS prevention)
- Return summary string for JavaScript to write to `GITHUB_STEP_SUMMARY`

**Interfaces**:
- **Input**: Exit code, `Vec<DetectedSecret>`, repository URL
- **Output**: Markdown/HTML string

**Dependencies**: String formatting utilities

**Summary Types**:

1. **Success**: `## No leaks detected ✅`
2. **Secrets**: HTML table with all findings
3. **Error**: `## ❌ Gitleaks exited with error. Exit code [1]`

**URL Patterns**:
- Commit: `{repo_url}/commit/{commitSha}`
- Secret: `{repo_url}/blob/{commitSha}/{filePath}#L{startLine}`
- File: `{repo_url}/blob/{commitSha}/{filePath}`

**Key Algorithms**:
- HTML table generation
- URL construction with encoding
- HTML entity escaping (prevent XSS)
- Conditional rendering based on exit code

**Error Handling**: Should never fail (defensive programming with fallbacks)

---

#### 6. Configuration Manager (`config.rs` → WASM)

**Purpose**: Load and validate all configuration from environment variables

**Responsibilities**:
- Parse 14 environment variables
- Validate required vs optional variables
- Implement special boolean parsing logic:
  - `"false"` → false
  - `"0"` → false
  - Everything else → true (including empty string)
- Discover configuration file:
  - Priority 1: `GITLEAKS_CONFIG` (explicit)
  - Priority 2: `gitleaks.toml` (auto-detect)
  - Priority 3: None (use gitleaks defaults)
- Validate paths (security checks)
- Return `Configuration` struct

**Interfaces**:
- **Input**: Environment variable map (from JavaScript)
- **Output**: `Configuration` struct or `ConfigurationError`

**Data Structures**:
```rust
struct Configuration {
    github_token: String,
    github_workspace: String,
    github_event_path: String,
    github_event_name: String,
    github_repository: String,
    github_repository_owner: String,
    gitleaks_version: String,
    gitleaks_config: Option<String>,
    gitleaks_license: Option<String>,
    enable_summary: bool,
    enable_upload_artifact: bool,
    enable_comments: bool,
    notify_user_list: Vec<String>,
    base_ref: Option<String>,
}
```

**Key Algorithms**:
- Boolean parsing with backward compatibility
- Config file discovery (priority order)
- Path validation (traversal prevention)
- Canonical path resolution

**Error Handling**:
- Missing required env var → Fatal error with clear message
- Invalid path → Fatal error
- Missing optional env var → Use default

---

#### 7. GitHub API Client (`github.rs` → JavaScript wrapper + Rust types)

**Purpose**: Interact with GitHub REST API

**Responsibilities**:
- Fetch user account type (`GET /users/{username}`)
- Fetch latest gitleaks release (`GET /repos/zricethezav/gitleaks/releases/latest`)
- Fetch PR commits with pagination (`GET /repos/{owner}/{repo}/pulls/{number}/commits`)
- Fetch PR comments (`GET /repos/{owner}/{repo}/pulls/{number}/comments`)
- Post PR comment (`POST /repos/{owner}/{repo}/pulls/{number}/comments`)
- Implement retry with exponential backoff (3 retries, 1s initial, 60s max)
- Track rate limits (workflow token: 1,000 req/hr)
- Respect `Retry-After` header

**Interfaces**:
- **Input**: Endpoint, method, body, GITHUB_TOKEN
- **Output**: Response body or `APIError`

**Dependencies**: HTTP client (JavaScript fetch or reqwest), retry logic (WASM)

**Retry Strategy**:
- Retry on: 429, 500, 502, 503, 504, network errors, timeouts
- Don't retry on: 401, 403, 404, 400 (bad input)
- Multiplier: 2.0 (exponential)
- Jitter: 10% (prevent thundering herd)

**Key Algorithms**:
- Exponential backoff with jitter
- Rate limit tracking (atomic counters)
- Response pagination (follow `Link` header)
- Error classification (retry vs fail)

**Error Handling**:
- Authentication failures → Fatal (401, 403)
- Not found → Context-dependent (404)
- Rate limits → Retry with backoff (429)
- Server errors → Retry with backoff (5xx)
- Network errors → Retry with backoff

---

### Supporting Components

#### 8. License Validator (`license.rs` → WASM + JavaScript)

**Purpose**: Validate Keygen.sh licenses for organization accounts

**Status**: Currently disabled, retained for future re-enablement

**Responsibilities**:
- Determine account type (Organization vs User)
- Skip validation for personal accounts
- Validate license via Keygen API for organizations
- Handle responses: VALID, TOO_MANY_MACHINES, NO_MACHINE
- Activate repository (associate with license)

**Interfaces**:
- **Input**: Repository owner, GITLEAKS_LICENSE
- **Output**: Validation result or error

**Key Algorithms**:
- Account type detection with fallback
- License validation with activation retry
- Fingerprint generation (owner/repo)

**Error Handling**:
- License validation failure → Fatal error
- API unavailable → Fatal error
- Missing license (organizations) → Fatal error

---

#### 9. Error Handler (`error.rs` → WASM)

**Purpose**: Centralized error handling and logging

**Responsibilities**:
- Define error types for all modules
- Implement error context propagation
- Format user-friendly error messages
- Determine fatal vs non-fatal errors
- Log errors to GitHub Actions (::error::, ::warning::)

**Error Categories**:
- **Fatal**: Exit immediately with code 1
  - Unsupported event type
  - Missing required configuration
  - SARIF parse failure
  - Gitleaks exit code 1
- **Non-Fatal**: Log warning, continue
  - PR comment failures
  - Cache failures
  - Latest release fetch failure

**Interfaces**:
- **Input**: Error type, context
- **Output**: Formatted error message, logging side effect

**Key Algorithms**:
- Error type enumeration (thiserror)
- Context propagation (anyhow)
- Log level determination
- Secret masking in error messages

---

## DEPLOYMENT ARCHITECTURE

### Deployment Model

**Type**: GitHub Action (JavaScript Action with WASM core)

**Deployment Units**:

1. **Action Metadata**: `action.yml`
   - Defines inputs, outputs, runtime (Node.js 20)
   - Specifies entry point (`dist/index.js`)

2. **JavaScript Wrapper**: `dist/index.js`
   - Bundled with dependencies (no node_modules in repository)
   - Includes WASM loader code
   - Platform-agnostic (Node.js handles platform differences)

3. **WASM Module**: `dist/secretscout_bg.wasm`
   - Compiled Rust code
   - Universal binary (works on all platforms)
   - Size: ~500KB (optimized)

4. **WASM Bindings**: `dist/secretscout.js`
   - Generated by wasm-bindgen
   - Provides JavaScript interface to WASM

**Distribution**:
- **Primary**: GitHub repository (git clone/checkout)
- **Versioning**: Git tags (v3.0.0, v3.1.0, etc.)
- **User Consumption**:
  ```yaml
  steps:
    - uses: gitleaks/gitleaks-action@v3
  ```

### Runtime Environment

**Host**: GitHub Actions Runner
- **OS**: Ubuntu 22.04, Ubuntu 24.04, macOS 13/14, Windows Server 2022
- **Runtime**: Node.js 20 (current), Node.js 24 (future)
- **Memory**: 7 GB available (use ~100 MB peak)
- **Disk**: Sufficient for gitleaks binary (~20 MB) + caching

**Execution Lifecycle**:
1. GitHub Actions clones action repository
2. Node.js executes `dist/index.js`
3. JavaScript loads WASM module
4. JavaScript calls WASM entry point with event data
5. WASM processes and returns results
6. JavaScript performs I/O (API calls, file writes, process spawn)
7. Process exits with appropriate code (0/1/2)

**Resource Requirements**:
- **CPU**: Minimal (most time spent in gitleaks binary)
- **Memory**: <200 MB peak (including Node.js runtime)
- **Disk**: <100 MB (gitleaks binary + cache)
- **Network**: <50 MB download (gitleaks binary, first run)

### Build and Release Pipeline

**Build Steps**:
1. Compile Rust to WASM:
   ```bash
   wasm-pack build --target nodejs --release
   ```
2. Optimize WASM:
   ```bash
   wasm-opt -Oz dist/secretscout_bg.wasm -o dist/secretscout_bg.wasm
   ```
3. Create JavaScript wrapper (`dist/index.js`)
4. Test on all platforms (Ubuntu, macOS, Windows)
5. Commit `dist/` directory to repository
6. Tag release (e.g., v3.0.0)
7. Create GitHub release with notes

**Release Artifacts**:
- Git tag (e.g., `v3.0.0`)
- GitHub release (changelog, notes)
- `dist/` directory in repository (ready to use)

**Versioning Strategy**:
- **Major**: v3 (breaking changes from v2)
- **Minor**: v3.1 (new features, backward compatible)
- **Patch**: v3.1.1 (bug fixes)

**Users Pin**:
- Recommended: `@v3` (major version, auto-updates)
- Conservative: `@v3.1.2` (exact version)
- Bleeding edge: `@main` (not recommended)

---

## DATA FLOW ARCHITECTURE

### Primary Data Flow (Success Case)

```
1. GitHub Event Trigger
   │
   │ Event JSON (push/PR/workflow_dispatch/schedule)
   │
   ▼
2. JavaScript Wrapper
   │
   │ Parse action.yml inputs
   │ Read environment variables
   │
   ▼
3. WASM: Configuration Manager
   │
   │ Configuration struct
   │
   ▼
4. WASM: Event Router
   │
   │ ScanConfiguration (base ref, head ref, log-opts)
   │
   ▼
5. JavaScript: Binary Manager
   │
   │ Download/cache gitleaks binary
   │ Construct CLI arguments
   │
   ▼
6. External Process: Gitleaks Binary
   │
   │ Execute scan
   │ Generate results.sarif
   │ Exit with code (0/2)
   │
   ▼
7. JavaScript: Read SARIF File
   │
   │ SARIF JSON string
   │
   ▼
8. WASM: SARIF Parser
   │
   │ Vec<DetectedSecret>
   │
   ▼
9. WASM: PR Comment Generator (if PR event)
   │
   │ Comment Markdown strings
   │
   ▼
10. JavaScript: GitHub API (post comments)
    │
    │ HTTP POST responses
    │
    ▼
11. WASM: Job Summary Generator
    │
    │ Summary HTML/Markdown
    │
    ▼
12. JavaScript: Write to GITHUB_STEP_SUMMARY
    │
    ▼
13. JavaScript: Set Action Outputs
    │
    ▼
14. Exit with code (0 = success, 1 = error, 2 = secrets found)
```

### Data Flow: Pull Request Event (Detailed)

```
┌─────────────────────────────────────────────────────────────────┐
│ 1. GitHub Triggers PR Event (opened, synchronize)               │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 2. JavaScript: Read event JSON from GITHUB_EVENT_PATH           │
│    {                                                             │
│      "pull_request": {                                           │
│        "number": 123,                                            │
│        "head": { "sha": "abc123" },                             │
│        "base": { "sha": "def456" }                              │
│      }                                                           │
│    }                                                             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 3. WASM: Event Router                                           │
│    - Detect event type: "pull_request"                          │
│    - Extract PR number: 123                                     │
│    - Call HandlePullRequestEvent()                              │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 4. JavaScript: GitHub API - Fetch PR Commits                    │
│    GET /repos/{owner}/{repo}/pulls/123/commits                  │
│    Response: [                                                   │
│      { "sha": "abc123", ... },                                  │
│      { "sha": "def456", ... }                                   │
│    ]                                                             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 5. WASM: Determine Scan Range                                   │
│    - base_ref = first commit SHA (abc123)                       │
│    - head_ref = last commit SHA (def456)                        │
│    - log_opts = "--no-merges --first-parent abc123^..def456"   │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 6. JavaScript: Execute Gitleaks                                 │
│    gitleaks detect --redact -v --exit-code=2 \                  │
│      --report-format=sarif --report-path=results.sarif \        │
│      --log-level=debug \                                        │
│      --log-opts="--no-merges --first-parent abc123^..def456"   │
│    Exit code: 2 (secrets found)                                 │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 7. WASM: Parse SARIF                                            │
│    [                                                             │
│      DetectedSecret {                                            │
│        rule_id: "aws-access-token",                             │
│        commit_sha: "abc123",                                    │
│        file_path: "src/config.js",                              │
│        start_line: 42,                                          │
│        fingerprint: "abc123:src/config.js:aws-access-token:42" │
│      }                                                           │
│    ]                                                             │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 8. JavaScript: GitHub API - Fetch Existing Comments             │
│    GET /repos/{owner}/{repo}/pulls/123/comments                 │
│    (For deduplication)                                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 9. WASM: Generate Comment Body                                  │
│    "🛑 **Gitleaks detected:** aws-access-token\n\n             │
│     **Commit:** abc123\n                                        │
│     **Fingerprint:** abc123:src/config.js:aws-access-token:42" │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 10. JavaScript: GitHub API - Post Comment                       │
│     POST /repos/{owner}/{repo}/pulls/123/comments               │
│     {                                                            │
│       "body": "🛑 **Gitleaks detected:** ...",                 │
│       "commit_id": "abc123",                                    │
│       "path": "src/config.js",                                  │
│       "line": 42,                                               │
│       "side": "RIGHT"                                           │
│     }                                                            │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 11. WASM: Generate Job Summary (HTML Table)                     │
│     ## 🛑 Gitleaks detected secrets 🛑                          │
│     <table>...</table>                                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 12. JavaScript: Write Summary + Upload Artifact                 │
│     - Write to GITHUB_STEP_SUMMARY                              │
│     - Upload results.sarif as artifact                          │
└────────────────────────┬────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────────────────┐
│ 13. Exit with code 1 (failure, because secrets were found)      │
└─────────────────────────────────────────────────────────────────┘
```

### Data Storage

**Persistent Storage**: None (stateless execution)

**Temporary Storage**:
- **SARIF File**: `{GITHUB_WORKSPACE}/results.sarif` (deleted after run)
- **Gitleaks Binary**: GitHub Actions cache (persistent across runs)
- **WASM Module**: Loaded into memory (not cached between runs)

**Cache Strategy**:
- **Gitleaks Binary**: Cache key = `gitleaks-cache-{version}-{platform}-{arch}`
- **TTL**: 7 days (GitHub Actions default)
- **Invalidation**: Version change, manual cache clear

---

## INTEGRATION ARCHITECTURE

### Integration Points

#### 1. GitHub Actions Platform Integration

**Integration Type**: Host runtime environment

**Interfaces**:
- **action.yml**: Metadata (inputs, outputs, runtime)
- **Environment Variables**: Configuration source
- **GITHUB_STEP_SUMMARY**: Job summary output
- **core.setOutput()**: Action outputs
- **Process Exit Code**: Success/failure signal

**Data Flow**:
- Actions platform → Environment variables → Application
- Application → GITHUB_STEP_SUMMARY file → Actions platform
- Application → Exit code → Actions platform

**Error Handling**:
- Uncaught exceptions → Exit code 1
- Exit code 0 → Success (green checkmark)
- Exit code 1 → Failure (red X)
- Exit code 2 → Treated as failure (secrets found)

---

#### 2. GitHub REST API Integration

**Integration Type**: RESTful HTTP API

**Endpoints**:
1. `GET /users/{username}` - Account type detection
2. `GET /repos/zricethezav/gitleaks/releases/latest` - Latest version
3. `GET /repos/{owner}/{repo}/pulls/{number}/commits` - PR commits
4. `GET /repos/{owner}/{repo}/pulls/{number}/comments` - PR comments
5. `POST /repos/{owner}/{repo}/pulls/{number}/comments` - Post comment

**Authentication**:
- Header: `Authorization: Bearer {GITHUB_TOKEN}`
- Token type: Workflow token (automatic) or PAT (manual)

**Rate Limiting**:
- Workflow token: 1,000 requests/hour
- Personal token: 5,000 requests/hour
- Strategy: Track requests, pre-emptive backoff when <100 remaining

**Retry Strategy**:
- Transient errors (429, 5xx): Exponential backoff (3 retries)
- Client errors (4xx): No retry
- Network errors: Retry with backoff

**Data Flow**:
- Application → HTTP client (JavaScript) → GitHub API
- GitHub API → HTTP response → Application

---

#### 3. Gitleaks Binary Integration

**Integration Type**: External process execution

**Execution Model**:
- Spawn child process
- Pass arguments via CLI
- Capture stdout, stderr
- Wait for exit code
- Read output file (results.sarif)

**Argument Construction**:
- Base: `detect --redact -v --exit-code=2 --report-format=sarif --report-path=results.sarif --log-level=debug`
- Event-specific: `--log-opts="..."`
- Optional: `--config={path}`

**Exit Code Mapping**:
- 0 → No secrets found (success)
- 1 → Error occurred (failure)
- 2 → Secrets found (process results, then failure)

**Data Flow**:
- Application → Process spawner (JavaScript) → Gitleaks binary
- Gitleaks binary → SARIF file → Application
- Gitleaks binary → Exit code → Application

---

#### 4. File System Integration

**Integration Type**: Synchronous file I/O (via JavaScript)

**Operations**:
- **Read**: Event JSON, SARIF files, configuration files
- **Write**: Job summary (GITHUB_STEP_SUMMARY), SARIF files
- **Discover**: Auto-detect gitleaks.toml

**Security**:
- Path validation (no `..` traversal)
- Constrain to GITHUB_WORKSPACE
- Verify file existence and permissions

**Data Flow**:
- Application → File reader (JavaScript) → File system
- File system → File contents → Application

---

### Integration Patterns

**Pattern 1: Request-Response (Synchronous)**
- Used for: File I/O, process execution
- Characteristics: Blocking, sequential
- Example: Read event JSON → Parse → Route

**Pattern 2: Async Request-Response**
- Used for: HTTP API calls
- Characteristics: Non-blocking, concurrent
- Example: Fetch PR commits while downloading binary

**Pattern 3: Callback (WASM ↔ JavaScript)**
- Used for: WASM requesting I/O operations
- Characteristics: Asynchronous, bidirectional
- Example: WASM requests GitHub API call → JavaScript executes → Returns response to WASM

**Pattern 4: Fire and Forget**
- Used for: Logging, artifact uploads
- Characteristics: Non-blocking, no response needed
- Example: Upload artifact to GitHub Actions

---

## SECURITY ARCHITECTURE

### Security Layers

#### Layer 1: WASM Sandboxing (Isolation)

**Threat Model**: Malicious input data, compromised dependencies

**Mitigation**:
- WASM runs in isolated linear memory (no access to process memory)
- No direct file system access (capability-based security)
- No direct network access (must call through JavaScript)
- No process spawning (must call through JavaScript)
- Stack overflow protection (WASM runtime)
- Type safety at module boundary (wasm-bindgen)

**Attack Surface Reduction**:
- WASM module cannot:
  - Read arbitrary files
  - Make arbitrary network requests
  - Execute arbitrary code
  - Access environment variables directly
  - Spawn child processes

**Limitations**:
- WASM does not protect against logic bugs
- WASM does not prevent denial of service (resource exhaustion)
- JavaScript layer is not sandboxed (Node.js permissions)

---

#### Layer 2: Input Validation (Defense in Depth)

**Threat Model**: Injection attacks, path traversal, malicious JSON

**Mitigation**:

1. **Path Validation**:
   ```rust
   fn validate_path(path: &str, workspace: &str) -> Result<PathBuf> {
       // Reject paths with ..
       if path.contains("..") {
           return Err(PathTraversalError);
       }
       // Resolve to canonical path
       let canonical = fs::canonicalize(path)?;
       // Ensure within workspace
       if !canonical.starts_with(workspace) {
           return Err(OutsideWorkspaceError);
       }
       Ok(canonical)
   }
   ```

2. **Git Reference Validation**:
   - Commit SHAs: Validate 40-character hex
   - Branch names: Reject shell metacharacters (`;`, `|`, `&`, etc.)
   - Sanitize before passing to gitleaks CLI

3. **JSON Validation**:
   - Schema validation with serde
   - Reject malformed JSON
   - Validate required fields exist
   - Use safe deserialization (no `unsafe`)

4. **Environment Variable Validation**:
   - Boolean: Only accept `"true"`, `"false"`, `"0"`, `"1"`
   - Version: Validate semantic versioning format
   - Repository: Validate `owner/repo` format

---

#### Layer 3: Secrets Management (Confidentiality)

**Threat Model**: Secret exposure in logs, outputs, error messages

**Mitigation**:

1. **Input Secrets**:
   - Never log GITHUB_TOKEN value
   - Never log GITLEAKS_LICENSE value
   - Mask in error messages: `Error: Invalid token '***'`

2. **Detected Secrets**:
   - Always use gitleaks `--redact` flag
   - SARIF output contains redacted values only
   - PR comments never include actual secret values
   - Job summaries never include actual secret values

3. **GitHub Actions Secret Masking**:
   - Use `::add-mask::` command if handling secrets in JavaScript
   - Ensure WASM module doesn't log sensitive data

**Secret Redaction Example**:
```
Detected: AWS_ACCESS_KEY_ID
Value (redacted): AKIA****************
Location: src/config.js:42
```

---

#### Layer 4: Dependency Security (Supply Chain)

**Threat Model**: Compromised dependencies, vulnerabilities

**Mitigation**:

1. **Vulnerability Scanning**:
   - Run `cargo audit` in CI on every build
   - Fail build on high/critical vulnerabilities
   - Document accepted low/medium vulnerabilities

2. **Supply Chain Integrity**:
   - Commit Cargo.lock to repository
   - Pin dependency versions
   - Use `cargo verify-project` to check integrity
   - Regularly update dependencies (Dependabot)

3. **License Compliance**:
   - Run `cargo deny` to check licenses
   - Ensure all dependencies use permissive licenses (MIT, Apache)
   - Document any copyleft dependencies

4. **Minimal Dependencies**:
   - Use `default-features = false` to avoid bloat
   - Avoid unnecessary dependencies
   - Prefer standard library over external crates

**Example Cargo.toml**:
```toml
[dependencies]
serde = { version = "1.0", default-features = false, features = ["derive"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
```

---

#### Layer 5: Authentication and Authorization

**Threat Model**: Unauthorized API access, token theft

**Mitigation**:

1. **GitHub Token Handling**:
   - Token stored in environment variable (GITHUB_TOKEN)
   - Never logged or printed
   - Automatically provided by GitHub Actions
   - Scoped to repository (no access to other repos)
   - Expires after job completion

2. **License Key Handling** (when enabled):
   - Stored in environment variable (GITLEAKS_LICENSE)
   - Never logged or printed
   - Validated via Keygen.sh API
   - Scoped to repository fingerprint

3. **API Authentication**:
   - All GitHub API calls include `Authorization: Bearer {token}`
   - Token validation by GitHub (not by action)
   - 401/403 errors → Fatal failure

---

#### Layer 6: Error Handling (Information Disclosure)

**Threat Model**: Error messages revealing sensitive information

**Mitigation**:

1. **Error Message Sanitization**:
   - Remove sensitive data from error messages
   - Use generic messages for external users
   - Detailed messages for debugging (in logs only)

2. **Stack Trace Filtering**:
   - Don't expose internal paths in production
   - Sanitize file paths (relative to workspace)

3. **Example**:
   ```rust
   // Bad: Exposes token
   Err(format!("API call failed with token {}", token))

   // Good: Masks token
   Err(format!("API call failed with token '***'"))
   ```

---

### Security Checklist

**Input Validation**:
- [ ] All file paths validated (no `..` traversal)
- [ ] All git references validated (no shell injection)
- [ ] All JSON validated with schema
- [ ] All environment variables validated

**Secrets Management**:
- [ ] GITHUB_TOKEN never logged
- [ ] GITLEAKS_LICENSE never logged
- [ ] Detected secrets always redacted (gitleaks `--redact`)
- [ ] Error messages sanitized

**Dependency Security**:
- [ ] `cargo audit` passes (no high/critical vulns)
- [ ] `cargo deny` passes (license compliance)
- [ ] Cargo.lock committed to repository
- [ ] Dependencies use `default-features = false`

**Authentication**:
- [ ] All GitHub API calls authenticated
- [ ] Tokens scoped appropriately
- [ ] 401/403 errors handled (fatal)

**Error Handling**:
- [ ] Error messages sanitized
- [ ] Stack traces filtered
- [ ] No information disclosure

---

## ARCHITECTURAL DECISIONS

### ADR-001: Hybrid JavaScript + WASM Architecture

**Decision**: Use JavaScript for I/O operations and WASM for business logic

**Rationale**:
- **WASM Strengths**: Memory safety, performance, cross-platform, sandboxing
- **WASM Limitations**: No file I/O, no network, no process spawning
- **JavaScript Strengths**: Native GitHub Actions support, I/O operations, ecosystem
- **JavaScript Limitations**: Memory safety, performance

**Alternatives Considered**:
1. **Pure JavaScript**: Simple but loses Rust benefits (safety, performance)
2. **Pure Rust Native**: Requires cross-compilation for all platforms
3. **Docker Container**: Slower startup, larger distribution size

**Trade-offs**:
- **Pros**: Best of both worlds, universal binary (WASM), sandboxed execution
- **Cons**: FFI boundary overhead, complexity of two languages

**Status**: Accepted

---

### ADR-002: External Gitleaks Binary Dependency

**Decision**: Delegate secret detection to external gitleaks binary instead of reimplementing

**Rationale**:
- Gitleaks is mature, well-tested, and actively maintained
- Reimplementing secret detection would be significant effort
- Gitleaks team is expert in this domain
- Allows independent evolution of detection rules

**Alternatives Considered**:
1. **Reimplement in Rust**: High effort, duplication, maintenance burden
2. **Embed gitleaks as library**: Not feasible (different languages)

**Trade-offs**:
- **Pros**: Leverage existing expertise, faster development, lower maintenance
- **Cons**: External dependency, download overhead, version compatibility

**Mitigation**:
- Cache gitleaks binary (reduces download overhead)
- Pin default version (stability)
- Allow version override (flexibility)

**Status**: Accepted

---

### ADR-003: Stateless Execution Model

**Decision**: No persistent state between runs; each execution is independent

**Rationale**:
- GitHub Actions model is stateless (each job is isolated)
- Simplifies implementation (no database, no state management)
- Easier to reason about (deterministic)
- No cache invalidation issues

**Alternatives Considered**:
1. **Persistent Database**: Store historical results for deduplication
2. **File-Based Cache**: Store scan results between runs

**Trade-offs**:
- **Pros**: Simple, deterministic, no cache invalidation bugs
- **Cons**: Cannot deduplicate across runs, re-scan on every trigger

**Status**: Accepted

---

### ADR-004: Event-Driven Architecture with Specialized Handlers

**Decision**: Route events to specialized handlers based on event type (push, PR, workflow_dispatch, schedule)

**Rationale**:
- Each event type has different requirements (PR needs comments, push doesn't)
- Specialized handlers simplify logic (no complex conditionals)
- Clear separation of concerns
- Easier to test (mock one event type at a time)

**Alternatives Considered**:
1. **Generic Handler**: Single handler with conditionals for event type
2. **Polymorphic Handlers**: Abstract base class with overrides

**Trade-offs**:
- **Pros**: Clear, maintainable, testable
- **Cons**: Some code duplication across handlers

**Status**: Accepted

---

### ADR-005: SARIF as Interchange Format

**Decision**: Use SARIF v2 as the data format for secret detection results

**Rationale**:
- Gitleaks native output format
- Industry standard for security findings (OASIS)
- Rich metadata (locations, fingerprints, rules)
- Supported by GitHub Code Scanning

**Alternatives Considered**:
1. **Custom JSON Format**: Would require gitleaks modification
2. **Plain Text**: Difficult to parse, no structure

**Trade-offs**:
- **Pros**: Standard format, rich metadata, GitHub integration
- **Cons**: Complex schema (deep nesting)

**Status**: Accepted

---

### ADR-006: Inline PR Comments for User Feedback

**Decision**: Post inline review comments on pull requests at exact secret locations

**Rationale**:
- Best user experience (see error at exact location)
- Actionable feedback (fingerprint for .gitleaksignore)
- Aligns with GitHub code review workflow
- Prevents secrets from being merged

**Alternatives Considered**:
1. **Job Summary Only**: Less visibility, no direct feedback
2. **PR Description Comment**: Not line-specific, easy to miss

**Trade-offs**:
- **Pros**: Excellent UX, prevents merges, actionable
- **Cons**: API rate limits, large diff errors (HTTP 422)

**Mitigation**:
- Deduplication prevents spam
- Non-fatal errors (secrets still in summary/artifacts)
- Rate limit tracking and backoff

**Status**: Accepted

---

### ADR-007: Exponential Backoff for API Retries

**Decision**: Implement exponential backoff with jitter for transient API failures

**Rationale**:
- Transient failures are common (network blips, rate limits)
- Exponential backoff prevents overwhelming the server
- Jitter prevents thundering herd (synchronized retries)
- Industry best practice

**Configuration**:
- Default: 3 retries
- Initial delay: 1 second
- Multiplier: 2.0 (exponential)
- Max delay: 60 seconds
- Jitter: 10%

**Alternatives Considered**:
1. **Linear Backoff**: Less effective, can still overwhelm server
2. **No Retries**: Brittle, fails on transient errors

**Trade-offs**:
- **Pros**: Resilient, self-healing, best practice
- **Cons**: Increased latency on failures

**Status**: Accepted

---

### ADR-008: Size Optimization for WASM Binary

**Decision**: Aggressively optimize WASM binary size (target: <500 KB)

**Rationale**:
- Faster distribution (smaller git clone)
- Faster load time (network transfer, parsing)
- Better user experience (responsive)
- Enables npm distribution (size limits)

**Optimization Techniques**:
- `opt-level = 'z'` (optimize for size)
- `lto = true` (link-time optimization)
- `codegen-units = 1` (maximum optimization)
- `strip = true` (remove debug symbols)
- `wasm-opt -Oz` (post-process optimization)
- `default-features = false` (minimal dependencies)

**Alternatives Considered**:
1. **Optimize for Speed**: Larger binary (1-2 MB)
2. **No Optimization**: Debug build (5-10 MB)

**Trade-offs**:
- **Pros**: Small binary, fast distribution, better UX
- **Cons**: Longer build times, harder debugging (in release mode)

**Status**: Accepted

---

### ADR-009: Backward Compatibility with v2

**Decision**: Maintain 100% backward compatibility with gitleaks-action v2.x

**Rationale**:
- Ease migration (no workflow changes required)
- User trust (predictable behavior)
- Drop-in replacement (users can switch with confidence)
- Avoid ecosystem fragmentation

**Compatibility Requirements**:
- Same environment variables
- Same boolean parsing logic (quirks and all)
- Same output formats (SARIF, comments, summaries)
- Same exit codes (0, 1, 2)
- Same error messages (where practical)

**Alternatives Considered**:
1. **Clean Slate**: Redesign API, breaking changes
2. **Partial Compatibility**: Support new API, deprecate old

**Trade-offs**:
- **Pros**: Easy migration, user trust, no fragmentation
- **Cons**: Inherits v2 quirks (e.g., boolean parsing)

**Status**: Accepted

---

### ADR-010: License Validation Feature Toggle

**Decision**: Implement license validation but keep it disabled by default (feature flag)

**Rationale**:
- Original implementation had license validation (via Keygen.sh)
- Currently disabled due to payment processing issues
- Retain code for future re-enablement
- No impact on users (disabled)

**Status**: Accepted (disabled)

**Future Considerations**:
- Re-enable if Keygen service restored
- Provide fallback mechanism (grace period)
- Document clearly in user-facing docs

---

## QUALITY ATTRIBUTES

### Performance

**Build Performance**:
- **Target**: Cold build ≤5 min, cached build ≤2 min
- **Strategy**: Rust cache (Swatinem/rust-cache), sccache, parallel compilation
- **Measurement**: CI pipeline duration

**Runtime Performance**:
- **Target**: Total overhead ≤2 seconds (excluding gitleaks scan)
- **Breakdown**:
  - WASM load: ≤50ms
  - Event parsing: ≤10ms
  - SARIF parsing: ≤100ms (10 findings)
  - GitHub API: ≤500ms per request
- **Measurement**: Instrumented logging, benchmarks

**Binary Size**:
- **Target**: WASM ≤500 KB (uncompressed), ≤200 KB (gzip)
- **Strategy**: `opt-level='z'`, LTO, `wasm-opt -Oz`, minimal dependencies
- **Measurement**: File size after build

**Memory Usage**:
- **Target**: Peak ≤200 MB (including Node.js runtime)
- **Strategy**: Streaming JSON parsing, bounded collections, no memory leaks
- **Measurement**: Process memory monitoring

---

### Reliability

**Availability**:
- **Target**: 99.9% success rate (excluding secrets found)
- **Strategy**: Retry with backoff, graceful degradation, comprehensive error handling
- **Measurement**: Success rate in CI/CD pipelines

**Fault Tolerance**:
- **Transient Failures**: Retry with exponential backoff (API, network)
- **Non-Critical Failures**: Log warning, continue (PR comments, cache)
- **Critical Failures**: Fail fast with clear error message (authentication, SARIF parse)

**Error Recovery**:
- Cache failures → Download fresh
- API failures (non-critical) → Continue (secrets still in summary)
- Large diff errors → Skip comment, log info

---

### Security

**Confidentiality**:
- Secrets never logged or exposed
- Tokens masked in error messages
- WASM sandboxing prevents unauthorized access

**Integrity**:
- Input validation prevents injection attacks
- Dependency pinning prevents supply chain attacks
- Cargo.lock ensures reproducible builds

**Availability**:
- Rate limit tracking prevents exhaustion
- Resource limits prevent denial of service
- Retry logic prevents transient failures

---

### Maintainability

**Modularity**:
- Clear separation of concerns (event routing, SARIF parsing, etc.)
- Each module has single responsibility
- Minimal coupling between modules

**Testability**:
- Pure functions (no side effects) easy to test
- Mocking interfaces for external dependencies
- 70+ unit tests, 25+ integration tests

**Documentation**:
- Comprehensive pseudocode (12,674 lines)
- Inline code comments
- API documentation (rustdoc)
- User-facing README

---

### Scalability

**Horizontal Scalability**: N/A (each run is independent, no shared state)

**Vertical Scalability**:
- Memory usage scales with SARIF size (streaming parsing mitigates)
- GitHub API rate limits are primary constraint (1,000 req/hr)
- Large repositories → Longer gitleaks scan (not action overhead)

---

### Compatibility

**Platform Compatibility**:
- Ubuntu 22.04, 24.04 (linux/x64)
- macOS 13, 14 (darwin/x64, darwin/arm64)
- Windows Server 2022 (windows/x64)

**Runtime Compatibility**:
- Node.js 20 (current)
- Node.js 24 (future, fall 2025)

**Backward Compatibility**:
- 100% compatible with gitleaks-action v2.x
- Same inputs, outputs, behavior

---

## ARCHITECTURAL CONSTRAINTS

### Technical Constraints

1. **WASM Cannot Perform I/O**:
   - Limitation: No file system, network, or process access
   - Mitigation: JavaScript layer handles all I/O

2. **GitHub Actions Runtime**:
   - Constraint: Node.js 20/24 only
   - Mitigation: Target `nodejs` for wasm-pack

3. **GitHub API Rate Limits**:
   - Constraint: 1,000 requests/hour (workflow token)
   - Mitigation: Minimize API calls, track rate limits, backoff

4. **External Gitleaks Dependency**:
   - Constraint: Requires gitleaks binary for scanning
   - Mitigation: Download and cache binary, allow version override

---

### Organizational Constraints

1. **Backward Compatibility**:
   - Constraint: Must maintain v2.x compatibility
   - Mitigation: Preserve all interfaces, quirks, and behaviors

2. **Open Source License**:
   - Constraint: All dependencies must be permissively licensed
   - Mitigation: Use MIT/Apache licenses, avoid GPL

---

### Business Constraints

1. **License Validation Disabled**:
   - Constraint: Keygen service payment issues
   - Mitigation: Feature toggle (disabled), retain code for future

2. **No Persistent Storage**:
   - Constraint: GitHub Actions is stateless
   - Mitigation: Stateless architecture, no database

---

## CONCLUSION

The SecretScout architecture is a **hybrid JavaScript + WASM design** that leverages the strengths of both technologies:

- **JavaScript** handles I/O operations (file system, network, process spawning)
- **WASM** handles business logic (event routing, SARIF parsing, comment generation)
- **Gitleaks binary** handles secret detection (external dependency)

This architecture achieves:

1. **Functional Parity**: 100% compatibility with gitleaks-action v2.x
2. **Performance**: <2 second overhead, <500 KB binary size
3. **Security**: WASM sandboxing, input validation, secrets management
4. **Reliability**: Retry logic, graceful degradation, comprehensive error handling
5. **Maintainability**: Modular design, extensive documentation, testability

The architecture is **implementation-ready** and provides a solid foundation for the Rust implementation phase.

---

**Architecture Phase Status**: ✅ **COMPLETE**

**Document Version**: 1.0
**Date**: October 16, 2025
**Architect**: Lead System Architect (Claude Code)
**Review Status**: Ready for approval

**Next Steps** (not included in this phase):
1. Review and approve architecture
2. Proceed to implementation (Refinement phase)
3. Set up Rust project structure based on architecture
4. Implement components following architectural guidelines
