# ERROR HANDLING ARCHITECTURE

**Project:** SecretScout - Rust Port of gitleaks-action
**Phase:** Architecture
**Document:** Error Handling & Recovery Architecture
**Date:** October 16, 2025
**Version:** 1.0

---

## TABLE OF CONTENTS

1. [Executive Summary](#1-executive-summary)
2. [Error Type Taxonomy](#2-error-type-taxonomy)
3. [Error Handling Patterns](#3-error-handling-patterns)
4. [Recovery Strategies](#4-recovery-strategies)
5. [WASM Boundary Error Handling](#5-wasm-boundary-error-handling)
6. [Component Error Strategies](#6-component-error-strategies)
7. [Logging Architecture](#7-logging-architecture)
8. [User-Facing Error Messages](#8-user-facing-error-messages)
9. [Exit Code Mapping](#9-exit-code-mapping)
10. [Error Context & Diagnostics](#10-error-context--diagnostics)

---

## 1. EXECUTIVE SUMMARY

### 1.1 Purpose

This document defines the comprehensive error handling and recovery architecture for SecretScout. It provides a systematic approach to error management that:

- **Classifies errors** by severity and recoverability
- **Preserves context** for debugging and user guidance
- **Handles WASM boundaries** correctly
- **Provides clear user feedback** for all error conditions
- **Enables graceful degradation** where appropriate

### 1.2 Design Principles

**1. Type Safety First**
- Leverage Rust's `Result<T, E>` and `Option<T>` types
- No panics in production code
- Explicit error propagation via `?` operator

**2. Context Preservation**
- Attach context at error creation point
- Maintain error chains for debugging
- Include operation details in error messages

**3. Clear Boundaries**
- Fatal errors exit immediately
- Non-fatal errors log and continue
- Expected errors (like "no commits") handled gracefully

**4. User-Centric Messages**
- Technical details for debugging
- Actionable guidance for users
- No secret leakage in error messages

**5. WASM Compatibility**
- Errors serialize across WASM boundary
- JavaScript wrapper handles final exit codes
- No stack traces in WASM (limited support)

### 1.3 Error Flow Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Error Occurs                         │
└────────────────┬────────────────────────────────────────┘
                 │
                 ├─> Classify Error Severity
                 │   ├─ Fatal
                 │   ├─ NonFatal
                 │   └─ Expected
                 │
                 ├─> Attach Context
                 │   ├─ Operation name
                 │   ├─ Input parameters
                 │   └─ Underlying cause
                 │
                 ├─> Log Appropriately
                 │   ├─ Fatal → ERROR
                 │   ├─ NonFatal → WARNING
                 │   └─ Expected → INFO
                 │
                 ├─> Attempt Recovery (if applicable)
                 │   ├─ Retry with backoff
                 │   ├─ Fallback strategy
                 │   └─ Graceful degradation
                 │
                 └─> Return Result
                     ├─ Success → Continue
                     ├─ Fatal → Exit 1
                     └─ NonFatal → Continue with warning
```

---

## 2. ERROR TYPE TAXONOMY

### 2.1 Error Hierarchy

```rust
/// Top-level error type for SecretScout
#[derive(Debug, thiserror::Error)]
pub enum SecretScoutError {
    // Configuration Errors (Fatal)
    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigError),

    // Event Processing Errors (Fatal)
    #[error("Event processing error: {0}")]
    EventProcessing(#[from] EventError),

    // Binary Management Errors (Fatal)
    #[error("Binary management error: {0}")]
    BinaryManagement(#[from] BinaryError),

    // SARIF Processing Errors (Fatal)
    #[error("SARIF processing error: {0}")]
    SarifProcessing(#[from] SarifError),

    // GitHub API Errors (Mixed)
    #[error("GitHub API error: {0}")]
    GitHubApi(#[from] GitHubError),

    // License Validation Errors (Fatal when enabled)
    #[error("License validation error: {0}")]
    LicenseValidation(#[from] LicenseError),

    // Expected Conditions (Non-Error)
    #[error("{0}")]
    Expected(String),
}

/// Configuration-related errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {variable}")]
    MissingEnvVar { variable: String },

    #[error("Invalid environment variable value: {variable} = {value}")]
    InvalidEnvVar { variable: String, value: String },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Invalid configuration file: {path} - {reason}")]
    InvalidConfig { path: String, reason: String },

    #[error("Path validation failed: {path} - {reason}")]
    PathValidation { path: String, reason: String },
}

/// Event processing errors
#[derive(Debug, thiserror::Error)]
pub enum EventError {
    #[error("Unsupported event type: {event_type}")]
    UnsupportedEvent { event_type: String },

    #[error("Failed to read event file: {path}")]
    FileRead { path: String, source: std::io::Error },

    #[error("Failed to parse event JSON: {reason}")]
    JsonParse { reason: String },

    #[error("Missing required event field: {field}")]
    MissingField { field: String },

    #[error("Invalid git reference: {reference} - {reason}")]
    InvalidGitRef { reference: String, reason: String },

    #[error("No commits in push event (expected condition)")]
    NoCommits,
}

/// Binary management errors
#[derive(Debug, thiserror::Error)]
pub enum BinaryError {
    #[error("Unsupported platform: {platform}")]
    UnsupportedPlatform { platform: String },

    #[error("Unsupported architecture: {arch}")]
    UnsupportedArchitecture { arch: String },

    #[error("Failed to download binary from {url}")]
    DownloadFailed { url: String, source: reqwest::Error },

    #[error("Failed to extract archive: {path}")]
    ExtractionFailed { path: String, source: std::io::Error },

    #[error("Binary not found in archive: {binary_name}")]
    BinaryNotFound { binary_name: String },

    #[error("Failed to execute gitleaks: {command}")]
    ExecutionFailed { command: String, source: std::io::Error },

    #[error("Gitleaks execution error (exit code {exit_code})")]
    GitleaksError { exit_code: i32, stderr: String },
}

/// SARIF processing errors
#[derive(Debug, thiserror::Error)]
pub enum SarifError {
    #[error("SARIF file not found: {path}")]
    FileNotFound { path: String },

    #[error("Failed to read SARIF file: {path}")]
    FileRead { path: String, source: std::io::Error },

    #[error("Failed to parse SARIF JSON")]
    JsonParse { source: serde_json::Error },

    #[error("Invalid SARIF structure: {reason}")]
    InvalidStructure { reason: String },

    #[error("Missing required SARIF field: {field}")]
    MissingField { field: String },
}

/// GitHub API errors
#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("Failed to authenticate with GitHub API")]
    Authentication { source: octocrab::Error },

    #[error("GitHub API request failed: {endpoint}")]
    RequestFailed { endpoint: String, source: octocrab::Error },

    #[error("Rate limit exceeded. Retry after {retry_after} seconds")]
    RateLimitExceeded { retry_after: u64 },

    #[error("Resource not found: {resource}")]
    NotFound { resource: String },

    #[error("Failed to post PR comment on {file}:{line}")]
    CommentFailed { file: String, line: u32, source: octocrab::Error },

    #[error("Maximum retry attempts exceeded for {operation}")]
    MaxRetriesExceeded { operation: String },
}

/// License validation errors
#[derive(Debug, thiserror::Error)]
pub enum LicenseError {
    #[error("License key required for organization accounts")]
    MissingLicense,

    #[error("License validation failed: {code}")]
    ValidationFailed { code: String, message: String },

    #[error("License limit exceeded (too many machines)")]
    LimitExceeded,

    #[error("Failed to activate license for repository")]
    ActivationFailed { source: reqwest::Error },
}
```

### 2.2 Error Severity Classification

```rust
/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Fatal errors that require immediate exit
    Fatal,

    /// Non-fatal errors that allow continued execution with warnings
    NonFatal,

    /// Expected conditions that are not true errors (e.g., empty commit list)
    Expected,
}

impl SecretScoutError {
    /// Determine severity of error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Fatal errors - must exit immediately
            SecretScoutError::Configuration(_) => ErrorSeverity::Fatal,
            SecretScoutError::EventProcessing(e) => match e {
                EventError::NoCommits => ErrorSeverity::Expected,
                _ => ErrorSeverity::Fatal,
            },
            SecretScoutError::BinaryManagement(e) => match e {
                BinaryError::GitleaksError { exit_code: 1, .. } => ErrorSeverity::Fatal,
                BinaryError::GitleaksError { exit_code: 2, .. } => ErrorSeverity::Expected,
                _ => ErrorSeverity::Fatal,
            },
            SecretScoutError::SarifProcessing(_) => ErrorSeverity::Fatal,
            SecretScoutError::LicenseValidation(_) => ErrorSeverity::Fatal,

            // Non-fatal errors - log warning and continue
            SecretScoutError::GitHubApi(e) => match e {
                GitHubError::CommentFailed { .. } => ErrorSeverity::NonFatal,
                GitHubError::NotFound { .. } => ErrorSeverity::NonFatal,
                _ => ErrorSeverity::Fatal,
            },

            // Expected conditions
            SecretScoutError::Expected(_) => ErrorSeverity::Expected,
        }
    }
}
```

### 2.3 Fatal vs Non-Fatal Classification

#### Fatal Errors (Exit Immediately with Code 1)

**Configuration Errors:**
- Missing `GITHUB_TOKEN` for PR events
- Missing `GITLEAKS_LICENSE` for organizations (when feature enabled)
- Invalid environment variable values
- Path traversal attempts

**Event Processing Errors:**
- Unsupported event type
- Malformed event JSON
- Missing required event fields

**Binary Management Errors:**
- Unsupported platform/architecture
- Binary download failure
- Binary extraction failure
- Binary execution failure (exit code 1)

**SARIF Processing Errors:**
- SARIF file not found (after gitleaks succeeded)
- Invalid SARIF structure
- Failed to parse SARIF JSON

**GitHub API Errors:**
- Authentication failure
- Failed to fetch PR commits (blocks scan range)
- Max retries exceeded for critical operations

**License Validation Errors:**
- Missing license for organization
- License validation failure
- License limit exceeded

#### Non-Fatal Errors (Log Warning, Continue)

**GitHub API Errors:**
- Failed to post individual PR comment (large diff)
- Failed to fetch existing comments for deduplication
- Account info lookup failure (assume organization)

**Cache Errors:**
- Cache read failure (download fresh)
- Cache write failure (continue without caching)

#### Expected Conditions (Not True Errors)

**Event Conditions:**
- Empty commit list in push event (exit 0)
- Gitleaks exit code 2 (secrets found - process results then exit 1)

---

## 3. ERROR HANDLING PATTERNS

### 3.1 Result Type Usage

```rust
/// Standard Result type for SecretScout operations
pub type Result<T> = std::result::Result<T, SecretScoutError>;

/// Example function signature with Result
pub fn parse_event_context(config: &Config) -> Result<EventContext> {
    let event_data = read_event_file(&config.event_path)
        .map_err(|e| EventError::FileRead {
            path: config.event_path.clone(),
            source: e,
        })?;

    let event_json: serde_json::Value = serde_json::from_str(&event_data)
        .map_err(|e| EventError::JsonParse {
            reason: e.to_string(),
        })?;

    // Continue processing...
    Ok(event_context)
}
```

### 3.2 Error Propagation with Context

```rust
/// Add context when propagating errors
pub fn obtain_gitleaks_binary(config: &Config) -> Result<PathBuf> {
    let version = resolve_version(&config.gitleaks_version)?;

    let platform = detect_platform()
        .map_err(|e| BinaryError::UnsupportedPlatform {
            platform: std::env::consts::OS.to_string(),
        })?;

    let arch = detect_architecture()
        .map_err(|e| BinaryError::UnsupportedArchitecture {
            arch: std::env::consts::ARCH.to_string(),
        })?;

    // Check cache first
    if let Some(cached_path) = check_cache(&version, &platform, &arch) {
        return Ok(cached_path);
    }

    // Download and extract
    let url = build_download_url(&version, &platform, &arch);
    download_and_extract(&url, &version)
        .map_err(|e| BinaryError::DownloadFailed {
            url: url.clone(),
            source: e,
        })
}
```

### 3.3 Option Type Usage

```rust
/// Use Option for values that may legitimately be absent
pub struct Config {
    pub github_token: String,
    pub gitleaks_license: Option<String>,  // Only for orgs
    pub gitleaks_config_path: Option<PathBuf>,  // Auto-detected
    pub base_ref: Option<String>,  // Override
    // ...
}

/// Convert Option to Result when absence is an error
pub fn get_required_env(key: &str) -> Result<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ConfigError::MissingEnvVar {
            variable: key.to_string(),
        }.into())
}

/// Use Option when absence is acceptable
pub fn get_optional_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
}
```

### 3.4 Error Conversion

```rust
/// Implement From trait for automatic error conversion
impl From<std::io::Error> for SecretScoutError {
    fn from(error: std::io::Error) -> Self {
        // Context should be added at call site, not here
        // This is a fallback for unexpected I/O errors
        SecretScoutError::Configuration(ConfigError::InvalidConfig {
            path: "unknown".to_string(),
            reason: error.to_string(),
        })
    }
}

/// Manual error conversion with context
pub fn parse_sarif_file(path: &Path) -> Result<SarifReport> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| SarifError::FileRead {
            path: path.display().to_string(),
            source: e,
        })?;

    let sarif: SarifReport = serde_json::from_str(&content)
        .map_err(|e| SarifError::JsonParse { source: e })?;

    Ok(sarif)
}
```

### 3.5 Early Return Pattern

```rust
/// Use early returns for validation
pub fn validate_git_reference(git_ref: &str) -> Result<()> {
    if git_ref.is_empty() {
        return Err(EventError::InvalidGitRef {
            reference: git_ref.to_string(),
            reason: "Reference cannot be empty".to_string(),
        }.into());
    }

    // Check for shell metacharacters
    const DANGEROUS_CHARS: &[char] = &[';', '&', '|', '$', '`', '\n', '\r'];
    if git_ref.chars().any(|c| DANGEROUS_CHARS.contains(&c)) {
        return Err(EventError::InvalidGitRef {
            reference: git_ref.to_string(),
            reason: "Contains dangerous shell characters".to_string(),
        }.into());
    }

    // Check for path traversal
    if git_ref.contains("..") {
        return Err(EventError::InvalidGitRef {
            reference: git_ref.to_string(),
            reason: "Contains path traversal sequence".to_string(),
        }.into());
    }

    Ok(())
}
```

---

## 4. RECOVERY STRATEGIES

### 4.1 Retry with Exponential Backoff

```rust
/// Retry configuration
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,  // 1 second
            max_delay_ms: 10_000,  // 10 seconds
        }
    }
}

/// Retry wrapper for fallible operations
pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> std::result::Result<T, E>
where
    F: Fn() -> std::result::Result<T, E>,
    E: std::fmt::Display,
{
    let mut attempt = 0;

    loop {
        attempt += 1;

        match operation() {
            Ok(result) => return Ok(result),
            Err(error) => {
                if attempt >= config.max_attempts {
                    log::error!(
                        "Operation '{}' failed after {} attempts: {}",
                        operation_name,
                        attempt,
                        error
                    );
                    return Err(error);
                }

                let delay_ms = std::cmp::min(
                    config.base_delay_ms * (2_u64.pow(attempt - 1)),
                    config.max_delay_ms,
                );

                log::warn!(
                    "Operation '{}' failed (attempt {}/{}): {}. Retrying in {}ms",
                    operation_name,
                    attempt,
                    config.max_attempts,
                    error,
                    delay_ms
                );

                std::thread::sleep(std::time::Duration::from_millis(delay_ms));
            }
        }
    }
}
```

### 4.2 Fallback Strategies

```rust
/// Cache with fallback to download
pub fn get_gitleaks_binary(
    version: &str,
    platform: &str,
    arch: &str,
) -> Result<PathBuf> {
    // Primary: Try cache
    if let Some(cached_path) = try_cache(version, platform, arch) {
        log::info!("Using cached gitleaks binary");
        return Ok(cached_path);
    }

    // Fallback: Download fresh
    log::info!("Cache miss, downloading gitleaks binary");
    download_gitleaks(version, platform, arch)
}

/// Account type detection with fallback
pub fn determine_account_type(
    token: &str,
    username: &str,
) -> Result<AccountType> {
    match fetch_account_info(token, username) {
        Ok(info) => Ok(info.account_type),
        Err(e) => {
            log::warn!(
                "Failed to fetch account info for '{}': {}. Assuming organization.",
                username,
                e
            );
            // Conservative fallback: Assume organization (requires license)
            Ok(AccountType::Organization)
        }
    }
}
```

### 4.3 Graceful Degradation

```rust
/// Post PR comments with graceful degradation
pub fn post_pr_comments(
    config: &Config,
    event_context: &EventContext,
    findings: &[Finding],
) -> Result<CommentStats> {
    let pr = event_context.pull_request.as_ref()
        .ok_or_else(|| GitHubError::NotFound {
            resource: "pull_request".to_string(),
        })?;

    // Try to fetch existing comments for deduplication
    let existing_comments = match fetch_existing_comments(config, pr.number) {
        Ok(comments) => comments,
        Err(e) => {
            log::warn!("Failed to fetch existing comments: {}. Proceeding without deduplication.", e);
            Vec::new()  // Graceful degradation
        }
    };

    let mut stats = CommentStats::default();

    for finding in findings {
        // Skip duplicates
        if is_duplicate(&existing_comments, finding) {
            stats.skipped += 1;
            continue;
        }

        // Attempt to post comment
        match post_single_comment(config, pr.number, finding) {
            Ok(_) => {
                stats.posted += 1;
                log::debug!("Posted comment on {}:{}", finding.file_path, finding.line_number);
            }
            Err(e) => {
                stats.failed += 1;
                log::warn!(
                    "Failed to post comment on {}:{}: {}. Continuing with other comments.",
                    finding.file_path,
                    finding.line_number,
                    e
                );
                // Non-fatal: Continue posting other comments
            }
        }
    }

    log::info!(
        "Comment stats: {} posted, {} skipped, {} failed",
        stats.posted,
        stats.skipped,
        stats.failed
    );

    Ok(stats)
}
```

### 4.4 Partial Success Handling

```rust
/// Process results with partial success tracking
pub struct ProcessingResult<T> {
    pub successes: Vec<T>,
    pub failures: Vec<(String, SecretScoutError)>,
}

impl<T> ProcessingResult<T> {
    pub fn is_complete_success(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn has_partial_success(&self) -> bool {
        !self.successes.is_empty() && !self.failures.is_empty()
    }

    pub fn is_complete_failure(&self) -> bool {
        self.successes.is_empty() && !self.failures.is_empty()
    }
}

/// Extract findings with partial success
pub fn extract_findings(sarif: &SarifReport) -> ProcessingResult<Finding> {
    let mut result = ProcessingResult {
        successes: Vec::new(),
        failures: Vec::new(),
    };

    for run in &sarif.runs {
        for sarif_result in &run.results {
            match extract_single_finding(sarif_result) {
                Ok(finding) => result.successes.push(finding),
                Err(e) => {
                    let id = sarif_result.rule_id.clone().unwrap_or_else(|| "unknown".to_string());
                    result.failures.push((id, e));
                }
            }
        }
    }

    if !result.failures.is_empty() {
        log::warn!(
            "Failed to extract {} out of {} findings",
            result.failures.len(),
            result.successes.len() + result.failures.len()
        );
    }

    result
}
```

---

## 5. WASM BOUNDARY ERROR HANDLING

### 5.1 WASM-JavaScript Error Bridge

```rust
/// WASM-compatible error representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WasmError {
    /// Error type identifier
    pub error_type: String,

    /// Human-readable error message
    pub message: String,

    /// Additional context fields
    pub context: std::collections::HashMap<String, String>,

    /// Error severity
    pub severity: String,  // "fatal", "non_fatal", "expected"
}

impl From<SecretScoutError> for WasmError {
    fn from(error: SecretScoutError) -> Self {
        let severity = match error.severity() {
            ErrorSeverity::Fatal => "fatal",
            ErrorSeverity::NonFatal => "non_fatal",
            ErrorSeverity::Expected => "expected",
        };

        let (error_type, context) = match &error {
            SecretScoutError::Configuration(e) => {
                let mut ctx = std::collections::HashMap::new();
                if let ConfigError::MissingEnvVar { variable } = e {
                    ctx.insert("variable".to_string(), variable.clone());
                }
                ("configuration".to_string(), ctx)
            }
            SecretScoutError::GitHubApi(e) => {
                let mut ctx = std::collections::HashMap::new();
                if let GitHubError::CommentFailed { file, line, .. } = e {
                    ctx.insert("file".to_string(), file.clone());
                    ctx.insert("line".to_string(), line.to_string());
                }
                ("github_api".to_string(), ctx)
            }
            _ => (error.to_string(), std::collections::HashMap::new()),
        };

        WasmError {
            error_type,
            message: error.to_string(),
            context,
            severity: severity.to_string(),
        }
    }
}

/// Convert WasmError back to JavaScript Error
#[cfg(target_arch = "wasm32")]
impl From<WasmError> for wasm_bindgen::JsValue {
    fn from(error: WasmError) -> Self {
        let js_error = js_sys::Error::new(&error.message);

        // Attach custom properties
        js_sys::Reflect::set(
            &js_error,
            &"errorType".into(),
            &error.error_type.into(),
        ).unwrap_or_default();

        js_sys::Reflect::set(
            &js_error,
            &"severity".into(),
            &error.severity.into(),
        ).unwrap_or_default();

        js_error.into()
    }
}
```

### 5.2 WASM Entry Point Error Handling

```rust
/// WASM entry point with error handling
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_secretscout(config_json: &str) -> Result<JsValue, JsValue> {
    // Set panic hook for better error messages
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    // Parse configuration
    let config: Config = serde_json::from_str(config_json)
        .map_err(|e| {
            let wasm_error = WasmError {
                error_type: "configuration".to_string(),
                message: format!("Failed to parse configuration: {}", e),
                context: std::collections::HashMap::new(),
                severity: "fatal".to_string(),
            };
            JsValue::from(wasm_error)
        })?;

    // Run main logic
    match run_main(&config) {
        Ok(result) => {
            // Serialize success result
            serde_wasm_bindgen::to_value(&result)
                .map_err(|e| {
                    let wasm_error = WasmError {
                        error_type: "serialization".to_string(),
                        message: format!("Failed to serialize result: {}", e),
                        context: std::collections::HashMap::new(),
                        severity: "fatal".to_string(),
                    };
                    JsValue::from(wasm_error)
                })
        }
        Err(error) => {
            // Convert to WasmError and return as JsValue
            Err(JsValue::from(WasmError::from(error)))
        }
    }
}
```

### 5.3 JavaScript Wrapper Error Handling

```javascript
// JavaScript wrapper in dist/index.js
const { run_secretscout } = require('./secretscout.js');

async function main() {
    try {
        // Prepare configuration
        const config = {
            github_token: process.env.GITHUB_TOKEN,
            event_path: process.env.GITHUB_EVENT_PATH,
            // ... other config
        };

        // Call WASM module
        const result = await run_secretscout(JSON.stringify(config));

        // Handle success
        if (result.exit_code === 0) {
            console.log('✅ No secrets detected');
            process.exit(0);
        } else if (result.exit_code === 2) {
            console.log('🛑 Secrets detected');
            // Write summary, upload artifacts, etc.
            process.exit(1);  // Fail workflow
        }
    } catch (error) {
        // Handle WASM errors
        if (error.severity === 'fatal') {
            console.error('❌ Fatal error:', error.message);
            if (error.context) {
                console.error('Context:', JSON.stringify(error.context, null, 2));
            }
            process.exit(1);
        } else if (error.severity === 'expected') {
            console.log('ℹ️', error.message);
            process.exit(0);
        } else {
            // Unknown error
            console.error('❌ Unexpected error:', error);
            process.exit(1);
        }
    }
}

main();
```

### 5.4 Error Serialization Constraints

**WASM Limitations:**
- No stack traces (limited support in WASM)
- Error objects must be serializable
- No `std::io::Error` directly (not `Serialize`)

**Solution:**
```rust
/// Wrapper for non-serializable errors
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SerializableError {
    pub message: String,
    pub kind: String,
}

impl From<std::io::Error> for SerializableError {
    fn from(error: std::io::Error) -> Self {
        Self {
            message: error.to_string(),
            kind: format!("{:?}", error.kind()),
        }
    }
}

/// Use in error types that cross WASM boundary
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SarifError {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub io_error: Option<SerializableError>,
}
```

---

## 6. COMPONENT ERROR STRATEGIES

### 6.1 Configuration Module

**Error Strategy:**
- All errors are fatal (can't proceed without valid config)
- Validate early, fail fast
- Provide clear guidance on what's missing/invalid

```rust
pub fn load_configuration() -> Result<Config> {
    // Required environment variables
    let workspace = get_required_env("GITHUB_WORKSPACE")?;
    let event_path = get_required_env("GITHUB_EVENT_PATH")?;
    let event_name = get_required_env("GITHUB_EVENT_NAME")?;

    // Conditional requirements
    let github_token = std::env::var("GITHUB_TOKEN").ok();
    if event_name == "pull_request" && github_token.is_none() {
        return Err(ConfigError::MissingEnvVar {
            variable: "GITHUB_TOKEN".to_string(),
        }.into());
    }

    // Validate paths
    validate_path(&workspace)?;
    validate_path(&event_path)?;

    // Parse boolean flags with defaults
    let enable_summary = parse_boolean_env("GITLEAKS_ENABLE_SUMMARY", true);

    Ok(Config {
        workspace,
        event_path,
        event_name,
        github_token,
        enable_summary,
        // ...
    })
}
```

### 6.2 Event Routing Module

**Error Strategy:**
- Unsupported events: Fatal
- Malformed JSON: Fatal
- Empty commits (push): Expected condition (exit 0)
- Missing optional fields: Use defaults

```rust
pub fn parse_event_context(config: &Config) -> Result<EventContext> {
    let event_data = std::fs::read_to_string(&config.event_path)
        .map_err(|e| EventError::FileRead {
            path: config.event_path.clone(),
            source: e,
        })?;

    let event_json: serde_json::Value = serde_json::from_str(&event_data)
        .map_err(|e| EventError::JsonParse {
            reason: e.to_string(),
        })?;

    match config.event_name.as_str() {
        "push" => parse_push_event(&event_json, config),
        "pull_request" => parse_pull_request_event(&event_json, config),
        "workflow_dispatch" => parse_workflow_dispatch_event(&event_json, config),
        "schedule" => parse_schedule_event(&event_json, config),
        unsupported => Err(EventError::UnsupportedEvent {
            event_type: unsupported.to_string(),
        }.into()),
    }
}

pub fn parse_push_event(
    event_json: &serde_json::Value,
    config: &Config,
) -> Result<EventContext> {
    let commits = event_json["commits"].as_array()
        .ok_or_else(|| EventError::MissingField {
            field: "commits".to_string(),
        })?;

    if commits.is_empty() {
        // Expected condition - not a true error
        return Err(EventError::NoCommits.into());
    }

    // Continue parsing...
    Ok(event_context)
}
```

### 6.3 Binary Management Module

**Error Strategy:**
- Platform detection failure: Fatal
- Download failure: Fatal with retry (3 attempts)
- Cache failure: Non-fatal (download fresh)
- Execution failure (exit 1): Fatal
- Secrets detected (exit 2): Expected condition

```rust
pub fn obtain_gitleaks_binary(config: &Config) -> Result<PathBuf> {
    let version = resolve_version(&config.gitleaks_version)?;
    let platform = detect_platform()?;
    let arch = detect_architecture()?;

    // Try cache (non-fatal if fails)
    match try_cache(&version, &platform, &arch) {
        Ok(Some(path)) => {
            log::info!("Using cached gitleaks binary: {}", path.display());
            return Ok(path);
        }
        Ok(None) => {
            log::info!("Cache miss, downloading binary");
        }
        Err(e) => {
            log::warn!("Cache error: {}. Downloading fresh binary.", e);
        }
    }

    // Download with retry
    let url = build_download_url(&version, &platform, &arch);
    let retry_config = RetryConfig::default();

    retry_with_backoff(
        || download_binary(&url),
        &retry_config,
        "download gitleaks binary",
    )
    .map_err(|e| BinaryError::DownloadFailed {
        url: url.clone(),
        source: e,
    }.into())
}

pub fn execute_gitleaks(
    binary_path: &Path,
    args: &[String],
) -> Result<GitleaksResult> {
    let output = std::process::Command::new(binary_path)
        .args(args)
        .output()
        .map_err(|e| BinaryError::ExecutionFailed {
            command: format!("{} {}", binary_path.display(), args.join(" ")),
            source: e,
        })?;

    match output.status.code() {
        Some(0) => Ok(GitleaksResult::NoSecrets),
        Some(2) => Ok(GitleaksResult::SecretsFound),
        Some(1) => Err(BinaryError::GitleaksError {
            exit_code: 1,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }.into()),
        Some(code) => Err(BinaryError::GitleaksError {
            exit_code: code,
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }.into()),
        None => Err(BinaryError::ExecutionFailed {
            command: format!("{} {}", binary_path.display(), args.join(" ")),
            source: std::io::Error::new(
                std::io::ErrorKind::Other,
                "Process terminated by signal",
            ),
        }.into()),
    }
}
```

### 6.4 SARIF Processing Module

**Error Strategy:**
- File not found: Fatal
- Invalid JSON: Fatal
- Invalid structure: Fatal
- Missing optional fields: Use defaults, log warning

```rust
pub fn parse_sarif_report(sarif_path: &Path) -> Result<SarifReport> {
    if !sarif_path.exists() {
        return Err(SarifError::FileNotFound {
            path: sarif_path.display().to_string(),
        }.into());
    }

    let content = std::fs::read_to_string(sarif_path)
        .map_err(|e| SarifError::FileRead {
            path: sarif_path.display().to_string(),
            source: e,
        })?;

    let sarif: SarifReport = serde_json::from_str(&content)
        .map_err(|e| SarifError::JsonParse { source: e })?;

    // Validate structure
    if sarif.runs.is_empty() {
        return Err(SarifError::InvalidStructure {
            reason: "No runs in SARIF report".to_string(),
        }.into());
    }

    Ok(sarif)
}

pub fn extract_finding(sarif_result: &SarifResult) -> Result<Finding> {
    // Required fields
    let rule_id = sarif_result.rule_id.as_ref()
        .ok_or_else(|| SarifError::MissingField {
            field: "ruleId".to_string(),
        })?;

    let location = sarif_result.locations.first()
        .ok_or_else(|| SarifError::InvalidStructure {
            reason: "No locations in result".to_string(),
        })?;

    // Optional fields with defaults
    let commit_sha = sarif_result.partial_fingerprints
        .as_ref()
        .and_then(|fp| fp.commit_sha.as_ref())
        .cloned()
        .unwrap_or_else(|| {
            log::warn!("Missing commitSha in partialFingerprints for rule {}", rule_id);
            "unknown".to_string()
        });

    Ok(Finding {
        rule_id: rule_id.clone(),
        commit_sha,
        // ...
    })
}
```

### 6.5 PR Comment Module

**Error Strategy:**
- Not a PR event: Fatal
- Failed to fetch existing comments: Non-fatal (skip deduplication)
- Failed to post individual comment: Non-fatal (log, continue with others)
- All comments failed: Non-fatal (findings still in summary/artifacts)

```rust
pub fn post_pr_comments(
    github_client: &GitHubClient,
    pr_number: u32,
    findings: &[Finding],
) -> Result<CommentStats> {
    // Fetch existing comments (non-fatal)
    let existing_comments = match github_client.fetch_pr_comments(pr_number) {
        Ok(comments) => comments,
        Err(e) => {
            log::warn!("Failed to fetch existing comments: {}. Skipping deduplication.", e);
            Vec::new()
        }
    };

    let mut stats = CommentStats::default();

    for finding in findings {
        if is_duplicate(&existing_comments, finding) {
            stats.skipped += 1;
            continue;
        }

        // Post comment (non-fatal)
        match github_client.post_comment(pr_number, finding) {
            Ok(_) => {
                stats.posted += 1;
            }
            Err(GitHubError::CommentFailed { file, line, source }) => {
                stats.failed += 1;
                log::warn!(
                    "Failed to post comment on {}:{}: {}",
                    file, line, source
                );
            }
            Err(e) => {
                stats.failed += 1;
                log::warn!("Unexpected error posting comment: {}", e);
            }
        }
    }

    log::info!(
        "Posted {} comments, skipped {} duplicates, {} failed",
        stats.posted, stats.skipped, stats.failed
    );

    Ok(stats)
}
```

### 6.6 GitHub API Module

**Error Strategy:**
- Authentication failure: Fatal
- Rate limit exceeded: Retry with backoff
- Not found: Context-dependent (fatal for PR commits, non-fatal for account info)
- Network errors: Retry with backoff
- Max retries exceeded: Fatal

```rust
pub async fn fetch_pr_commits(
    client: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u32,
) -> Result<Vec<Commit>> {
    let retry_config = RetryConfig::default();

    retry_with_backoff(
        || {
            // Make API call
            let result = client
                .pulls(owner, repo)
                .list_commits(pr_number)
                .send()
                .await;

            match result {
                Ok(commits) => Ok(commits),
                Err(octocrab::Error::GitHub { source, .. })
                    if source.message.contains("rate limit") => {
                    Err(GitHubError::RateLimitExceeded { retry_after: 60 })
                }
                Err(e) => Err(GitHubError::RequestFailed {
                    endpoint: format!("/repos/{}/{}/pulls/{}/commits", owner, repo, pr_number),
                    source: e,
                }),
            }
        },
        &retry_config,
        "fetch PR commits",
    )
    .await
    .map_err(|e| GitHubError::MaxRetriesExceeded {
        operation: "fetch PR commits".to_string(),
    }.into())
}
```

---

## 7. LOGGING ARCHITECTURE

### 7.1 Log Levels

```rust
/// Log level mapping
///
/// ERROR: Fatal errors that cause immediate exit
/// WARN: Non-fatal errors and degraded functionality
/// INFO: Normal execution progress and expected conditions
/// DEBUG: Detailed execution information for troubleshooting
/// TRACE: Very detailed internal state (development only)
```

### 7.2 Structured Logging

```rust
/// Use structured logging with context
use log::{error, warn, info, debug, trace};

pub fn process_finding(finding: &Finding) -> Result<()> {
    debug!(
        "Processing finding: rule={}, file={}, line={}",
        finding.rule_id,
        finding.file_path,
        finding.line_number
    );

    match do_processing(finding) {
        Ok(_) => {
            info!("Successfully processed finding: {}", finding.fingerprint);
            Ok(())
        }
        Err(e) => {
            error!(
                "Failed to process finding {}: {}",
                finding.fingerprint,
                e
            );
            Err(e)
        }
    }
}
```

### 7.3 GitHub Actions Log Commands

```rust
/// GitHub Actions log command formatting
pub struct GitHubActionsLogger;

impl GitHubActionsLogger {
    pub fn error(message: &str) {
        eprintln!("::error::{}", message);
    }

    pub fn warning(message: &str) {
        println!("::warning::{}", message);
    }

    pub fn notice(message: &str) {
        println!("::notice::{}", message);
    }

    pub fn debug(message: &str) {
        if std::env::var("RUNNER_DEBUG").unwrap_or_default() == "1" {
            println!("::debug::{}", message);
        }
    }

    pub fn group(name: &str) {
        println!("::group::{}", name);
    }

    pub fn end_group() {
        println!("::endgroup::");
    }
}

/// Usage in error handling
pub fn log_error(error: &SecretScoutError) {
    let message = format!("{}", error);

    // Standard logging
    log::error!("{}", message);

    // GitHub Actions annotation
    GitHubActionsLogger::error(&message);

    // Add context if available
    if let Some(context) = error.context() {
        log::debug!("Error context: {:?}", context);
    }
}
```

### 7.4 Log Sanitization

```rust
/// Sanitize logs to prevent secret leakage
pub struct SecretSanitizer;

impl SecretSanitizer {
    /// Mask sensitive values in log messages
    pub fn sanitize(message: &str, secrets: &[&str]) -> String {
        let mut sanitized = message.to_string();

        for secret in secrets {
            if !secret.is_empty() {
                sanitized = sanitized.replace(secret, "***REDACTED***");
            }
        }

        sanitized
    }

    /// Mask GitHub token
    pub fn mask_token(token: &str) -> String {
        if token.len() > 8 {
            format!("{}...{}", &token[..4], &token[token.len()-4..])
        } else {
            "***".to_string()
        }
    }
}

/// Safe error logging
pub fn log_error_safe(error: &SecretScoutError, config: &Config) {
    let message = error.to_string();

    // Sanitize sensitive values
    let secrets = vec![
        config.github_token.as_str(),
        config.gitleaks_license.as_deref().unwrap_or(""),
    ];

    let sanitized = SecretSanitizer::sanitize(&message, &secrets);

    log::error!("{}", sanitized);
    GitHubActionsLogger::error(&sanitized);
}
```

### 7.5 Debug Information

```rust
/// Capture debug context for error investigation
pub struct DebugContext {
    pub environment: Vec<(String, String)>,
    pub files: Vec<(String, bool)>,  // (path, exists)
    pub process_info: ProcessInfo,
}

impl DebugContext {
    pub fn capture() -> Self {
        Self {
            environment: Self::capture_environment(),
            files: Self::check_files(),
            process_info: ProcessInfo::current(),
        }
    }

    fn capture_environment() -> Vec<(String, String)> {
        // Safe environment variables (no secrets)
        let safe_vars = vec![
            "GITHUB_WORKSPACE",
            "GITHUB_EVENT_NAME",
            "GITHUB_EVENT_PATH",
            "GITHUB_REPOSITORY",
            "RUNNER_OS",
            "RUNNER_ARCH",
        ];

        safe_vars.iter()
            .filter_map(|var| {
                std::env::var(var)
                    .ok()
                    .map(|val| (var.to_string(), val))
            })
            .collect()
    }

    fn check_files() -> Vec<(String, bool)> {
        let important_files = vec![
            "results.sarif",
            "gitleaks.toml",
            ".gitleaksignore",
        ];

        important_files.iter()
            .map(|file| {
                let path = std::env::var("GITHUB_WORKSPACE")
                    .map(|ws| format!("{}/{}", ws, file))
                    .unwrap_or_else(|_| file.to_string());
                (path.clone(), std::path::Path::new(&path).exists())
            })
            .collect()
    }

    pub fn log(&self) {
        log::debug!("=== Debug Context ===");
        log::debug!("Environment:");
        for (key, value) in &self.environment {
            log::debug!("  {}={}", key, value);
        }
        log::debug!("Files:");
        for (path, exists) in &self.files {
            log::debug!("  {} [{}]", path, if *exists { "exists" } else { "missing" });
        }
        log::debug!("Process: {:?}", self.process_info);
        log::debug!("=== End Debug Context ===");
    }
}

#[derive(Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub cwd: String,
}

