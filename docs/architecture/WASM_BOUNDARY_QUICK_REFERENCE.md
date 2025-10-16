# WASM BOUNDARY QUICK REFERENCE

**Project:** SecretScout - Rust Port of gitleaks-action
**Component:** Quick Reference Guide
**Date:** October 16, 2025

---

## WHAT GOES WHERE?

### WASM Module (Rust) - Pure Computation

**✅ Implement in WASM:**

```rust
// Configuration parsing and validation
fn parse_config(json: &str) -> Result<Config, Error>
fn validate_config(config: &Config) -> Result<(), Error>

// Event routing and parsing
fn parse_event_context(event_json: &str, config: &Config) -> Result<EventContext, Error>
fn determine_event_type(event_name: &str) -> Result<EventType, Error>

// Command building
fn build_gitleaks_args(config: &Config, context: &EventContext) -> Vec<String>

// SARIF parsing and transformation
fn parse_sarif(sarif_json: &str) -> Result<SarifReport, Error>
fn extract_findings(sarif: &SarifReport) -> Vec<Finding>
fn generate_fingerprint(finding: &Finding) -> String

// Content generation
fn build_pr_comment_body(finding: &Finding, users: &[String]) -> String
fn is_duplicate_comment(existing: &[Comment], new: &Finding) -> bool
fn generate_job_summary_html(findings: &[Finding], repo: &Repository) -> String

// Input validation
fn validate_git_ref(git_ref: &str) -> Result<(), Error>
fn validate_path(path: &str, workspace: &str) -> Result<(), Error>
fn sanitize_for_logging(text: &str, config: &Config) -> String
```

**❌ NEVER in WASM:**
- File system access
- Network requests
- Process execution
- Environment variable access
- Direct GitHub API calls

---

### JavaScript Wrapper - I/O Operations

**✅ Implement in JavaScript:**

```javascript
// File operations
async function readFile(path) { return await fs.readFile(path, 'utf-8'); }
async function writeFile(path, content) { await fs.writeFile(path, content); }
async function fileExists(path) { /* check file exists */ }

// Process execution
async function execGitleaks(args) {
  const exitCode = await exec.exec('gitleaks', args);
  return { exitCode, stdout, stderr };
}

// Network operations
async function httpGet(url, headers) { /* fetch */ }
async function httpPost(url, body, headers) { /* fetch */ }

// GitHub API
async function fetchPrCommits(prNumber) {
  return await octokit.rest.pulls.listCommits({ owner, repo, pull_number: prNumber });
}
async function fetchPrComments(prNumber) { /* ... */ }
async function postPrComment(prNumber, comment) { /* ... */ }

// Logging
function logInfo(message) { core.info(message); }
function logError(message) { core.error(message); }

// Cache
async function checkCache(key) { /* ... */ }
async function saveCache(key, path) { /* ... */ }

// GitHub Actions SDK
core.setOutput('exit-code', exitCode);
await fs.appendFile(process.env.GITHUB_STEP_SUMMARY, summary);
await artifactClient.uploadArtifact('gitleaks-results.sarif', files, workspace);
```

**❌ NEVER in JavaScript:**
- Business logic (parsing, validation, transformation)
- Content generation (PR comments, summaries)
- Fingerprint calculation
- Deduplication logic

---

## DATA SERIALIZATION PATTERNS

### Rust → JavaScript (Serialize)

```rust
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
pub struct ActionOutput {
    pub exit_code: u8,
    pub summary: Option<String>,
    pub artifact_ready: bool,
}

// Return as JsValue
#[wasm_bindgen]
pub fn run_action() -> Result<JsValue, JsValue> {
    let output = ActionOutput {
        exit_code: 0,
        summary: Some("## Success".to_string()),
        artifact_ready: false,
    };

    Ok(serde_wasm_bindgen::to_value(&output)?)
}
```

### JavaScript → Rust (Deserialize)

```rust
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub github_token: String,
    pub github_workspace: String,
    pub enable_summary: bool,
}

#[wasm_bindgen]
pub fn run_action(config_json: &str) -> Result<JsValue, JsValue> {
    let config: Config = serde_json::from_str(config_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    // Use config...
    Ok(JsValue::NULL)
}
```

---

## ERROR HANDLING PATTERNS

### Rust Error Definition

```rust
use serde::Serialize;

#[derive(Serialize)]
pub struct WasmError {
    pub error: String,
    pub code: String,
    pub severity: String,  // "fatal" or "warning"
    pub context: serde_json::Value,
}

impl WasmError {
    pub fn fatal(code: &str, message: &str) -> Self {
        Self {
            error: message.to_string(),
            code: code.to_string(),
            severity: "fatal".to_string(),
            context: serde_json::json!({}),
        }
    }
}

// Convert to JsValue
impl From<WasmError> for JsValue {
    fn from(err: WasmError) -> Self {
        serde_wasm_bindgen::to_value(&err).unwrap()
    }
}
```

