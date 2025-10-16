# WASM BOUNDARY ARCHITECTURE

**Project:** SecretScout - Rust Port of gitleaks-action
**Phase:** Architecture (SPARC Methodology)
**Component:** WASM-JavaScript Boundary Interface
**Date:** October 16, 2025
**Version:** 1.0

---

## TABLE OF CONTENTS

1. [Executive Summary](#1-executive-summary)
2. [Boundary Design Principles](#2-boundary-design-principles)
3. [WASM Module Exports](#3-wasm-module-exports)
4. [JavaScript Wrapper Design](#4-javascript-wrapper-design)
5. [Data Serialization](#5-data-serialization)
6. [Error Handling Strategy](#6-error-handling-strategy)
7. [Memory Management](#7-memory-management)
8. [Performance Optimization](#8-performance-optimization)
9. [Implementation Patterns](#9-implementation-patterns)
10. [Security Considerations](#10-security-considerations)

---

## 1. EXECUTIVE SUMMARY

### 1.1 Purpose

This document defines the interface between the WASM module (compiled from Rust) and the JavaScript wrapper that bridges to the GitHub Actions runtime. The boundary is critical because WASM cannot directly access:

- File system (reading event JSON, SARIF files, writing summaries)
- Network (GitHub API, downloading gitleaks binary)
- Process execution (spawning gitleaks binary)
- Environment variables (GitHub Actions context)

### 1.2 Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│         GitHub Actions Runtime (Node.js 20/24)          │
│  - Environment Variables (GITHUB_*, GITLEAKS_*)         │
│  - File System Access                                   │
│  - Network Access                                       │
│  - Process Spawning                                     │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ JavaScript API
                 ▼
┌─────────────────────────────────────────────────────────┐
│              JavaScript Wrapper (index.js)              │
│  ┌───────────────────────────────────────────────────┐  │
│  │ Responsibilities:                                 │  │
│  │ • Parse action inputs → Config object             │  │
│  │ • Read event JSON file → EventData               │  │
│  │ • Download & execute gitleaks binary             │  │
│  │ • Fetch GitHub API (commits, comments)           │  │
│  │ • Read SARIF file → parsed JSON                  │  │
│  │ • Write job summary file                         │  │
│  │ • Set action outputs                             │  │
│  │ • Upload artifacts                               │  │
│  └───────────────────────────────────────────────────┘  │
└────────────────┬────────────────────────────────────────┘
                 │
                 │ wasm-bindgen FFI
                 │ (Serialized JSON)
                 ▼
┌─────────────────────────────────────────────────────────┐
│               WASM Module (secretscout.wasm)            │
│  ┌───────────────────────────────────────────────────┐  │
│  │ Responsibilities:                                 │  │
│  │ • Parse and validate configuration               │  │
│  │ • Route event types to scan strategies           │  │
│  │ • Build gitleaks command arguments               │  │
│  │ • Parse SARIF report structure                   │  │
│  │ • Extract findings and generate fingerprints     │  │
│  │ • Build PR comment content (deduplication)       │  │
│  │ • Generate job summary content (HTML tables)     │  │
│  │ • Validate inputs (security)                     │  │
│  │ • Pure computation, no I/O                       │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### 1.3 Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| **Single Entry Point** | One main function simplifies state management and error handling |
| **JSON Serialization** | Universal format, wasm-bindgen has excellent support, easy to debug |
| **Callback Pattern** | JavaScript provides callbacks for I/O operations (read file, exec process, etc.) |
| **Stateless WASM** | Each invocation is independent, no persistent state across calls |
| **Error as JSON** | Structured error objects with codes enable proper handling in JS |

---

## 2. BOUNDARY DESIGN PRINCIPLES

### 2.1 Separation of Concerns

**WASM Responsibilities (Pure Computation):**
- Configuration parsing and validation
- Event routing logic
- SARIF parsing and transformation
- Fingerprint generation algorithms
- PR comment content generation
- Job summary HTML generation
- Input sanitization and security validation

**JavaScript Responsibilities (I/O Operations):**
- Environment variable access
- File system operations (read/write)
- Network requests (HTTP, GitHub API)
- Process execution (gitleaks binary)
- GitHub Actions SDK interactions
- Artifact upload
- Cache operations

### 2.2 Data Flow Direction

```
JavaScript → WASM:
  • Configuration (environment variables as JSON)
  • Event data (file contents as JSON)
  • SARIF report (file contents as JSON)
  • GitHub API responses (commits, comments as JSON)

WASM → JavaScript:
  • Commands (exec gitleaks, fetch URL, read file)
  • Results (findings, comments, summary HTML)
  • Errors (structured error objects)
  • Exit code (0, 1, 2)
```

### 2.3 Error Handling Philosophy

1. **WASM Never Panics**: All errors are caught and returned as structured objects
2. **JavaScript Interprets Errors**: Error codes determine fatal vs non-fatal
3. **Graceful Degradation**: Non-critical failures (e.g., PR comments) log warnings but continue
4. **Clear Error Context**: Errors include actionable messages for users

---

## 3. WASM MODULE EXPORTS

### 3.1 Primary Entry Point

The WASM module exports a single main function that orchestrates the entire workflow:

```rust
#[wasm_bindgen]
pub async fn run_action(
    config_json: &str,
    callbacks: JsValue
) -> Result<JsValue, JsValue>
```

**Parameters:**
- `config_json`: JSON string containing all configuration and environment variables
- `callbacks`: JavaScript object with I/O callback functions

**Returns:**
- `Ok(JsValue)`: JSON object with `{ exitCode: number, outputs: {...} }`
- `Err(JsValue)`: JSON object with `{ error: string, code: string, context: {...} }`

### 3.2 Configuration Input Structure

```typescript
interface WasmConfig {
  // GitHub Actions Context
  github_token: string;
  github_workspace: string;
  github_repository: string;
  github_repository_owner: string;
  github_event_name: string;
  github_event_path: string;
  github_step_summary?: string;

  // Gitleaks Configuration
  gitleaks_license?: string;
  gitleaks_version: string;  // Default: "8.24.3"
  gitleaks_config?: string;  // Path or undefined

  // Feature Flags
  enable_summary: boolean;
  enable_upload_artifact: boolean;
  enable_comments: boolean;

  // Optional Settings
  notify_user_list: string[];  // e.g., ["@user1", "@user2"]
  base_ref?: string;  // Override base git ref
}
```

### 3.3 JavaScript Callback Interface

The WASM module expects JavaScript to provide these I/O callbacks:

```typescript
interface WasmCallbacks {
  // File Operations
  readFile(path: string): Promise<string>;
  writeFile(path: string, content: string): Promise<void>;
  fileExists(path: string): Promise<boolean>;

  // Process Execution
  execGitleaks(args: string[]): Promise<{
    exitCode: number;
    stdout: string;
    stderr: string;
  }>;

  // Network Operations
  httpGet(url: string, headers?: Record<string, string>): Promise<{
    statusCode: number;
    body: string;
  }>;

  httpPost(url: string, body: string, headers?: Record<string, string>): Promise<{
    statusCode: number;
    body: string;
  }>;

  // GitHub API Operations
  fetchPrCommits(prNumber: number): Promise<Commit[]>;
  fetchPrComments(prNumber: number): Promise<Comment[]>;
  postPrComment(prNumber: number, comment: PrComment): Promise<void>;

  // Logging
  logInfo(message: string): void;
  logWarning(message: string): void;
  logError(message: string): void;
  logDebug(message: string): void;

  // Cache Operations
  checkCache(key: string): Promise<string | null>;
  saveCache(key: string, path: string): Promise<void>;
}
```

### 3.4 Output Structure

```typescript
interface WasmOutput {
  exitCode: 0 | 1 | 2;  // 0=success, 1=error, 2=secrets found
  outputs: {
    exitCode: number;  // For GitHub Actions output
  };
  summary?: string;  // HTML/Markdown summary content
  artifactReady: boolean;  // True if SARIF should be uploaded
  errors?: string[];  // Non-fatal errors encountered
}
```

### 3.5 Error Structure

```typescript
interface WasmError {
  error: string;  // Human-readable error message
  code: string;   // Machine-readable error code
  severity: "fatal" | "warning";  // Error severity
  context: Record<string, any>;  // Additional context
}

// Error Codes
enum ErrorCode {
  CONFIG_INVALID = "CONFIG_INVALID",
  EVENT_PARSE_ERROR = "EVENT_PARSE_ERROR",
  UNSUPPORTED_EVENT = "UNSUPPORTED_EVENT",
  SARIF_PARSE_ERROR = "SARIF_PARSE_ERROR",
  GITHUB_API_ERROR = "GITHUB_API_ERROR",
  GITLEAKS_ERROR = "GITLEAKS_ERROR",
  LICENSE_REQUIRED = "LICENSE_REQUIRED",
  LICENSE_INVALID = "LICENSE_INVALID",
  INTERNAL_ERROR = "INTERNAL_ERROR"
}
```

---

## 4. JAVASCRIPT WRAPPER DESIGN

### 4.1 Entry Point (dist/index.js)

```javascript
const core = require('@actions/core');
const github = require('@actions/github');
const exec = require('@actions/exec');
const cache = require('@actions/cache');
const artifact = require('@actions/artifact');
const fs = require('fs').promises;
const path = require('path');
const wasm = require('./secretscout.js');

async function main() {
  try {
    // Load WASM module
    await wasm.default();

    // Build configuration from action inputs and environment
    const config = buildConfig();

    // Create callback object with I/O implementations
    const callbacks = createCallbacks();

    // Execute WASM action
    const result = await wasm.run_action(
      JSON.stringify(config),
      callbacks
    );

    // Handle result
    const output = JSON.parse(result);

    // Write summary if generated
    if (output.summary && process.env.GITHUB_STEP_SUMMARY) {
      await fs.appendFile(process.env.GITHUB_STEP_SUMMARY, output.summary);
    }

    // Upload artifact if ready
    if (output.artifactReady && config.enable_upload_artifact) {
      const artifactClient = artifact.create();
      await artifactClient.uploadArtifact(
        'gitleaks-results.sarif',
        [path.join(config.github_workspace, 'results.sarif')],
        config.github_workspace
      );
    }

    // Set outputs
    core.setOutput('exit-code', output.exitCode);

    // Exit with appropriate code
    process.exit(output.exitCode === 0 ? 0 : 1);

  } catch (error) {
    // Handle WASM errors
    if (error && typeof error === 'object' && error.error) {
      core.error(`${error.error} [${error.code}]`);
      if (error.context) {
        core.debug(`Context: ${JSON.stringify(error.context)}`);
      }
    } else {
      core.error(`Unexpected error: ${error.message || error}`);
    }
    process.exit(1);
  }
}

main();
```

### 4.2 Configuration Builder

```javascript
function buildConfig() {
  return {
    // GitHub Actions context
    github_token: process.env.GITHUB_TOKEN || core.getInput('token') || '',
    github_workspace: process.env.GITHUB_WORKSPACE,
    github_repository: process.env.GITHUB_REPOSITORY,
    github_repository_owner: process.env.GITHUB_REPOSITORY_OWNER,
    github_event_name: process.env.GITHUB_EVENT_NAME,
    github_event_path: process.env.GITHUB_EVENT_PATH,
    github_step_summary: process.env.GITHUB_STEP_SUMMARY,

    // Gitleaks configuration
    gitleaks_license: process.env.GITLEAKS_LICENSE,
    gitleaks_version: core.getInput('version') || process.env.GITLEAKS_VERSION || '8.24.3',
    gitleaks_config: core.getInput('config-path') || process.env.GITLEAKS_CONFIG,

    // Feature flags
    enable_summary: parseBool(core.getInput('enable-summary') || process.env.GITLEAKS_ENABLE_SUMMARY, true),
    enable_upload_artifact: parseBool(core.getInput('enable-upload-artifact') || process.env.GITLEAKS_ENABLE_UPLOAD_ARTIFACT, true),
    enable_comments: parseBool(core.getInput('enable-comments') || process.env.GITLEAKS_ENABLE_COMMENTS, true),

    // Optional settings
    notify_user_list: parseUserList(core.getInput('notify-user-list') || process.env.GITLEAKS_NOTIFY_USER_LIST),
    base_ref: process.env.BASE_REF
  };
}

function parseBool(value, defaultValue) {
  if (!value) return defaultValue;
  return value !== 'false' && value !== '0';
}

function parseUserList(value) {
  if (!value) return [];
  return value.split(',').map(u => u.trim()).filter(u => u);
}
```

### 4.3 Callback Implementations

```javascript
function createCallbacks() {
  const octokit = github.getOctokit(process.env.GITHUB_TOKEN);
  const [owner, repo] = process.env.GITHUB_REPOSITORY.split('/');

  return {
    // File Operations
    readFile: async (path) => {
      return await fs.readFile(path, 'utf-8');
    },

    writeFile: async (path, content) => {
      await fs.writeFile(path, content, 'utf-8');
    },

    fileExists: async (path) => {
      try {
        await fs.access(path);
        return true;
      } catch {
        return false;
      }
    },

    // Process Execution
    execGitleaks: async (args) => {
      let stdout = '';
      let stderr = '';

      const options = {
        listeners: {
          stdout: (data) => { stdout += data.toString(); },
          stderr: (data) => { stderr += data.toString(); }
        },
        ignoreReturnCode: true
      };

      const exitCode = await exec.exec('gitleaks', args, options);

      return { exitCode, stdout, stderr };
    },

    // Network Operations
    httpGet: async (url, headers = {}) => {
      const response = await fetch(url, {
        method: 'GET',
        headers: headers
      });

      return {
        statusCode: response.status,
        body: await response.text()
      };
    },

    httpPost: async (url, body, headers = {}) => {
      const response = await fetch(url, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', ...headers },
        body: body
      });

      return {
        statusCode: response.status,
        body: await response.text()
      };
    },

    // GitHub API Operations
    fetchPrCommits: async (prNumber) => {
      const { data } = await octokit.rest.pulls.listCommits({
        owner,
        repo,
        pull_number: prNumber
      });
      return data;
    },

    fetchPrComments: async (prNumber) => {
      const { data } = await octokit.rest.pulls.listReviewComments({
        owner,
        repo,
        pull_number: prNumber
      });
      return data;
    },

    postPrComment: async (prNumber, comment) => {
      await octokit.rest.pulls.createReviewComment({
        owner,
        repo,
        pull_number: prNumber,
        body: comment.body,
        commit_id: comment.commit_id,
        path: comment.path,
        line: comment.line,
        side: comment.side || 'RIGHT'
      });
    },

    // Logging
    logInfo: (message) => core.info(message),
    logWarning: (message) => core.warning(message),
    logError: (message) => core.error(message),
    logDebug: (message) => core.debug(message),

    // Cache Operations
    checkCache: async (key) => {
      const paths = [path.join(process.env.RUNNER_TOOL_CACHE, 'gitleaks')];
      const cacheKey = await cache.restoreCache(paths, key);
      return cacheKey || null;
    },

    saveCache: async (key, cachePath) => {
      await cache.saveCache([cachePath], key);
    }
  };
}
```

---

## 5. DATA SERIALIZATION

### 5.1 Serialization Strategy

All data crossing the WASM boundary is serialized as JSON for these reasons:

1. **Universal Format**: Supported by both Rust (serde_json) and JavaScript natively
2. **Human Readable**: Easy to debug and inspect
3. **Wasm-Bindgen Support**: Excellent integration with `serde-wasm-bindgen`
4. **Type Safety**: Rust structs with `#[derive(Serialize, Deserialize)]`

### 5.2 Rust Serialization Patterns

```rust
use serde::{Serialize, Deserialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub github_token: String,
    pub github_workspace: String,
    pub github_repository: String,
    pub github_event_name: String,
    pub gitleaks_version: String,
    pub enable_summary: bool,
    pub enable_comments: bool,
    pub enable_upload_artifact: bool,
    #[serde(default)]
    pub notify_user_list: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gitleaks_config: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ActionOutput {
    pub exit_code: u8,
    pub outputs: OutputValues,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub artifact_ready: bool,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct OutputValues {
    pub exit_code: u8,
}

#[derive(Serialize)]
pub struct WasmError {
    pub error: String,
    pub code: String,
    pub severity: String,
    pub context: serde_json::Value,
}
```

### 5.3 Event Data Structures

```rust
#[derive(Deserialize)]
pub struct PushEvent {
    pub commits: Vec<Commit>,
    pub repository: Repository,
}

#[derive(Deserialize)]
pub struct PullRequestEvent {
    pub pull_request: PullRequest,
    pub repository: Repository,
}

#[derive(Deserialize)]
pub struct Commit {
    pub id: String,
    pub author: Author,
    pub message: String,
}

#[derive(Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

#[derive(Deserialize)]
pub struct Repository {
    pub owner: Owner,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

#[derive(Deserialize)]
pub struct Owner {
    pub login: String,
}

#[derive(Deserialize)]
pub struct PullRequest {
    pub number: u64,
    pub base: GitRef,
    pub head: GitRef,
}

#[derive(Deserialize)]
pub struct GitRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}
```

### 5.4 SARIF Data Structures

```rust
#[derive(Deserialize)]
pub struct SarifReport {
    pub runs: Vec<SarifRun>,
}

#[derive(Deserialize)]
pub struct SarifRun {
    pub results: Vec<SarifResult>,
}

#[derive(Deserialize)]
pub struct SarifResult {
    #[serde(rename = "ruleId")]
    pub rule_id: String,
    pub message: Message,
    pub locations: Vec<Location>,
    #[serde(rename = "partialFingerprints", default)]
    pub partial_fingerprints: PartialFingerprints,
}

#[derive(Deserialize)]
pub struct Message {
    pub text: String,
}

#[derive(Deserialize)]
pub struct Location {
    #[serde(rename = "physicalLocation")]
    pub physical_location: PhysicalLocation,
}

#[derive(Deserialize)]
pub struct PhysicalLocation {
    #[serde(rename = "artifactLocation")]
    pub artifact_location: ArtifactLocation,
    pub region: Region,
}

#[derive(Deserialize)]
pub struct ArtifactLocation {
    pub uri: String,
}

#[derive(Deserialize)]
pub struct Region {
    #[serde(rename = "startLine")]
    pub start_line: u32,
}

#[derive(Deserialize, Default)]
pub struct PartialFingerprints {
    #[serde(rename = "commitSha", default)]
    pub commit_sha: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub date: String,
}
```

### 5.5 Finding Output Structure

```rust
#[derive(Serialize)]
pub struct Finding {
    pub rule_id: String,
    pub file_path: String,
    pub line_number: u32,
    pub commit_sha: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub fingerprint: String,
}

#[derive(Serialize)]
pub struct PrComment {
    pub body: String,
    pub commit_id: String,
    pub path: String,
    pub line: u32,
    pub side: String,
}
```

### 5.6 Serialization Performance

**Optimization Strategies:**

1. **Avoid Redundant Clones**: Use references where possible
2. **Stream Processing**: For large SARIF files, consider streaming parser
3. **Minimal Allocations**: Use `&str` instead of `String` where lifetime allows
4. **Serde Features**: Enable `serde` with `default-features = false` and only `alloc`

```rust
// Good: Zero-copy deserialization where possible
#[derive(Deserialize)]
pub struct Config<'a> {
    #[serde(borrow)]
    pub github_token: &'a str,
    #[serde(borrow)]
    pub github_workspace: &'a str,
    // ... other borrowed fields
}

// When ownership is needed
impl Config<'_> {
    pub fn to_owned(&self) -> OwnedConfig {
        OwnedConfig {
            github_token: self.github_token.to_string(),
            github_workspace: self.github_workspace.to_string(),
            // ...
        }
    }
}
```

---

## 6. ERROR HANDLING STRATEGY

### 6.1 Error Propagation Pattern

```rust
use wasm_bindgen::prelude::*;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Serialize)]
pub struct WasmError {
    pub error: String,
    pub code: String,
    pub severity: String,
    pub context: serde_json::Value,
}

impl WasmError {
    pub fn fatal(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: code.into(),
            severity: "fatal".to_string(),
            context: serde_json::json!({}),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
            code: code.into(),
            severity: "warning".to_string(),
            context: serde_json::json!({}),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Some(obj) = self.context.as_object_mut() {
            obj.insert(key.into(), serde_json::to_value(value).unwrap_or(serde_json::Value::Null));
        }
        self
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.error)
    }
}

impl std::error::Error for WasmError {}

// Convert to JsValue for wasm-bindgen
impl From<WasmError> for JsValue {
    fn from(err: WasmError) -> Self {
        serde_wasm_bindgen::to_value(&err).unwrap_or_else(|_| {
            JsValue::from_str(&format!("Internal error: {}", err.error))
        })
    }
}

// Result type alias
pub type WasmResult<T> = Result<T, WasmError>;
```

### 6.2 Error Code Definitions

```rust
pub mod error_codes {
    pub const CONFIG_INVALID: &str = "CONFIG_INVALID";
    pub const EVENT_PARSE_ERROR: &str = "EVENT_PARSE_ERROR";
    pub const UNSUPPORTED_EVENT: &str = "UNSUPPORTED_EVENT";
    pub const SARIF_PARSE_ERROR: &str = "SARIF_PARSE_ERROR";
    pub const GITHUB_API_ERROR: &str = "GITHUB_API_ERROR";
    pub const GITLEAKS_ERROR: &str = "GITLEAKS_ERROR";
    pub const LICENSE_REQUIRED: &str = "LICENSE_REQUIRED";
    pub const LICENSE_INVALID: &str = "LICENSE_INVALID";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const NO_COMMITS: &str = "NO_COMMITS";
}
```

### 6.3 Error Conversion Patterns

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum InternalError {
    #[error("JSON parsing failed: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Invalid git reference: {0}")]
    InvalidGitRef(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("SARIF validation failed: {0}")]
    SarifValidation(String),
}

impl From<InternalError> for WasmError {
    fn from(err: InternalError) -> Self {
        match err {
            InternalError::JsonParse(e) => {
                WasmError::fatal(error_codes::EVENT_PARSE_ERROR, format!("JSON parsing failed: {}", e))
            }
            InternalError::InvalidGitRef(ref_name) => {
                WasmError::fatal(error_codes::CONFIG_INVALID, "Invalid git reference")
                    .with_context("git_ref", ref_name)
            }
            InternalError::MissingField(field) => {
                WasmError::fatal(error_codes::EVENT_PARSE_ERROR, format!("Missing required field: {}", field))
                    .with_context("field", field)
            }
            InternalError::SarifValidation(msg) => {
                WasmError::fatal(error_codes::SARIF_PARSE_ERROR, msg)
            }
        }
    }
}
```

### 6.4 Panic Handling

```rust
use std::panic;

#[wasm_bindgen]
pub async fn run_action(config_json: &str, callbacks: JsValue) -> Result<JsValue, JsValue> {
    // Set panic hook to convert panics to WasmError
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    // Catch panics and convert to errors
    let result = panic::catch_unwind(|| {
        run_action_impl(config_json, callbacks)
    });

    match result {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(err)) => Err(err),
        Err(panic_err) => {
            let error = WasmError::fatal(
                error_codes::INTERNAL_ERROR,
                "Internal panic occurred"
            );
            Err(error.into())
        }
    }
}
```

---

## 7. MEMORY MANAGEMENT

### 7.1 WASM Memory Model

```
┌────────────────────────────────────────┐
│      WASM Linear Memory (Heap)         │
│  ┌──────────────────────────────────┐  │
│  │ Rust Allocator                   │  │
│  │ - String buffers                 │  │
│  │ - JSON deserialization           │  │
│  │ - SARIF structures               │  │
│  │ - Temporary computations         │  │
│  └──────────────────────────────────┘  │
│                                        │
│  wasm-bindgen manages memory          │
│  boundaries automatically              │
└────────────────────────────────────────┘
         ▲                    │
         │                    │
    JS → WASM            WASM → JS
  (copies data)        (copies data)
```

### 7.2 Memory Allocation Strategy

**Key Principles:**

1. **No Shared Memory**: JavaScript and WASM have separate heaps
2. **Copy on Boundary**: All data is copied when crossing boundary
3. **WASM Owns Data**: WASM manages its own memory lifecycle
4. **JS Cannot Access**: JavaScript cannot directly access WASM memory

**Implementation:**

```rust
#[wasm_bindgen]
pub struct WasmContext {
    // Internal state (not exposed to JS)
    config: Config,
    findings: Vec<Finding>,
}

#[wasm_bindgen]
impl WasmContext {
    #[wasm_bindgen(constructor)]
    pub fn new(config_json: &str) -> Result<WasmContext, JsValue> {
        let config: Config = serde_json::from_str(config_json)
            .map_err(|e| WasmError::fatal(error_codes::CONFIG_INVALID, e.to_string()))?;

        Ok(WasmContext {
            config,
            findings: Vec::new(),
        })
    }

    // Methods that return owned data (copied to JS)
    pub fn get_findings(&self) -> JsValue {
        serde_wasm_bindgen::to_value(&self.findings).unwrap()
    }
}
```

### 7.3 Memory Optimization Techniques

**1. Avoid Large Clones:**

```rust
// Bad: Clones entire structure
pub fn process_findings(findings: Vec<Finding>) -> Vec<Finding> {
    findings.iter().map(|f| f.clone()).collect()
}

// Good: Processes in-place or by reference
pub fn process_findings(findings: &mut Vec<Finding>) {
    for finding in findings.iter_mut() {
        // Process finding in-place
    }
}
```

**2. Stream Large Data:**

```rust
// Instead of loading entire SARIF into memory
pub fn parse_sarif_stream(sarif_chunks: Vec<&str>) -> WasmResult<Vec<Finding>> {
    let mut findings = Vec::new();

    for chunk in sarif_chunks {
        let partial: PartialSarif = serde_json::from_str(chunk)?;
        findings.extend(extract_findings(&partial));
    }

    Ok(findings)
}
```

**3. Use String Interning:**

```rust
use std::collections::HashMap;

pub struct StringPool {
    pool: HashMap<String, u32>,
    next_id: u32,
}

impl StringPool {
    pub fn intern(&mut self, s: &str) -> u32 {
        if let Some(&id) = self.pool.get(s) {
            id
        } else {
            let id = self.next_id;
            self.pool.insert(s.to_string(), id);
            self.next_id += 1;
            id
        }
    }
}

// Use for repeated strings (e.g., rule IDs, file paths)
```

### 7.4 Memory Limits

**Target Memory Usage:**

| Scenario | Expected Memory | Maximum Acceptable |
|----------|----------------|-------------------|
| Minimal config | < 1 MB | 2 MB |
| Small SARIF (10 findings) | < 5 MB | 10 MB |
| Large SARIF (1000 findings) | < 50 MB | 100 MB |
| Peak (with processing) | < 100 MB | 200 MB |

**Memory Monitoring:**

```rust
#[cfg(feature = "memory_stats")]
pub fn log_memory_usage() {
    if let Some(usage) = wasm_bindgen::memory::get_memory_usage() {
        web_sys::console::log_1(&format!("Memory: {} bytes", usage).into());
    }
}
```

---

## 8. PERFORMANCE OPTIMIZATION

### 8.1 Performance Goals

| Operation | Target Time | Max Acceptable |
|-----------|------------|---------------|
| WASM module load | < 50 ms | 100 ms |
| Config parsing | < 5 ms | 10 ms |
| Event parsing | < 10 ms | 20 ms |
| SARIF parsing (100 findings) | < 50 ms | 100 ms |
| Fingerprint generation (100) | < 10 ms | 20 ms |
| PR comment generation (100) | < 50 ms | 100 ms |
| Job summary HTML (100 findings) | < 100 ms | 200 ms |
| **Total WASM overhead** | **< 500 ms** | **1000 ms** |

### 8.2 Optimization Strategies

**1. Lazy Initialization:**

```rust
use once_cell::sync::Lazy;

static CONFIG: Lazy<Config> = Lazy::new(|| {
    // Initialize config once
});
```

**2. Parallel Processing (where applicable):**

```rust
#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

pub fn process_findings(findings: Vec<Finding>) -> Vec<Finding> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        findings.par_iter().map(|f| process_finding(f)).collect()
    }

    #[cfg(target_arch = "wasm32")]
    {
        findings.iter().map(|f| process_finding(f)).collect()
    }
}
```

**3. Minimize Allocations:**

```rust
// Use string builder for HTML generation
pub fn generate_summary(findings: &[Finding]) -> String {
    let mut output = String::with_capacity(findings.len() * 200); // Pre-allocate

    output.push_str("## 🛑 Gitleaks detected secrets 🛑\n\n");
    output.push_str("<table>\n");

    for finding in findings {
        output.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>\n",
            finding.rule_id, finding.file_path
        ));
    }

    output.push_str("</table>\n");
    output
}
```

**4. Efficient JSON Parsing:**

```rust
// Use serde's streaming API for large payloads
use serde_json::Deserializer;

pub fn parse_large_sarif(json: &str) -> WasmResult<Vec<Finding>> {
    let mut deserializer = Deserializer::from_str(json);
    let report: SarifReport = SarifReport::deserialize(&mut deserializer)
        .map_err(|e| WasmError::fatal(error_codes::SARIF_PARSE_ERROR, e.to_string()))?;

    Ok(extract_findings(&report))
}
```

### 8.3 Binary Size Optimization

**Cargo.toml Configuration:**

```toml
[profile.release]
opt-level = 'z'        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Single codegen unit for better optimization
strip = true           # Strip debug symbols
panic = 'abort'        # Smaller panic handler

[profile.release.package."*"]
opt-level = 'z'
```

**wasm-opt Post-Processing:**

```bash
# After wasm-pack build
wasm-opt -Oz -o optimized.wasm input.wasm

# Target: < 500 KB uncompressed
# Target: < 200 KB gzipped
```

**Dependency Minimization:**

```toml
[dependencies]
# Minimal serde
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }

# No unnecessary features
wasm-bindgen = { version = "0.2", default-features = false }

# Avoid bloated crates
# NO: regex (large), chrono (large), tokio (not needed in WASM)
# YES: lightweight alternatives when needed
```

### 8.4 Benchmarking

```rust
#[cfg(test)]
mod benchmarks {
    use super::*;
    use std::time::Instant;

    #[test]
    fn bench_sarif_parsing() {
        let sarif_json = include_str!("../fixtures/sample_sarif.json");

        let start = Instant::now();
        let result = parse_sarif_report(sarif_json);
        let duration = start.elapsed();

        assert!(result.is_ok());
        assert!(duration.as_millis() < 100, "SARIF parsing too slow: {:?}", duration);
    }
}
```

---

## 9. IMPLEMENTATION PATTERNS

### 9.1 wasm-bindgen Usage Pattern

**Module Structure:**

```rust
// src/wasm.rs

use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};

// Entry point
#[wasm_bindgen]
pub async fn run_action(
    config_json: &str,
    callbacks: JsValue
) -> Result<JsValue, JsValue> {
    // Parse config
    let config: Config = serde_json::from_str(config_json)
        .map_err(|e| WasmError::fatal(error_codes::CONFIG_INVALID, e.to_string()))?;

    // Convert JS callbacks to Rust trait
    let io = JsCallbacks::from_js(callbacks)?;

    // Run main logic
    let output = run_action_impl(config, io).await?;

    // Serialize output
    Ok(serde_wasm_bindgen::to_value(&output)?)
}

// Internal implementation (pure Rust)
async fn run_action_impl(config: Config, io: impl IoCallbacks) -> WasmResult<ActionOutput> {
    // Business logic here
    Ok(ActionOutput {
        exit_code: 0,
        outputs: OutputValues { exit_code: 0 },
        summary: None,
        artifact_ready: false,
        errors: vec![],
    })
}
```

### 9.2 Callback Trait Pattern

```rust
use async_trait::async_trait;

#[async_trait(?Send)]  // WASM is single-threaded
pub trait IoCallbacks {
    async fn read_file(&self, path: &str) -> WasmResult<String>;
    async fn exec_gitleaks(&self, args: Vec<String>) -> WasmResult<ExecResult>;
    async fn fetch_pr_commits(&self, pr_number: u64) -> WasmResult<Vec<Commit>>;
    fn log_info(&self, message: &str);
    fn log_error(&self, message: &str);
}

// JavaScript callback wrapper
pub struct JsCallbacks {
    inner: JsValue,
}

impl JsCallbacks {
    pub fn from_js(value: JsValue) -> WasmResult<Self> {
        Ok(Self { inner: value })
    }

    fn get_function(&self, name: &str) -> WasmResult<js_sys::Function> {
        let obj = js_sys::Object::from(self.inner.clone());
        let func = js_sys::Reflect::get(&obj, &JsValue::from_str(name))
            .map_err(|_| WasmError::fatal(error_codes::INTERNAL_ERROR, format!("Missing callback: {}", name)))?;

        func.dyn_into::<js_sys::Function>()
            .map_err(|_| WasmError::fatal(error_codes::INTERNAL_ERROR, format!("Invalid callback: {}", name)))
    }
}

#[async_trait(?Send)]
impl IoCallbacks for JsCallbacks {
    async fn read_file(&self, path: &str) -> WasmResult<String> {
        let func = self.get_function("readFile")?;
        let promise = func.call1(&JsValue::NULL, &JsValue::from_str(path))
            .map_err(|e| WasmError::fatal(error_codes::INTERNAL_ERROR, "readFile call failed"))?;

        let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
            .await
            .map_err(|e| WasmError::fatal(error_codes::INTERNAL_ERROR, "readFile promise rejected"))?;

        result.as_string()
            .ok_or_else(|| WasmError::fatal(error_codes::INTERNAL_ERROR, "readFile returned non-string"))
    }

    async fn exec_gitleaks(&self, args: Vec<String>) -> WasmResult<ExecResult> {
        let func = self.get_function("execGitleaks")?;
        let args_array = js_sys::Array::from_iter(args.iter().map(|s| JsValue::from_str(s)));

        let promise = func.call1(&JsValue::NULL, &args_array)
            .map_err(|_| WasmError::fatal(error_codes::INTERNAL_ERROR, "execGitleaks call failed"))?;

        let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise))
            .await
            .map_err(|_| WasmError::fatal(error_codes::GITLEAKS_ERROR, "execGitleaks failed"))?;

        serde_wasm_bindgen::from_value(result)
            .map_err(|e| WasmError::fatal(error_codes::INTERNAL_ERROR, "Invalid execGitleaks response"))
    }

    fn log_info(&self, message: &str) {
        if let Ok(func) = self.get_function("logInfo") {
            let _ = func.call1(&JsValue::NULL, &JsValue::from_str(message));
        }
    }

    fn log_error(&self, message: &str) {
        if let Ok(func) = self.get_function("logError") {
            let _ = func.call1(&JsValue::NULL, &JsValue::from_str(message));
        }
    }
}
```

### 9.3 Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Mock implementation for testing
    struct MockCallbacks {
        files: std::collections::HashMap<String, String>,
    }

    #[async_trait(?Send)]
    impl IoCallbacks for MockCallbacks {
        async fn read_file(&self, path: &str) -> WasmResult<String> {
            self.files.get(path)
                .cloned()
                .ok_or_else(|| WasmError::fatal(error_codes::INTERNAL_ERROR, "File not found"))
        }

        async fn exec_gitleaks(&self, args: Vec<String>) -> WasmResult<ExecResult> {
            Ok(ExecResult {
                exit_code: 0,
                stdout: "".to_string(),
                stderr: "".to_string(),
            })
        }

        fn log_info(&self, _: &str) {}
        fn log_error(&self, _: &str) {}
    }

    #[tokio::test]
    async fn test_push_event_processing() {
        let config = Config {
            github_event_name: "push".to_string(),
            // ... other fields
        };

        let mut callbacks = MockCallbacks {
            files: HashMap::new(),
        };

        callbacks.files.insert(
            "/path/to/event.json".to_string(),
            r#"{"commits": [{"id": "abc123"}]}"#.to_string()
        );

        let result = run_action_impl(config, callbacks).await;
        assert!(result.is_ok());
    }
}
```

---

## 10. SECURITY CONSIDERATIONS

### 10.1 Input Validation

**All inputs from JavaScript must be validated:**

```rust
pub fn validate_config(config: &Config) -> WasmResult<()> {
    // Validate git references (prevent injection)
    if let Some(ref base_ref) = config.base_ref {
        validate_git_ref(base_ref)?;
    }

    // Validate file paths (prevent traversal)
    validate_path(&config.github_workspace)?;

    if let Some(ref config_path) = config.gitleaks_config {
        validate_path(config_path)?;
    }

    // Validate repository name format
    validate_repository(&config.github_repository)?;

    Ok(())
}

fn validate_git_ref(git_ref: &str) -> WasmResult<()> {
    // Check for shell metacharacters
    let dangerous_chars = [';', '&', '|', '$', '`', '\n', '\r', '\\'];

    for c in dangerous_chars {
        if git_ref.contains(c) {
            return Err(WasmError::fatal(
                error_codes::CONFIG_INVALID,
                format!("Invalid git reference: contains '{}'", c)
            ));
        }
    }

    // Check for path traversal
    if git_ref.contains("..") {
        return Err(WasmError::fatal(
            error_codes::CONFIG_INVALID,
            "Invalid git reference: contains path traversal"
        ));
    }

    Ok(())
}