impl ProcessInfo {
    fn current() -> Self {
        Self {
            pid: std::process::id(),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}
```

---

## 8. USER-FACING ERROR MESSAGES

### 8.1 Error Message Principles

1. **Be Specific**: Tell users exactly what went wrong
2. **Be Actionable**: Provide clear next steps
3. **Be Safe**: Never leak secrets in error messages
4. **Be Helpful**: Include links to documentation when relevant

### 8.2 Error Message Templates

```rust
impl SecretScoutError {
    /// Get user-facing error message with actionable guidance
    pub fn user_message(&self) -> String {
        match self {
            // Configuration errors
            SecretScoutError::Configuration(ConfigError::MissingEnvVar { variable }) => {
                format!(
                    "Missing required environment variable: {}\n\
                     \n\
                     Action required:\n\
                     - Add '{}' to your workflow secrets or environment variables\n\
                     - See: https://docs.github.com/en/actions/security-guides/encrypted-secrets",
                    variable, variable
                )
            }

            SecretScoutError::Configuration(ConfigError::PathValidation { path, reason }) => {
                format!(
                    "Path validation failed: {}\n\
                     Reason: {}\n\
                     \n\
                     This could indicate:\n\
                     - A path traversal attempt (security protection)\n\
                     - An incorrect file path in configuration\n\
                     \n\
                     Ensure all paths are within GITHUB_WORKSPACE.",
                    path, reason
                )
            }

            // Event processing errors
            SecretScoutError::EventProcessing(EventError::UnsupportedEvent { event_type }) => {
                format!(
                    "Unsupported GitHub event type: {}\n\
                     \n\
                     SecretScout supports:\n\
                     - push\n\
                     - pull_request\n\
                     - workflow_dispatch\n\
                     - schedule\n\
                     \n\
                     Check your workflow trigger configuration.",
                    event_type
                )
            }

            SecretScoutError::EventProcessing(EventError::NoCommits) => {
                "No commits found in push event.\n\
                 This is expected for empty pushes. Skipping scan.".to_string()
            }

            // Binary management errors
            SecretScoutError::BinaryManagement(BinaryError::UnsupportedPlatform { platform }) => {
                format!(
                    "Unsupported platform: {}\n\
                     \n\
                     Gitleaks binaries are available for:\n\
                     - Linux (linux)\n\
                     - macOS (darwin)\n\
                     - Windows (windows)\n\
                     \n\
                     If you're using a self-hosted runner, ensure it runs a supported OS.",
                    platform
                )
            }

            SecretScoutError::BinaryManagement(BinaryError::DownloadFailed { url, .. }) => {
                format!(
                    "Failed to download gitleaks binary from:\n\
                     {}\n\
                     \n\
                     Possible causes:\n\
                     - Network connectivity issues\n\
                     - GitHub releases temporarily unavailable\n\
                     - Incorrect version specified\n\
                     \n\
                     Try:\n\
                     1. Retry the workflow (transient network error)\n\
                     2. Check if specified version exists at: https://github.com/zricethezav/gitleaks/releases\n\
                     3. Use default version (8.24.3) by not setting GITLEAKS_VERSION",
                    url
                )
            }

            SecretScoutError::BinaryManagement(BinaryError::GitleaksError { exit_code, stderr }) => {
                format!(
                    "Gitleaks execution failed with exit code: {}\n\
                     \n\
                     Error output:\n\
                     {}\n\
                     \n\
                     Possible causes:\n\
                     - Invalid gitleaks configuration file\n\
                     - Git repository issues\n\
                     - Insufficient permissions\n\
                     \n\
                     See gitleaks documentation: https://github.com/zricethezav/gitleaks",
                    exit_code, stderr
                )
            }

            // SARIF processing errors
            SecretScoutError::SarifProcessing(SarifError::FileNotFound { path }) => {
                format!(
                    "SARIF report not found: {}\n\
                     \n\
                     This usually indicates:\n\
                     - Gitleaks completed but didn't generate a report\n\
                     - Incorrect report path configuration\n\
                     \n\
                     Expected: Gitleaks creates 'results.sarif' in GITHUB_WORKSPACE",
                    path
                )
            }

            SecretScoutError::SarifProcessing(SarifError::InvalidStructure { reason }) => {
                format!(
                    "Invalid SARIF report structure: {}\n\
                     \n\
                     This could mean:\n\
                     - Gitleaks version incompatibility\n\
                     - Corrupted report file\n\
                     \n\
                     Try:\n\
                     - Use a tested gitleaks version (8.24.3)\n\
                     - Check gitleaks logs for warnings",
                    reason
                )
            }

            // GitHub API errors
            SecretScoutError::GitHubApi(GitHubError::Authentication { .. }) => {
                "GitHub API authentication failed.\n\
                 \n\
                 Action required:\n\
                 - Ensure GITHUB_TOKEN is set correctly\n\
                 - For pull_request events, token is required\n\
                 - Use: secrets.GITHUB_TOKEN in workflow\n\
                 \n\
                 Example:\n\
                 env:\n\
                   GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}".to_string()
            }

            SecretScoutError::GitHubApi(GitHubError::RateLimitExceeded { retry_after }) => {
                format!(
                    "GitHub API rate limit exceeded.\n\
                     Retry after: {} seconds\n\
                     \n\
                     This is usually temporary. The action will retry automatically.\n\
                     If this persists, check:\n\
                     - Number of API calls in your workflow\n\
                     - Other workflows consuming API quota",
                    retry_after
                )
            }

            SecretScoutError::GitHubApi(GitHubError::CommentFailed { file, line, .. }) => {
                format!(
                    "Failed to post PR comment on {}:{}.\n\
                     \n\
                     This is non-fatal. Possible causes:\n\
                     - File not in PR diff (large diff limitation)\n\
                     - Line number outside diff context\n\
                     \n\
                     The secret is still reported in the job summary and artifacts.",
                    file, line
                )
            }

            // License validation errors
            SecretScoutError::LicenseValidation(LicenseError::MissingLicense) => {
                "License key required for organization accounts.\n\
                 \n\
                 Action required:\n\
                 - Add GITLEAKS_LICENSE environment variable\n\
                 - Set it to your gitleaks license key\n\
                 \n\
                 Note: Personal accounts do not require a license.\n\
                 Get a license at: https://gitleaks.io".to_string()
            }

            SecretScoutError::LicenseValidation(LicenseError::LimitExceeded) => {
                "License limit exceeded (too many machines).\n\
                 \n\
                 Your license has reached its machine limit.\n\
                 \n\
                 Action required:\n\
                 - Contact support to increase limit\n\
                 - Or deactivate unused machines\n\
                 \n\
                 Visit: https://gitleaks.io/dashboard".to_string()
            }

            // Default
            _ => format!("{}", self),
        }
    }
}
```

### 8.3 Error Message Formatting

```rust
/// Format error for GitHub Actions display
pub fn format_error_for_github(error: &SecretScoutError) -> String {
    let mut output = String::new();

    // Header with emoji
    output.push_str("❌ SecretScout Error\n\n");

    // User message
    output.push_str(&error.user_message());
    output.push_str("\n\n");

    // Technical details (collapsed)
    output.push_str("<details>\n");
    output.push_str("<summary>Technical Details</summary>\n\n");
    output.push_str("```\n");
    output.push_str(&format!("{:?}", error));
    output.push_str("\n```\n");
    output.push_str("</details>\n");

    output
}

/// Write error to GitHub step summary
pub fn write_error_summary(error: &SecretScoutError) -> Result<()> {
    let summary_path = std::env::var("GITHUB_STEP_SUMMARY")
        .ok()
        .filter(|s| !s.is_empty());

    if let Some(path) = summary_path {
        let formatted = format_error_for_github(error);
        std::fs::write(&path, formatted)?;
    }

    Ok(())
}
```

---

## 9. EXIT CODE MAPPING

### 9.1 Exit Code Definitions

```rust
/// Exit codes for SecretScout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum ExitCode {
    /// Success - no secrets detected
    Success = 0,

    /// Error occurred during execution
    Error = 1,

    /// Secrets detected (internal code, mapped to 1 after processing)
    SecretsDetected = 2,
}

impl ExitCode {
    /// Convert to i32 for process exit
    pub fn as_i32(self) -> i32 {
        self as i32
    }