### JavaScript Error Handling

```javascript
try {
  const result = await wasm.run_action(config, callbacks);
  const output = JSON.parse(result);
  // Handle success
} catch (error) {
  if (error.code) {
    // Structured WASM error
    core.error(`[${error.code}] ${error.error}`);
    if (error.severity === 'fatal') {
      process.exit(1);
    }
  } else {
    // Unexpected error
    core.error(`Unexpected: ${error.message}`);
    process.exit(1);
  }
}
```

---

## CALLBACK PATTERN

### Rust Side (Trait Definition)

```rust
use async_trait::async_trait;

#[async_trait(?Send)]  // WASM is single-threaded
pub trait IoCallbacks {
    async fn read_file(&self, path: &str) -> Result<String, WasmError>;
    async fn exec_gitleaks(&self, args: Vec<String>) -> Result<ExecResult, WasmError>;
    fn log_info(&self, message: &str);
}

// Use in implementation
async fn run_action_impl(config: Config, io: impl IoCallbacks) -> Result<ActionOutput, WasmError> {
    // Read file via callback
    let event_json = io.read_file(&config.event_path).await?;

    // Execute process via callback
    let result = io.exec_gitleaks(args).await?;

    // Log via callback
    io.log_info("Processing complete");

    Ok(output)
}
```

### JavaScript Side (Implementation)

```javascript
const callbacks = {
  readFile: async (path) => {
    return await fs.readFile(path, 'utf-8');
  },

  execGitleaks: async (args) => {
    let stdout = '', stderr = '';
    const exitCode = await exec.exec('gitleaks', args, {
      listeners: {
        stdout: (data) => { stdout += data.toString(); },
        stderr: (data) => { stderr += data.toString(); }
      }
    });
    return { exitCode, stdout, stderr };
  },

  logInfo: (message) => {
    core.info(message);
  }
};

// Pass to WASM
await wasm.run_action(JSON.stringify(config), callbacks);
```

---

## COMMON PATTERNS

### Pattern 1: Parse JSON Input

```rust
#[wasm_bindgen]
pub fn process_event(event_json: &str) -> Result<JsValue, JsValue> {
    // Deserialize
    let event: PushEvent = serde_json::from_str(event_json)
        .map_err(|e| WasmError::fatal("EVENT_PARSE_ERROR", &e.to_string()))?;

    // Process
    let result = process_push_event(&event)?;

    // Serialize and return
    Ok(serde_wasm_bindgen::to_value(&result)?)
}
```

### Pattern 2: Call JavaScript Callback

```rust
async fn download_gitleaks(io: &impl IoCallbacks, version: &str) -> Result<String, WasmError> {
    let url = format!("https://github.com/.../gitleaks_{}.tar.gz", version);

    // Call JS to perform HTTP request
    let response = io.http_get(&url, &HashMap::new()).await?;

    if response.status_code != 200 {
        return Err(WasmError::fatal(
            "DOWNLOAD_FAILED",
            &format!("HTTP {}", response.status_code)
        ));
    }

    Ok(response.body)
}
```

### Pattern 3: Generate HTML Output

```rust
pub fn generate_summary(findings: &[Finding], repo: &Repository) -> String {
    let mut html = String::with_capacity(findings.len() * 200);

    html.push_str("## 🛑 Gitleaks detected secrets 🛑\n\n<table>\n");
    html.push_str("<tr><th>Rule</th><th>File</th><th>Line</th></tr>\n");

    for finding in findings {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            escape_html(&finding.rule_id),
            escape_html(&finding.file_path),
            finding.line_number
        ));
    }

    html.push_str("</table>\n");
    html
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
```

### Pattern 4: Input Validation

```rust
pub fn validate_config(config: &Config) -> Result<(), WasmError> {
    // Validate git reference
    if let Some(ref base_ref) = config.base_ref {
        if base_ref.contains(&[';', '&', '|', '$'][..]) {
            return Err(WasmError::fatal(
                "CONFIG_INVALID",
                "Invalid git reference: contains shell metacharacters"
            ));
        }
    }

    // Validate path
    if config.workspace.contains("..") {
        return Err(WasmError::fatal(
            "CONFIG_INVALID",
            "Invalid workspace: contains path traversal"
        ));
    }

    Ok(())
}
```

---

## PERFORMANCE TIPS

### ✅ DO:

```rust
// Pre-allocate strings
let mut output = String::with_capacity(findings.len() * 200);

// Use references to avoid clones
fn process_findings(findings: &[Finding]) -> Vec<String> {
    findings.iter().map(|f| f.fingerprint.clone()).collect()
}

// Use &str when possible
fn validate_ref(git_ref: &str) -> bool {
    !git_ref.contains(&[';', '&'][..])
}

// Avoid unnecessary allocations
fn build_url(base: &str, path: &str) -> String {
    format!("{}/{}", base, path)  // OK: single allocation
}
```