fn validate_path(path: &str) -> WasmResult<()> {
    // Check for path traversal
    if path.contains("..") {
        return Err(WasmError::fatal(
            error_codes::CONFIG_INVALID,
            "Invalid path: contains path traversal"
        ));
    }

    // Ensure absolute path or relative to workspace
    // Additional checks as needed

    Ok(())
}

fn validate_repository(repo: &str) -> WasmResult<()> {
    // Format: owner/repo
    if !repo.contains('/') {
        return Err(WasmError::fatal(
            error_codes::CONFIG_INVALID,
            "Invalid repository format (expected: owner/repo)"
        ));
    }

    // Check for invalid characters
    let invalid_chars = ['<', '>', '"', '\\', '|', '?', '*'];
    for c in invalid_chars {
        if repo.contains(c) {
            return Err(WasmError::fatal(
                error_codes::CONFIG_INVALID,
                format!("Invalid repository: contains '{}'", c)
            ));
        }
    }

    Ok(())
}
```

### 10.2 Secret Sanitization

```rust
pub fn sanitize_for_logging(text: &str, config: &Config) -> String {
    let mut sanitized = text.to_string();

    // Mask GitHub token (if present)
    if !config.github_token.is_empty() {
        sanitized = sanitized.replace(&config.github_token, "***REDACTED***");
    }

    // Mask license key (if present)
    if let Some(ref license) = config.gitleaks_license {
        sanitized = sanitized.replace(license, "***REDACTED***");
    }

    sanitized
}