    /// Map to final exit code for GitHub Actions
    ///
    /// Note: Exit code 2 (secrets detected) is mapped to 1 after
    /// processing results (comments, summary, artifacts) to ensure
    /// workflow fails.
    pub fn to_workflow_exit_code(self) -> i32 {
        match self {
            ExitCode::Success => 0,
            ExitCode::Error => 1,
            ExitCode::SecretsDetected => 1,  // Fail workflow
        }
    }
}

impl From<GitleaksResult> for ExitCode {
    fn from(result: GitleaksResult) -> Self {
        match result {
            GitleaksResult::NoSecrets => ExitCode::Success,
            GitleaksResult::SecretsFound => ExitCode::SecretsDetected,
            GitleaksResult::Error => ExitCode::Error,
        }
    }
}
```

### 9.2 Exit Code Flow

```rust
pub fn main() -> ExitCode {
    match run_main() {
        Ok(result) => {
            match result.exit_code {
                ExitCode::Success => {
                    log::info!("✅ No secrets detected");
                    ExitCode::Success
                }
                ExitCode::SecretsDetected => {
                    log::error!("🛑 Secrets detected");
                    // Process results BEFORE returning failure
                    if let Err(e) = process_results(&result) {
                        log::error!("Failed to process results: {}", e);
                    }
                    ExitCode::Error  // Return failure to fail workflow
                }
                ExitCode::Error => {
                    log::error!("❌ Execution error");
                    ExitCode::Error
                }
            }
        }
        Err(error) => {
            // Log error
            log_error_safe(&error, &get_config());

            // Write error summary
            if let Err(e) = write_error_summary(&error) {
                log::warn!("Failed to write error summary: {}", e);
            }

            // Determine exit code from error severity
            match error.severity() {
                ErrorSeverity::Fatal => ExitCode::Error,
                ErrorSeverity::Expected => ExitCode::Success,
                ErrorSeverity::NonFatal => ExitCode::Success,
            }
        }
    }
}
```

### 9.3 Exit Code Documentation

| Exit Code | Meaning | GitHub Actions Status | When Used |
|-----------|---------|----------------------|-----------|
| 0 | Success | ✅ Pass | No secrets detected OR expected condition (empty commits) |
| 1 | Error | ❌ Fail | Fatal error OR secrets detected (after processing results) |
| 2 | Internal | N/A | Secrets detected (internal code, not returned to workflow) |

**Critical Note:** Exit code 2 from gitleaks indicates secrets were found. The action MUST:
1. Parse SARIF report
2. Post PR comments (if applicable)
3. Generate job summary
4. Upload artifacts (if enabled)
5. **THEN** exit with code 1 to fail the workflow

---

## 10. ERROR CONTEXT & DIAGNOSTICS

### 10.1 Error Context Wrapper

```rust
/// Wrapper to add context to errors
pub struct ErrorContext {
    pub operation: String,
    pub details: HashMap<String, String>,
    pub timestamp: std::time::SystemTime,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            details: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }

