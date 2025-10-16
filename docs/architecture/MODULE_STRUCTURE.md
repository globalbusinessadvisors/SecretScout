# SecretScout Module Structure

**Project:** SecretScout - Rust Port of gitleaks-action
**Phase:** SPARC Architecture
**Date:** October 16, 2025
**Version:** 1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Crate Structure](#crate-structure)
3. [Module Hierarchy](#module-hierarchy)
4. [Module Specifications](#module-specifications)
5. [Data Type Definitions](#data-type-definitions)
6. [Dependency Graph](#dependency-graph)
7. [Public API Surface](#public-api-surface)
8. [Module Interaction Patterns](#module-interaction-patterns)
9. [WASM Considerations](#wasm-considerations)

---

## Overview

SecretScout is architected as a single Rust crate compiled to both native binary and WASM targets. The module structure prioritizes:

- **Separation of concerns**: Clear boundaries between input routing, scanning, and output generation
- **Testability**: Pure functions and dependency injection patterns
- **WASM compatibility**: No file I/O or OS-specific dependencies in core logic
- **Minimal circular dependencies**: Unidirectional data flow
- **Type safety**: Rich domain types instead of primitives

### Design Principles

1. **Platform abstraction**: Core logic is platform-agnostic; platform-specific code isolated to adapters
2. **Explicit error handling**: All operations return `Result<T, E>` with rich error types
3. **Configuration as data**: Environment variables parsed early, passed as structured config
4. **Pure business logic**: Side effects isolated to boundary modules
5. **Single responsibility**: Each module handles one aspect of the workflow

---

## Crate Structure

```
secretscout/
├── Cargo.toml                 # Crate manifest with feature flags
├── src/
│   ├── lib.rs                 # Library root (for WASM and testing)
│   ├── main.rs                # Binary entry point (native only)
│   ├── config/                # Configuration module
│   │   └── mod.rs
│   ├── events/                # Event routing module
│   │   └── mod.rs
│   ├── binary/                # Binary management module
│   │   ├── mod.rs
│   │   ├── download.rs
│   │   └── execution.rs
│   ├── sarif/                 # SARIF processing module
│   │   ├── mod.rs
│   │   └── types.rs
│   ├── github/                # GitHub API client module
│   │   ├── mod.rs
│   │   ├── client.rs
│   │   └── types.rs
│   ├── outputs/               # Output generation module
│   │   ├── mod.rs
│   │   ├── summary.rs
│   │   ├── comments.rs
│   │   └── artifacts.rs
│   ├── error.rs               # Global error types
│   └── types.rs               # Shared domain types
└── tests/                     # Integration tests
    ├── integration_test.rs
    └── fixtures/
```

### Feature Flags

```toml
[features]
default = ["native"]
native = ["tokio/rt-multi-thread", "reqwest/native-tls"]
wasm = ["wasm-bindgen", "web-sys", "js-sys"]
```

---

## Module Hierarchy

### Root Module (`lib.rs`)

**Purpose**: Library entry point, re-exports public API

**Exports**:
- `pub mod config` - Configuration types
- `pub mod error` - Error types
- `pub use events::execute` - Main execution function
- `pub mod types` - Common domain types

**Internal Modules**:
- `mod events` - Event routing (private)
- `mod binary` - Binary management (private)
- `mod sarif` - SARIF processing (private)
- `mod github` - GitHub API (private)
- `mod outputs` - Output generation (private)

**Visibility Strategy**: Only expose what's needed for WASM bindings and testing; implementation details stay private.

---

### Binary Entry Point (`main.rs`)

**Purpose**: Native executable entry point

**Responsibilities**:
1. Parse command-line arguments (if any)
2. Initialize async runtime (tokio)
3. Call `secretscout::execute()`
4. Handle top-level errors and exit codes

**Code Structure**:
```rust
#[tokio::main]
async fn main() {
    // Initialize logging
    // Load configuration from environment
    // Execute workflow
    // Map errors to exit codes
}
```

**Exit Codes**:
- `0`: Success (no secrets detected)
- `1`: Error (configuration, execution failure)
- `2`: Secrets detected

---

## Module Specifications

### 1. Configuration Module (`config/`)

**File**: `src/config/mod.rs`

**Purpose**: Parse, validate, and provide structured configuration from environment variables

**Public Types**:
```rust
pub struct Config {
    pub event_context: EventContext,
    pub scan_config: ScanConfig,
    pub output_config: OutputConfig,
    pub github_config: GitHubConfig,
}

pub enum EventContext {
    Push(PushContext),
    PullRequest(PullRequestContext),
}

pub struct ScanConfig {
    pub gitleaks_version: String,
    pub config_path: Option<PathBuf>,
    pub baseline_path: Option<PathBuf>,
    pub redact_secrets: bool,
    pub fail_on_error: bool,
}

pub struct OutputConfig {
    pub enable_upload_artifact: bool,
    pub enable_job_summary: bool,
    pub enable_pr_comment: bool,
    pub notify_user_list: Vec<String>,
}

pub struct GitHubConfig {
    pub token: String,
    pub repository: RepositoryInfo,
    pub server_url: String,
    pub api_url: String,
}

pub struct RepositoryInfo {
    pub owner: String,
    pub name: String,
}
```

**Public Functions**:
```rust
/// Load configuration from environment variables
pub fn load_from_env() -> Result<Config, ConfigError>;

/// Validate configuration for consistency
pub fn validate(config: &Config) -> Result<(), ConfigError>;
```

**Error Type**:
```rust
pub enum ConfigError {
    MissingRequired(String),
    InvalidValue { field: String, value: String, reason: String },
    ParseError { field: String, source: Box<dyn Error> },
}
```

**Responsibilities**:
- Parse all 16 environment variables
- Apply defaults (redact=true, fail_on_error=true, etc.)
- Validate combinations (e.g., PR comment requires PR event)
- Parse comma-separated lists (notify users)
- Validate URLs and paths

**Dependencies**: None (leaf module)

---

### 2. Event Routing Module (`events/`)

**File**: `src/events/mod.rs`

**Purpose**: Route execution based on GitHub event type (push vs PR)

**Public Functions**:
```rust
/// Main entry point - execute SecretScout workflow
pub async fn execute(config: Config) -> Result<ExitCode, ExecutionError>;
```

**Internal Functions**:
```rust
async fn handle_push_event(
    config: Config,
    context: PushContext,
) -> Result<ExitCode, ExecutionError>;

async fn handle_pull_request_event(
    config: Config,
    context: PullRequestContext,
) -> Result<ExitCode, ExecutionError>;
```

**Event-Specific Logic**:

**Push Event**:
1. Determine commit range (before_sha..after_sha)
2. Download/verify gitleaks binary
3. Execute scan with commit range
4. Parse SARIF results
5. Generate job summary
6. Upload artifacts (if enabled)
7. Return exit code

**Pull Request Event**:
1. Validate license (if organization)
2. Fetch PR commits via GitHub API
3. Determine commit range (base^..head)
4. Download/verify gitleaks binary
5. Execute scan with commit range
6. Parse SARIF results
7. Post PR comments (if enabled)
8. Generate job summary
9. Upload artifacts (if enabled)
10. Return exit code

**Error Type**:
```rust
pub enum ExecutionError {
    Config(ConfigError),
    Binary(BinaryError),
    Scan(ScanError),
    Sarif(SarifError),
    GitHub(GitHubError),
    Output(OutputError),
}
```

**Dependencies**:
- `config` - Configuration types
- `binary` - Binary download/execution
- `sarif` - SARIF parsing
- `github` - GitHub API client
- `outputs` - Output generation

---

### 3. Binary Management Module (`binary/`)

**Files**:
- `src/binary/mod.rs` - Module interface
- `src/binary/download.rs` - Download logic
- `src/binary/execution.rs` - Execution logic

**Purpose**: Download, verify, cache, and execute gitleaks binary

#### 3.1 Module Interface (`mod.rs`)

**Public Functions**:
```rust
/// Ensure gitleaks binary is available (download if needed)
pub async fn ensure_binary(version: &str) -> Result<PathBuf, BinaryError>;

/// Execute gitleaks scan with specified arguments
pub async fn execute_scan(
    binary_path: PathBuf,
    args: ScanArgs,
) -> Result<ScanResult, BinaryError>;
```

**Public Types**:
```rust
pub struct ScanArgs {
    pub command: ScanCommand,
    pub config_path: Option<PathBuf>,
    pub baseline_path: Option<PathBuf>,
    pub report_format: ReportFormat,
    pub report_path: PathBuf,
    pub redact: bool,
    pub log_opts: String,
    pub extra_args: Vec<String>,
}

pub enum ScanCommand {
    Detect { source: PathBuf },
    Protect {
        source: PathBuf,
        commit_from: Option<String>,
        commit_to: Option<String>,
        commit_since: Option<String>,
    },
}

pub enum ReportFormat {
    Sarif,
    Json,
}

pub struct ScanResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub report_path: PathBuf,
}

pub enum BinaryError {
    DownloadFailed { url: String, reason: String },
    VerificationFailed { path: PathBuf, reason: String },
    ExecutionFailed { command: String, exit_code: i32, stderr: String },
    NotFound { path: PathBuf },
    PermissionDenied { path: PathBuf },
}
```

#### 3.2 Download Logic (`download.rs`)

**Internal Functions**:
```rust
async fn download_gitleaks(
    version: &str,
    target_dir: PathBuf,
) -> Result<PathBuf, BinaryError>;

async fn detect_platform() -> Platform;

fn construct_download_url(version: &str, platform: Platform) -> String;

async fn download_file(url: &str, dest: PathBuf) -> Result<(), BinaryError>;

async fn extract_archive(
    archive_path: PathBuf,
    extract_to: PathBuf,
) -> Result<(), BinaryError>;

fn verify_binary(binary_path: PathBuf) -> Result<(), BinaryError>;

fn set_executable_permissions(path: PathBuf) -> Result<(), BinaryError>;
```

**Platform Detection**:
```rust
enum Platform {
    LinuxAmd64,
    LinuxArm64,
    DarwinAmd64,
    DarwinArm64,
    WindowsAmd64,
    WindowsArm64,
}
```

**Download URL Pattern**:
```
https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{os}_{arch}.{ext}
```

**Responsibilities**:
- Detect OS and architecture
- Construct GitHub release download URL
- Download binary archive (tar.gz or zip)
- Extract archive
- Set executable permissions (Unix)
- Verify binary is executable
- Cache in `~/.gitleaks/` or `/tmp/gitleaks/`

#### 3.3 Execution Logic (`execution.rs`)

**Internal Functions**:
```rust
async fn run_gitleaks(
    binary_path: PathBuf,
    args: Vec<String>,
) -> Result<ScanResult, BinaryError>;

fn build_argument_list(args: &ScanArgs) -> Vec<String>;

fn handle_exit_code(code: i32, stderr: &str) -> Result<(), BinaryError>;
```

**Argument Construction**:
- Command: `detect` or `protect`
- `--source=<path>` - Git repository path
- `--config=<path>` - Custom config (optional)
- `--baseline-path=<path>` - Baseline file (optional)
- `--report-format=sarif` - Output format
- `--report-path=<path>` - Output file path
- `--redact` - Redact secrets (conditional)
- `--log-opts=<opts>` - Git log options for commit range
- `--exit-code=<code>` - Exit code behavior

**Exit Code Handling**:
- `0`: No secrets detected → Success
- `1`: Error occurred → BinaryError
- `2`: Secrets detected → Success (with findings)

**Dependencies**:
- `reqwest` - HTTP downloads (native or WASM)
- `tar` / `zip` - Archive extraction (native only)
- `tokio::process` - Process execution (native only)

---

### 4. SARIF Processing Module (`sarif/`)

**Files**:
- `src/sarif/mod.rs` - Module interface and processing logic
- `src/sarif/types.rs` - SARIF type definitions

**Purpose**: Parse and validate SARIF reports from gitleaks

#### 4.1 Module Interface (`mod.rs`)

**Public Functions**:
```rust
/// Parse SARIF report from file
pub fn parse_sarif_file(path: PathBuf) -> Result<SarifReport, SarifError>;

/// Parse SARIF report from string
pub fn parse_sarif_str(json: &str) -> Result<SarifReport, SarifError>;

/// Extract detected secrets from SARIF report
pub fn extract_secrets(report: &SarifReport) -> Vec<DetectedSecret>;

/// Validate SARIF report structure
pub fn validate_sarif(report: &SarifReport) -> Result<(), SarifError>;
```

**Error Type**:
```rust
pub enum SarifError {
    ParseError { source: serde_json::Error },
    FileReadError { path: PathBuf, source: io::Error },
    InvalidStructure { reason: String },
    MissingField { field: String },
}
```

#### 4.2 Type Definitions (`types.rs`)

**SARIF Types** (following SARIF 2.1.0 spec):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifReport {
    pub version: String,
    pub schema: String,
    pub runs: Vec<SarifRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRun {
    pub tool: SarifTool,
    pub results: Vec<SarifResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifTool {
    pub driver: SarifDriver,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifDriver {
    pub name: String,
    pub version: String,
    pub rules: Vec<SarifRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRule {
    pub id: String,
    pub name: String,
    pub short_description: SarifMessage,
    pub properties: Option<SarifRuleProperties>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifResult {
    pub rule_id: String,
    pub rule_index: usize,
    pub level: SarifLevel,
    pub message: SarifMessage,
    pub locations: Vec<SarifLocation>,
    pub partial_fingerprints: SarifFingerprints,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifLocation {
    pub physical_location: SarifPhysicalLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifPhysicalLocation {
    pub artifact_location: SarifArtifactLocation,
    pub region: SarifRegion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifArtifactLocation {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifRegion {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub snippet: Option<SarifSnippet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SarifFingerprints {
    #[serde(rename = "commitSha")]
    pub commit_sha: String,
    pub email: String,
    pub author: String,
    pub date: String,
    #[serde(rename = "primaryLocationLineHash")]
    pub primary_location_line_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SarifLevel {
    Error,
    Warning,
    Note,
}
```

**Domain Types**:
```rust
/// Extracted and enriched secret detection
#[derive(Debug, Clone)]
pub struct DetectedSecret {
    pub rule_id: String,
    pub rule_name: String,
    pub file_path: String,
    pub start_line: u32,
    pub end_line: u32,
    pub commit_sha: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub fingerprint: String,
    pub secret_snippet: Option<String>,
}
```

**Processing Logic**:
```rust
fn extract_secrets_impl(report: &SarifReport) -> Vec<DetectedSecret> {
    // For each run
    //   For each result
    //     Extract location info
    //     Extract commit metadata from fingerprints
    //     Generate fingerprint string
    //     Build DetectedSecret
}

fn generate_fingerprint(result: &SarifResult) -> String {
    // Format: "{commitSha}:{filePath}:{ruleId}:{startLine}"
}
```

**Responsibilities**:
- Deserialize SARIF JSON
- Validate required fields
- Extract secrets with metadata
- Generate unique fingerprints
- Handle missing optional fields gracefully

**Dependencies**:
- `serde` / `serde_json` - JSON parsing
- `std::fs` - File reading (native only)

---

### 5. GitHub API Client Module (`github/`)

**Files**:
- `src/github/mod.rs` - Module interface
- `src/github/client.rs` - HTTP client and API functions
- `src/github/types.rs` - GitHub API types

**Purpose**: Interact with GitHub REST API for metadata and PR comments

#### 5.1 Module Interface (`mod.rs`)

**Public Functions**:
```rust
/// Create GitHub API client
pub fn create_client(token: String) -> Result<GitHubClient, GitHubError>;

/// Get account type (User or Organization)
pub async fn get_account_type(
    client: &GitHubClient,
    username: &str,
) -> Result<AccountType, GitHubError>;

/// Get latest gitleaks release version
pub async fn get_latest_gitleaks_version(
    client: &GitHubClient,
) -> Result<String, GitHubError>;

/// Get commits in pull request
pub async fn get_pr_commits(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Vec<PullRequestCommit>, GitHubError>;

/// Get existing PR review comments
pub async fn get_pr_comments(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
) -> Result<Vec<ReviewComment>, GitHubError>;

/// Post PR review comment
pub async fn post_pr_comment(
    client: &GitHubClient,
    owner: &str,
    repo: &str,
    pr_number: u64,
    comment: CreateCommentRequest,
) -> Result<ReviewComment, GitHubError>;
```

**Error Type**:
```rust
pub enum GitHubError {
    NetworkError { source: reqwest::Error },
    HttpError { status: u16, message: String },
    RateLimitExceeded { reset_at: u64, retry_after: Option<u64> },
    Unauthorized { message: String },
    Forbidden { message: String },
    NotFound { resource: String },
    ParseError { source: serde_json::Error },
    InvalidInput { field: String, reason: String },
}
```

#### 5.2 Client Implementation (`client.rs`)

**Client Structure**:
```rust
pub struct GitHubClient {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
    rate_limit_state: Arc<RateLimitState>,
}

struct RateLimitState {
    remaining: AtomicUsize,
    reset_time: AtomicU64,
    limit: AtomicUsize,
}
```

**Retry Logic**:
```rust
async fn retry_with_backoff<F, T>(
    operation: F,
    config: RetryConfig,
) -> Result<T, GitHubError>
where
    F: Fn() -> Future<Output = Result<T, GitHubError>>,
{
    // Exponential backoff with jitter
    // Retry on: 429, 500, 502, 503, 504
    // Max retries: 3
    // Initial backoff: 1s
    // Max backoff: 60s
}
```

**Rate Limit Handling**:
```rust
fn parse_rate_limit_headers(response: &Response) -> Option<RateLimitInfo>;

fn update_rate_limit_state(
    client: &GitHubClient,
    info: RateLimitInfo,
);

async fn check_rate_limit(client: &GitHubClient) -> Result<(), GitHubError>;
```

#### 5.3 API Types (`types.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestCommit {
    pub sha: String,
    pub commit: CommitDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitDetails {
    pub message: String,
    pub author: GitAuthor,
    pub committer: GitAuthor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitAuthor {
    pub name: String,
    pub email: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: u64,
    pub body: String,
    pub path: String,
    pub line: Option<u32>,
    pub commit_id: String,
    pub user: User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub login: String,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateCommentRequest {
    pub body: String,
    pub commit_id: String,
    pub path: String,
    pub side: String, // Always "RIGHT"
    pub line: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum AccountType {
    User,
    Organization,
}
```

**Responsibilities**:
- HTTP client with auth headers
- Retry with exponential backoff
- Rate limit tracking and handling
- Pagination for list endpoints
- Error mapping and context

**Dependencies**:
- `reqwest` - HTTP client
- `serde` / `serde_json` - JSON serialization
- `tokio` - Async runtime

---

### 6. Output Generation Module (`outputs/`)

**Files**:
- `src/outputs/mod.rs` - Module interface
- `src/outputs/summary.rs` - Job summary generation
- `src/outputs/comments.rs` - PR comment generation
- `src/outputs/artifacts.rs` - Artifact upload (delegated to GitHub Actions)

**Purpose**: Generate various output formats from scan results

#### 6.1 Module Interface (`mod.rs`)

**Public Functions**:
```rust
/// Generate job summary markdown
pub fn generate_job_summary(
    exit_code: ExitCode,
    secrets: &[DetectedSecret],
    config: &SummaryConfig,
) -> Result<String, OutputError>;

/// Generate PR comment body for a secret
pub fn generate_pr_comment(
    secret: &DetectedSecret,
    notify_users: &[String],
) -> String;

/// Check if comment is duplicate
pub fn is_duplicate_comment(
    existing: &[ReviewComment],
    new_comment: &CreateCommentRequest,
) -> bool;
```

**Error Type**:
```rust
pub enum OutputError {
    GenerationFailed { reason: String },
    HtmlEscapingFailed { source: Box<dyn Error> },
    UrlEncodingFailed { path: String },
}
```

#### 6.2 Job Summary (`summary.rs`)

**Summary Types**:
```rust
pub struct SummaryConfig {
    pub repository_url: Option<String>,
    pub enabled: bool,
}

pub enum SummaryType {
    Success,             // Exit code 0
    SecretsDetected,     // Exit code 2
    Error(i32),          // Exit code 1 or other
}
```

**Generation Functions**:
```rust
fn generate_success_summary() -> String {
    "## No leaks detected ✅\n"
}

fn generate_secrets_summary(
    secrets: &[DetectedSecret],
    config: &SummaryConfig,
) -> String {
    // Header: "## 🛑 Gitleaks detected secrets 🛑"
    // HTML table with columns:
    //   - Rule ID
    //   - Commit (linked)
    //   - Secret URL (linked to file:line)
    //   - Start Line
    //   - Author
    //   - Date
    //   - Email
    //   - File (linked)
}

fn generate_error_summary(exit_code: i32) -> String {
    format!("## ❌ Gitleaks exited with error. Exit code [{}]\n", exit_code)
}

fn escape_html(text: &str) -> String;

fn url_encode(path: &str) -> String;

fn generate_commit_url(repo_url: &str, commit_sha: &str) -> String;

fn generate_secret_url(
    repo_url: &str,
    commit_sha: &str,
    file_path: &str,
    line: u32,
) -> String;
```

#### 6.3 PR Comments (`comments.rs`)

**Comment Template**:
```
🛑 **Gitleaks** has detected secret in code.

**Rule:** {rule_id}
**Commit:** {commit_sha_short}

**Fingerprint:** `{fingerprint}`

To ignore this secret, add the fingerprint to `.gitleaksignore`

**CC:** @user1 @user2
```

**Generation Function**:
```rust
pub fn generate_pr_comment(
    secret: &DetectedSecret,
    notify_users: &[String],
) -> String {
    let mut body = format!(
        "🛑 **Gitleaks** has detected secret in code.\n\n\
         **Rule:** {}\n\
         **Commit:** {}\n\n\
         **Fingerprint:** `{}`\n\n\
         To ignore this secret, add the fingerprint to `.gitleaksignore`",
        secret.rule_id,
        &secret.commit_sha[..7],
        secret.fingerprint,
    );

    if !notify_users.is_empty() {
        let mentions = notify_users
            .iter()
            .map(|u| format!("@{}", u))
            .collect::<Vec<_>>()
            .join(" ");
        body.push_str(&format!("\n\n**CC:** {}", mentions));
    }

    body
}
```

**Deduplication**:
```rust
pub fn is_duplicate_comment(
    existing: &[ReviewComment],
    new_comment: &CreateCommentRequest,
) -> bool {
    existing.iter().any(|c| {
        c.path == new_comment.path
            && c.line == Some(new_comment.line)
            && c.commit_id == new_comment.commit_id
            && c.body.contains(&new_comment.body)
    })
}
```

**Responsibilities**:
- HTML table generation with proper escaping
- URL construction for GitHub links
- Comment body formatting
- Duplicate detection
- User mention formatting

**Dependencies**:
- `html_escape` - HTML escaping
- `url` - URL encoding

---

### 7. Error Module (`error.rs`)

**Purpose**: Global error types and conversions

**Error Hierarchy**:
```rust
/// Top-level application error
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    #[error("Binary management error: {0}")]
    Binary(#[from] BinaryError),

    #[error("SARIF processing error: {0}")]
    Sarif(#[from] SarifError),

    #[error("GitHub API error: {0}")]
    GitHub(#[from] GitHubError),

    #[error("Output generation error: {0}")]
    Output(#[from] OutputError),
}

/// Exit code for process termination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Success = 0,
    Error = 1,
    SecretsFound = 2,
}

impl From<i32> for ExitCode {
    fn from(code: i32) -> Self {
        match code {
            0 => ExitCode::Success,
            2 => ExitCode::SecretsFound,
            _ => ExitCode::Error,
        }
    }
}

impl From<ExitCode> for i32 {
    fn from(code: ExitCode) -> i32 {
        code as i32
    }
}
```

**Error Conversion**:
```rust
impl AppError {
    /// Convert error to exit code
    pub fn exit_code(&self) -> ExitCode {
        match self {
            AppError::Execution(ExecutionError::SecretsDetected) => ExitCode::SecretsFound,
            _ => ExitCode::Error,
        }
    }

    /// Get user-friendly error message
    pub fn display_message(&self) -> String {
        // Format error for console output
    }
}
```

---

### 8. Shared Types Module (`types.rs`)

**Purpose**: Common domain types used across modules

**Types**:
```rust
/// Git commit SHA (40 hex characters)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommitSha(String);

impl CommitSha {
    pub fn new(sha: String) -> Result<Self, InvalidShaError> {
        if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(CommitSha(sha))
        } else {
            Err(InvalidShaError(sha))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn short(&self) -> &str {
        &self.0[..7]
    }
}

/// File path (relative to repository root)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePath(String);

impl FilePath {
    pub fn new(path: String) -> Self {
        // Normalize path separators
        let normalized = path.replace('\\', "/");
        FilePath(normalized)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Line number in file (1-indexed)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineNumber(u32);

impl LineNumber {
    pub fn new(line: u32) -> Result<Self, InvalidLineError> {
        if line > 0 {
            Ok(LineNumber(line))
        } else {
            Err(InvalidLineError(line))
        }
    }

    pub fn get(&self) -> u32 {
        self.0
    }
}

/// Secret fingerprint (unique identifier)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint(String);

impl Fingerprint {
    pub fn new(
        commit_sha: &CommitSha,
        file_path: &FilePath,
        rule_id: &str,
        line: LineNumber,
    ) -> Self {
        let value = format!(
            "{}:{}:{}:{}",
            commit_sha.as_str(),
            file_path.as_str(),
            rule_id,
            line.get(),
        );
        Fingerprint(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
```

---

## Data Type Definitions

### Type Categories

1. **Configuration Types** (`config/`)
   - Environment variable mappings
   - Validated settings
   - Event context

2. **Domain Types** (`types.rs`)
   - Value objects (CommitSha, FilePath, etc.)
   - Business entities (DetectedSecret)

3. **API Types** (`github/types.rs`)
   - GitHub API request/response
   - Pagination info

4. **SARIF Types** (`sarif/types.rs`)
   - SARIF 2.1.0 specification
   - Gitleaks-specific extensions

5. **Error Types** (`error.rs`)
   - Error hierarchy
   - Exit codes
   - Error context

### Type Safety Patterns

**Newtype Pattern**: Wrap primitives in dedicated types
```rust
CommitSha(String)  // Not just String
LineNumber(u32)    // Not just u32
```

**Builder Pattern**: Complex type construction
```rust
ScanArgs::builder()
    .command(ScanCommand::Protect)
    .source("/path/to/repo")
    .redact(true)
    .build()?
```

**Enum for Variants**: Explicit state modeling
```rust
enum EventContext {
    Push(PushContext),
    PullRequest(PullRequestContext),
}
```

**Result for Fallibility**: No exceptions
```rust
fn parse_config() -> Result<Config, ConfigError>
fn execute_scan() -> Result<ScanResult, BinaryError>
```

---

## Dependency Graph

### Module Dependencies (Acyclic)

```
                         ┌─────────────┐
                         │   main.rs   │
                         └──────┬──────┘
                                │
                         ┌──────▼──────┐
                         │  events/    │
                         └──────┬──────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
   ┌────▼─────┐          ┌─────▼─────┐          ┌─────▼──────┐
   │ binary/  │          │  github/  │          │  outputs/  │
   └────┬─────┘          └─────┬─────┘          └─────┬──────┘
        │                      │                       │
        │                      │                       │
        │                ┌─────▼─────┐                │
        │                │  sarif/   │                │
        │                └─────┬─────┘                │
        │                      │                       │
        └──────────────────────┼───────────────────────┘
                               │
                        ┌──────▼───────┐
                        │   config/    │
                        │   types.rs   │
                        │   error.rs   │
                        └──────────────┘
```

### Dependency Rules

1. **Leaf Modules** (no internal dependencies):
   - `config/` - Only depends on std and env
   - `types.rs` - Pure data types
   - `error.rs` - Error definitions

2. **Core Modules** (depend on leaves):
   - `sarif/` - Depends on: types, error
   - `binary/` - Depends on: types, error
   - `github/` - Depends on: types, error
   - `outputs/` - Depends on: types, error, sarif, github

3. **Orchestration Module** (depends on core):
   - `events/` - Depends on: config, binary, sarif, github, outputs, types, error

4. **Entry Points** (depend on orchestration):
   - `main.rs` - Depends on: events, config, error
   - `lib.rs` - Re-exports public API

### External Dependencies

**Required Crates**:
- `serde` / `serde_json` - Serialization
- `tokio` - Async runtime (native)
- `reqwest` - HTTP client
- `thiserror` - Error derivation
- `anyhow` - Error context (optional)
- `tracing` / `tracing-subscriber` - Logging

**Platform-Specific**:
- **Native**:
  - `tar` - TAR extraction
  - `zip` - ZIP extraction
  - `tokio::process` - Process execution
  - `reqwest/native-tls` - TLS implementation

- **WASM**:
  - `wasm-bindgen` - JS interop
  - `web-sys` - Browser APIs
  - `js-sys` - JS standard library
  - `reqwest/wasm` - Fetch API wrapper

---

## Public API Surface

### For Native Binary (`main.rs`)

```rust
// Main execution function
pub async fn execute(config: Config) -> Result<ExitCode, AppError>;
```

### For WASM Target

```rust
// WASM entry point
#[wasm_bindgen]
pub async fn execute_wasm(env_vars: JsValue) -> Result<JsValue, JsValue>;

// Convert JS object to Config
fn parse_config_from_js(env_vars: JsValue) -> Result<Config, JsValue>;

// Convert Rust result to JS object
fn serialize_result_to_js(exit_code: ExitCode) -> JsValue;
```

### For Testing

```rust
// Re-export public types
pub use config::Config;
pub use error::{AppError, ExitCode};
pub use types::{CommitSha, DetectedSecret, Fingerprint};

// Test utilities
#[cfg(test)]
pub mod test_utils {
    pub fn create_test_config() -> Config;
    pub fn create_mock_github_client() -> GitHubClient;
    pub fn create_test_sarif_report() -> SarifReport;
}
```

### API Stability

**Stable** (semver guarantees):
- `execute()` function signature
- `Config` struct fields
- `ExitCode` enum
- Error type hierarchy

**Unstable** (may change):
- Internal module organization
- Private helper functions
- Test utilities

---

## Module Interaction Patterns

### 1. Configuration Flow

```
Environment Variables
        │
        ▼
   config::load_from_env()
        │
        ▼
     Config
        │
        ├──► events::execute()
        │         │
        │         ├──► binary::ensure_binary()
        │         ├──► github::create_client()
        │         └──► outputs::generate_job_summary()
        │
        └──► Passed to all modules
```

### 2. Error Propagation

```
Module-Specific Error
        │
        ▼
   Result<T, ModuleError>
        │
        ▼
   ? operator (automatic conversion)
        │
        ▼
   Result<T, AppError>
        │
        ▼
   main.rs
        │
        ├──► Log error
        └──► Convert to exit code
```

### 3. Data Flow (Push Event)

```
events::handle_push_event()
        │
        ├──► binary::execute_scan()
        │         └──► ScanResult { report_path }
        │
        ├──► sarif::parse_sarif_file()
        │         └──► Vec<DetectedSecret>
        │
        └──► outputs::generate_job_summary()
                  └──► String (markdown)
```

### 4. Data Flow (PR Event)

```
events::handle_pull_request_event()
        │
        ├──► github::get_pr_commits()
        │         └──► Vec<PullRequestCommit>
        │
        ├──► binary::execute_scan()
        │         └──► ScanResult { report_path }
        │
        ├──► sarif::parse_sarif_file()
        │         └──► Vec<DetectedSecret>
        │
        ├──► github::get_pr_comments()
        │         └──► Vec<ReviewComment>
        │
        ├──► For each DetectedSecret:
        │    ├──► outputs::generate_pr_comment()
        │    │         └──► String (comment body)
        │    │
        │    ├──► outputs::is_duplicate_comment()
        │    │         └──► bool
        │    │
        │    └──► github::post_pr_comment()
        │              └──► ReviewComment
        │
        └──► outputs::generate_job_summary()
                  └──► String (markdown)
```

### 5. Dependency Injection Pattern

Instead of direct instantiation, pass dependencies as parameters:

```rust
// Good: Dependency injection
async fn handle_pull_request_event(
    config: Config,
    github_client: &GitHubClient,  // Injected
) -> Result<ExitCode, ExecutionError>

// Bad: Direct instantiation
async fn handle_pull_request_event(
    config: Config,
) -> Result<ExitCode, ExecutionError> {
    let github_client = GitHubClient::new(...);  // Hard to test
}
```

**Benefits**:
- Testability (inject mocks)
- Flexibility (swap implementations)
- Explicitness (dependencies visible in signature)

### 6. Builder Pattern for Complex Types

```rust
let scan_args = ScanArgs::builder()
    .command(ScanCommand::Protect)
    .source(PathBuf::from("/repo"))
    .config_path(Some(PathBuf::from("gitleaks.toml")))
    .redact(config.scan_config.redact_secrets)
    .report_format(ReportFormat::Sarif)
    .report_path(PathBuf::from("/tmp/report.sarif"))
    .build()?;

let result = binary::execute_scan(binary_path, scan_args).await?;
```

### 7. Result Chaining with Context

```rust
use anyhow::Context;

let report = sarif::parse_sarif_file(&report_path)
    .context("Failed to parse SARIF report")?;

let secrets = sarif::extract_secrets(&report)
    .context("Failed to extract secrets from SARIF")?;
```

---

## WASM Considerations

### Platform Abstraction Layer

**File I/O Abstraction**:
```rust
#[cfg(not(target_arch = "wasm32"))]
mod file_io {
    use std::fs;
    use std::path::Path;

    pub fn read_file(path: &Path) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }

    pub fn write_file(path: &Path, contents: &str) -> Result<(), std::io::Error> {
        fs::write(path, contents)
    }
}

#[cfg(target_arch = "wasm32")]
mod file_io {
    use wasm_bindgen::prelude::*;

    pub fn read_file(path: &str) -> Result<String, JsValue> {
        // Call JS function to read file
        let window = web_sys::window().unwrap();
        let promise = window.fs_read_file(path);
        // ... await promise
    }

    pub fn write_file(path: &str, contents: &str) -> Result<(), JsValue> {
        // Call JS function to write file
        let window = web_sys::window().unwrap();
        window.fs_write_file(path, contents)?;
        Ok(())
    }
}
```

### HTTP Client (Already Abstracted by reqwest)

```rust
// Works on both native and WASM
let client = reqwest::Client::new();
let response = client
    .get("https://api.github.com/users/octocat")
    .send()
    .await?;
```

### Process Execution (Native Only)

```rust
#[cfg(not(target_arch = "wasm32"))]
async fn execute_gitleaks_native(
    binary_path: PathBuf,
    args: Vec<String>,
) -> Result<ScanResult, BinaryError> {
    use tokio::process::Command;

    let output = Command::new(binary_path)
        .args(args)
        .output()
        .await?;

    // Process output...
}

#[cfg(target_arch = "wasm32")]
async fn execute_gitleaks_wasm(
    args: Vec<String>,
) -> Result<ScanResult, BinaryError> {
    // Call JS function that invokes gitleaks
    // (Gitleaks must be available as WASM or via worker)
    unimplemented!("WASM execution requires JS bridge")
}
```

### WASM Binary Size Optimization

**Cargo.toml**:
```toml
[profile.release]
opt-level = "z"           # Optimize for size
lto = true                # Link-time optimization
codegen-units = 1         # Single codegen unit for better optimization
strip = true              # Strip symbols
panic = "abort"           # Smaller panic handler
```

**Feature Flags**:
```toml
[features]
default = ["native"]

native = [
    "tokio/rt-multi-thread",
    "tokio/process",
    "reqwest/native-tls",
]

wasm = [
    "wasm-bindgen",
    "web-sys",
    "js-sys",
    "reqwest/wasm",
]
```

### Memory Management

**Avoid Large Allocations**:
- Stream large files instead of reading entirely into memory
- Process SARIF results incrementally
- Release intermediate data structures

**Shared Data via Arc**:
```rust
use std::sync::Arc;

// Share immutable config across async tasks
let config = Arc::new(config);
let config_clone = Arc::clone(&config);

tokio::spawn(async move {
    // Use config_clone
});
```

---

## Testing Strategy

### Unit Tests

**Per Module**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config_valid() {
        // Test configuration parsing
    }

    #[test]
    fn test_generate_fingerprint() {
        // Test fingerprint generation
    }

    #[tokio::test]
    async fn test_github_api_retry() {
        // Test retry logic with mock server
    }
}
```

### Integration Tests

**`tests/integration_test.rs`**:
```rust
use secretscout::{Config, execute};

#[tokio::test]
async fn test_push_event_workflow() {
    // Set up test environment
    // Create test config
    // Execute workflow
    // Assert exit code and outputs
}

#[tokio::test]
async fn test_pr_event_workflow() {
    // Similar integration test for PR event
}
```

### Mock Strategies

**GitHub API Mocking**:
```rust
use mockito::Server;

#[tokio::test]
async fn test_get_account_type() {
    let mut server = Server::new_async().await;

    let mock = server.mock("GET", "/users/octocat")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"login": "octocat", "type": "User"}"#)
        .create();

    let client = create_test_client(&server.url());
    let account_type = get_account_type(&client, "octocat").await.unwrap();

    assert_eq!(account_type, AccountType::User);
    mock.assert();
}
```

**File System Mocking**:
```rust
use tempfile::TempDir;

#[test]
fn test_sarif_parsing() {
    let temp_dir = TempDir::new().unwrap();
    let sarif_path = temp_dir.path().join("report.sarif");

    std::fs::write(&sarif_path, SAMPLE_SARIF_JSON).unwrap();

    let report = parse_sarif_file(&sarif_path).unwrap();
    assert_eq!(report.runs.len(), 1);
}
```

### Test Fixtures

**`tests/fixtures/`**:
- `sample_sarif.json` - Valid SARIF report
- `invalid_sarif.json` - Malformed SARIF
- `empty_sarif.json` - SARIF with no results
- `.env.test` - Test environment variables

---

## Build and Distribution

### Build Targets

**Native Binary**:
```bash
cargo build --release --features native
```

**WASM Package**:
```bash
wasm-pack build --target web --features wasm
```

### GitHub Actions Integration

**Distribution**: Publish as GitHub Action with pre-built binaries

```yaml
# action.yml
name: 'SecretScout'
description: 'Detect secrets in Git repositories'
inputs:
  github_token:
    required: true
runs:
  using: 'composite'
  steps:
    - name: Download SecretScout
      run: |
        curl -L -o secretscout https://github.com/owner/secretscout/releases/latest/download/secretscout-linux-amd64
        chmod +x secretscout

    - name: Run SecretScout
      run: ./secretscout
      env:
        GITHUB_TOKEN: ${{ inputs.github_token }}
```

---

## Summary

### Module Count: 8 Core Modules

1. **config/** - Configuration parsing and validation
2. **events/** - Event routing and orchestration
3. **binary/** - Binary download and execution
4. **sarif/** - SARIF parsing and processing
5. **github/** - GitHub API client
6. **outputs/** - Output generation (summary, comments, artifacts)
7. **error.rs** - Error types and conversions
8. **types.rs** - Shared domain types

### Lines of Code Estimate

| Module | Estimated LOC | Complexity |
|--------|---------------|------------|
| config/ | 300 | Low |
| events/ | 500 | High |
| binary/ | 600 | Medium |
| sarif/ | 400 | Medium |
| github/ | 700 | High |
| outputs/ | 400 | Medium |
| error.rs | 150 | Low |
| types.rs | 200 | Low |
| main.rs | 100 | Low |
| lib.rs | 50 | Low |
| **Total** | **~3,400** | **Medium** |

### Key Architectural Decisions

1. **Single Crate**: Simpler dependency management, easier WASM compilation
2. **Async Throughout**: Consistent async/await for I/O operations
3. **Type-Safe Domain Model**: Newtype pattern prevents primitive obsession
4. **Explicit Error Handling**: No unwraps in production code, rich error context
5. **Platform Abstraction**: Conditional compilation for native vs WASM
6. **Dependency Injection**: Functions accept dependencies as parameters
7. **Pure Core Logic**: Side effects at boundaries only

---

## Next Steps

1. **Implementation Phase**: Implement each module according to pseudocode
2. **Unit Testing**: Write tests alongside implementation
3. **Integration Testing**: End-to-end workflow tests
4. **Documentation**: Add rustdoc comments to public API
5. **Performance Testing**: Benchmark critical paths (SARIF parsing, API calls)
6. **Security Audit**: Review token handling, injection risks
7. **WASM Compilation**: Verify WASM build and bundle size
8. **CI/CD Pipeline**: Automated testing and release workflow

---

**Document Status**: ✅ COMPLETE

**Validation Checklist**:
- [x] All modules defined with clear responsibilities
- [x] Data types specified for each module
- [x] Dependency graph is acyclic
- [x] Public API surface documented
- [x] WASM compatibility considered
- [x] Error handling strategy defined
- [x] Testing approach outlined
- [x] Module interactions documented

**Approval**: Ready for implementation phase.