pub fn safe_log_info(message: &str, config: &Config, callbacks: &impl IoCallbacks) {
    let sanitized = sanitize_for_logging(message, config);
    callbacks.log_info(&sanitized);
}
```

### 10.3 WASM Sandboxing Benefits

**Enforced by WASM:**

1. **Memory Isolation**: Cannot access host memory outside linear memory
2. **No File System**: Cannot read/write files without explicit callbacks
3. **No Network**: Cannot make HTTP requests without explicit callbacks
4. **No Process Spawning**: Cannot execute shell commands
5. **Control Flow Integrity**: Cannot perform arbitrary jumps or execute data as code

**Security Guarantees:**

```
✅ WASM module cannot:
  - Access environment variables directly
  - Read arbitrary files
  - Make network requests
  - Execute shell commands
  - Access system clipboard
  - Modify system settings

✅ All I/O must go through:
  - Explicitly provided JavaScript callbacks
  - Validated and sanitized by Rust code
  - Logged and auditable
```

### 10.4 Dependency Security

**Cargo.toml Auditing:**

```toml
# Minimal dependencies reduce attack surface
[dependencies]
wasm-bindgen = "0.2.104"  # Well-audited, widely used
serde = { version = "1.0", default-features = false, features = ["derive", "alloc"] }
serde_json = { version = "1.0", default-features = false, features = ["alloc"] }
serde-wasm-bindgen = "0.6"