    pub fn with_detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Result type with context
pub type ContextResult<T> = std::result::Result<T, (SecretScoutError, ErrorContext)>;

/// Extension trait to add context to Results
pub trait ResultContextExt<T> {
    fn context(self, ctx: ErrorContext) -> ContextResult<T>;
    fn with_context<F>(self, f: F) -> ContextResult<T>
    where
        F: FnOnce() -> ErrorContext;
}

impl<T, E> ResultContextExt<T> for std::result::Result<T, E>
where
    E: Into<SecretScoutError>,
{
    fn context(self, ctx: ErrorContext) -> ContextResult<T> {
        self.map_err(|e| (e.into(), ctx))
    }

    fn with_context<F>(self, f: F) -> ContextResult<T>
    where
        F: FnOnce() -> ErrorContext,
    {
        self.map_err(|e| (e.into(), f()))
    }
}

/// Usage example
pub fn download_file(url: &str) -> Result<Vec<u8>> {
    let ctx = ErrorContext::new("download_file")
        .with_detail("url", url);

    reqwest::blocking::get(url)
        .context(ctx.clone())?
        .bytes()
        .context(ctx)
        .map(|b| b.to_vec())
}
```

### 10.2 Diagnostic Information Capture

```rust
/// Capture diagnostic information for error reports
pub struct DiagnosticSnapshot {
    pub error: SecretScoutError,
    pub context: Option<ErrorContext>,
    pub debug_context: DebugContext,
    pub stack_trace: Option<String>,
}

impl DiagnosticSnapshot {
    pub fn capture(error: SecretScoutError, context: Option<ErrorContext>) -> Self {
        Self {
            error,
            context,
            debug_context: DebugContext::capture(),
            stack_trace: Self::capture_stack_trace(),
        }
    }