### ❌ DON'T:

```rust
// Avoid unnecessary clones
let findings_copy = findings.clone();  // BAD: clones entire vector

// Avoid intermediate allocations
let s1 = format!("{}", x);
let s2 = format!("{}", s1);  // BAD: two allocations

// Avoid repeated string conversions
for i in 0..100 {
    let s = i.to_string();  // BAD: 100 allocations
}
```

---

## TESTING PATTERNS

### Rust Unit Test

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_config() {
        let json = r#"{
            "github_token": "token",
            "github_workspace": "/workspace",
            "enable_summary": true
        }"#;

        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.github_token, "token");
        assert!(config.enable_summary);
    }
}
```

### Rust Integration Test with Mock

```rust
#[cfg(test)]
mod integration {
    struct MockCallbacks {
        files: HashMap<String, String>,
    }

    #[async_trait(?Send)]
    impl IoCallbacks for MockCallbacks {
        async fn read_file(&self, path: &str) -> Result<String, WasmError> {
            self.files.get(path)
                .cloned()
                .ok_or_else(|| WasmError::fatal("FILE_NOT_FOUND", ""))
        }

        fn log_info(&self, _: &str) {}
    }

    #[tokio::test]
    async fn test_push_event() {
        let mut callbacks = MockCallbacks {
            files: HashMap::new(),
        };

        callbacks.files.insert(
            "/event.json".to_string(),
            r#"{"commits": [{"id": "abc"}]}"#.to_string()
        );

        let result = process_event(config, callbacks).await;
        assert!(result.is_ok());
    }
}
```

---

## SECURITY CHECKLIST

### Input Validation

- [ ] Validate all git references (no shell metacharacters)
- [ ] Validate all file paths (no path traversal)
- [ ] Validate repository names (valid format)
- [ ] Validate JSON structure (required fields present)
- [ ] Validate numeric ranges (line numbers, etc.)

### Secret Protection

- [ ] Never log `github_token` value
- [ ] Never log `gitleaks_license` value
- [ ] Sanitize error messages before returning
- [ ] Redact secrets in all log output
- [ ] Use `--redact` flag for gitleaks

### WASM Isolation

- [ ] No direct file system access
- [ ] No direct network access
- [ ] No direct process execution
- [ ] All I/O through callbacks only
- [ ] No shared memory with JavaScript

---

## COMMON PITFALLS

### ❌ Don't Pass Callbacks as Strings

```rust
// BAD: Can't pass callbacks as JSON
#[wasm_bindgen]
pub fn run_action(config_json: &str, callbacks_json: &str) -> ... {
    // Callbacks must be JavaScript objects!
}

// GOOD: Use JsValue for callbacks
#[wasm_bindgen]
pub fn run_action(config_json: &str, callbacks: JsValue) -> ... {
    // ✅ Correct
}
```

### ❌ Don't Forget Async

```rust
// BAD: Callbacks are async in JavaScript
fn read_file(&self, path: &str) -> Result<String, WasmError> {
    // Won't work with JS async functions
}

// GOOD: Use async/await
async fn read_file(&self, path: &str) -> Result<String, WasmError> {
    // ✅ Correct
}
```

### ❌ Don't Panic in WASM

```rust
// BAD: Panics are hard to debug in WASM
let config: Config = serde_json::from_str(json).unwrap();

// GOOD: Return errors
let config: Config = serde_json::from_str(json)
    .map_err(|e| WasmError::fatal("PARSE_ERROR", &e.to_string()))?;
```

---

## BUILD COMMANDS

### Build WASM

```bash
# Development build
wasm-pack build --target nodejs --dev

# Release build
wasm-pack build --target nodejs --release

# Optimized release
wasm-pack build --target nodejs --release
wasm-opt -Oz dist/secretscout_bg.wasm -o dist/secretscout_bg.wasm
```

### Test

```bash
# Rust unit tests
cargo test

# WASM tests
wasm-pack test --node

# JavaScript integration tests
npm test
```

---

## DEBUGGING

### Enable WASM Logging

```rust
// Add to Cargo.toml
[dependencies]
console_error_panic_hook = "0.1"

// In main
#[wasm_bindgen]
pub fn init() {
    console_error_panic_hook::set_once();
}
```

### Log from WASM

```rust
use web_sys::console;

pub fn debug_log(message: &str) {
    console::log_1(&format!("[WASM] {}", message).into());
}
```

### Inspect Memory

```javascript
// Check WASM memory usage
const memory = wasm.memory;
console.log('WASM memory:', memory.buffer.byteLength);
```

---

**Quick Reference Status:** ✅ COMPLETE
**Date:** October 16, 2025
**Last Updated:** October 16, 2025