# Avoid unnecessary dependencies
# NO: regex, chrono, tokio (in WASM), diesel, etc.
```

**CI Security Checks:**

```bash
# Run on every build
cargo audit         # Check for known vulnerabilities
cargo deny check    # Check licenses and advisories
cargo outdated      # Check for outdated dependencies
```

### 10.5 Data Leakage Prevention

**Never Log Sensitive Data:**

```rust
pub fn log_config(config: &Config, callbacks: &impl IoCallbacks) {
    // Good: Only log non-sensitive fields
    callbacks.log_info(&format!(
        "Config: event={}, version={}, features={{summary:{}, comments:{}, artifacts:{}}}",
        config.github_event_name,
        config.gitleaks_version,
        config.enable_summary,
        config.enable_comments,
        config.enable_upload_artifact
    ));

    // BAD: Would leak token
    // callbacks.log_info(&format!("Token: {}", config.github_token));
}

pub fn sanitize_error_context(mut error: WasmError, config: &Config) -> WasmError {
    // Remove sensitive data from error context before returning to JS
    if let Some(obj) = error.context.as_object_mut() {
        // Remove token if accidentally included
        obj.remove("github_token");
        obj.remove("gitleaks_license");
    }
    error
}
```

---

## 11. IMPLEMENTATION CHECKLIST

### 11.1 WASM Module Implementation

- [ ] Define `Config` struct with all environment variables
- [ ] Define `ActionOutput` struct with exit code and results
- [ ] Define `WasmError` struct with error codes and context
- [ ] Implement `run_action` entry point with wasm-bindgen
- [ ] Implement `IoCallbacks` trait for JavaScript interaction
- [ ] Implement `JsCallbacks` wrapper for JavaScript functions
- [ ] Add input validation for all config fields
- [ ] Add panic hook for graceful error handling
- [ ] Add comprehensive error handling with proper error codes
- [ ] Implement SARIF parsing logic
- [ ] Implement fingerprint generation
- [ ] Implement PR comment generation
- [ ] Implement job summary HTML generation
- [ ] Add unit tests for all modules
- [ ] Add integration tests with mock callbacks

### 11.2 JavaScript Wrapper Implementation

- [ ] Create `dist/index.js` entry point
- [ ] Implement `buildConfig()` function
- [ ] Implement `createCallbacks()` function
- [ ] Implement `readFile` callback
- [ ] Implement `writeFile` callback
- [ ] Implement `fileExists` callback
- [ ] Implement `execGitleaks` callback
- [ ] Implement `httpGet` callback
- [ ] Implement `httpPost` callback
- [ ] Implement `fetchPrCommits` callback
- [ ] Implement `fetchPrComments` callback
- [ ] Implement `postPrComment` callback
- [ ] Implement logging callbacks (info, warning, error, debug)
- [ ] Implement cache callbacks (check, save)
- [ ] Add error handling for WASM errors
- [ ] Add summary file writing
- [ ] Add artifact upload logic
- [ ] Add action output setting
- [ ] Add comprehensive logging

### 11.3 Build and Optimization

- [ ] Configure `Cargo.toml` with release optimizations
- [ ] Configure `wasm-pack` build command
- [ ] Add `wasm-opt` post-processing step
- [ ] Verify WASM binary size (< 500 KB target)
- [ ] Verify gzipped size (< 200 KB target)
- [ ] Add build script to package.json
- [ ] Add CI workflow for building WASM
- [ ] Test on all supported platforms (Linux, macOS, Windows)
- [ ] Benchmark WASM load time (< 50ms target)
- [ ] Benchmark total overhead (< 500ms target)

### 11.4 Testing and Validation

- [ ] Unit test all WASM functions
- [ ] Integration test with mock callbacks
- [ ] End-to-end test with real GitHub Actions
- [ ] Test all four event types (push, PR, workflow_dispatch, schedule)
- [ ] Test error scenarios
- [ ] Test with large SARIF files (1000+ findings)
- [ ] Test memory usage under load
- [ ] Verify no memory leaks
- [ ] Security audit of inputs
- [ ] Dependency vulnerability scan

---

## 12. CONCLUSION

### 12.1 Summary

This document defines a clear boundary between WASM and JavaScript:

- **WASM**: Pure computation, parsing, validation, content generation
- **JavaScript**: All I/O operations, GitHub Actions integration

The design leverages:
- JSON serialization for universal compatibility
- Callback pattern for I/O operations
- Structured errors for proper handling
- WASM sandboxing for security
- Performance optimizations for speed and size

### 12.2 Success Criteria

✅ **WASM binary size**: < 500 KB (uncompressed)
✅ **WASM load time**: < 50 ms
✅ **Total overhead**: < 500 ms
✅ **Memory usage**: < 100 MB (typical)
✅ **No panics**: All errors handled gracefully
✅ **Security**: All inputs validated
✅ **Maintainability**: Clear separation of concerns

### 12.3 Next Steps

1. Implement WASM module core functionality
2. Implement JavaScript wrapper with callbacks
3. Set up build pipeline with optimization
4. Add comprehensive tests
5. Benchmark and optimize performance
6. Security audit and validation
7. Documentation and examples

---

**Document Status:** ✅ COMPLETE
**Date:** October 16, 2025
**Architect:** WASM Boundary Architect (SPARC Architecture Phase)