    fn capture_stack_trace() -> Option<String> {
        // Stack traces not well-supported in WASM
        #[cfg(not(target_arch = "wasm32"))]
        {
            Some(format!("{:?}", backtrace::Backtrace::new()))
        }

        #[cfg(target_arch = "wasm32")]
        {
            None
        }
    }

    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)
    }

    pub fn log(&self) {
        log::error!("=== Diagnostic Snapshot ===");
        log::error!("Error: {}", self.error);

        if let Some(ctx) = &self.context {
            log::error!("Operation: {}", ctx.operation);
            for (key, value) in &ctx.details {
                log::error!("  {}: {}", key, value);
            }
        }

        self.debug_context.log();

        if let Some(stack) = &self.stack_trace {
            log::debug!("Stack trace:\n{}", stack);
        }

        log::error!("=== End Diagnostic Snapshot ===");
    }
}
```

### 10.3 Error Reporting

```rust
/// Generate error report for GitHub Issues
pub fn generate_error_report(snapshot: &DiagnosticSnapshot) -> String {
    let mut report = String::new();

    report.push_str("## Error Report\n\n");

    // Error summary
    report.push_str("### Error\n\n");
    report.push_str("```\n");
    report.push_str(&format!("{}", snapshot.error));
    report.push_str("\n```\n\n");

    // User-facing message
    report.push_str("### Guidance\n\n");
    report.push_str(&snapshot.error.user_message());
    report.push_str("\n\n");

    // Context
    if let Some(ctx) = &snapshot.context {
        report.push_str("### Context\n\n");
        report.push_str(&format!("**Operation:** {}\n\n", ctx.operation));

        if !ctx.details.is_empty() {
            report.push_str("**Details:**\n\n");
            for (key, value) in &ctx.details {
                report.push_str(&format!("- **{}:** {}\n", key, value));
            }
            report.push_str("\n");
        }
    }

    // Environment (sanitized)
    report.push_str("### Environment\n\n");
    report.push_str("```\n");
    for (key, value) in &snapshot.debug_context.environment {
        report.push_str(&format!("{}: {}\n", key, value));
    }
    report.push_str("```\n\n");

    // File system state
    report.push_str("### Files\n\n");
    for (path, exists) in &snapshot.debug_context.files {
        let status = if *exists { "✅" } else { "❌" };
        report.push_str(&format!("{} `{}`\n", status, path));
    }
    report.push_str("\n");

    // Technical details
    report.push_str("<details>\n");
    report.push_str("<summary>Technical Details</summary>\n\n");
    report.push_str("```\n");
    report.push_str(&format!("{:#?}", snapshot.error));
    report.push_str("\n```\n");
    report.push_str("</details>\n");

    report
}
```

---

## SUMMARY

### Key Takeaways

**1. Type-Safe Error Handling**
- Use Rust's `Result<T, E>` and `Option<T>` exclusively
- No panics in production code
- Explicit error propagation with `?` operator

**2. Severity-Based Classification**
- Fatal: Exit immediately (configuration, unsupported events, execution failures)
- Non-Fatal: Log warning, continue (PR comments, cache operations)
- Expected: Normal conditions (empty commits, secrets found)

**3. Context Preservation**
- Attach context at error creation point
- Include operation details and input parameters
- Maintain error chains for debugging

**4. WASM Boundary Handling**
- Serialize errors across WASM boundary
- No stack traces in WASM (limited support)
- JavaScript wrapper handles final exit codes

**5. User-Centric Messages**
- Specific: Exactly what went wrong
- Actionable: Clear next steps
- Safe: No secret leakage
- Helpful: Links to documentation

**6. Recovery Strategies**
- Retry with exponential backoff for transient failures
- Fallback strategies (cache miss → download)
- Graceful degradation (comment failure → continue)
- Partial success tracking

**7. Comprehensive Logging**
- Structured logging with context
- GitHub Actions log commands for annotations
- Debug context capture for diagnostics
- Secret sanitization in all logs

**8. Exit Code Mapping**
- 0: Success (no secrets or expected condition)
- 1: Error (fatal error or secrets detected after processing)
- 2: Internal code (secrets detected, process results then exit 1)

### Implementation Checklist

- [ ] Define error type hierarchy with `thiserror`
- [ ] Implement severity classification
- [ ] Create `Result<T>` type alias
- [ ] Add context wrappers for errors
- [ ] Implement retry logic with backoff
- [ ] Design WASM error serialization
- [ ] Create user-facing error messages
- [ ] Implement GitHub Actions logging
- [ ] Add secret sanitization
- [ ] Define exit code constants
- [ ] Create diagnostic snapshot system
- [ ] Write error report generator
- [ ] Document error handling patterns
- [ ] Add integration tests for error paths

---

**Document Status:** ✅ COMPLETE
**Version:** 1.0
**Date:** October 16, 2025
**Phase:** Architecture
