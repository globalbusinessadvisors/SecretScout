# Gitleaks Integration Specification
## Requirements Documentation for Rust Port

**Date**: 2025-10-15
**Source Analysis**: gitleaks-action v2.x (Node.js implementation)
**Purpose**: Document current gitleaks-action behavior as requirements for SecretScout Rust implementation

---

## 1. GITLEAKS BINARY INTERFACE

### 1.1 Binary Invocation

The gitleaks binary is invoked as a command-line executable with specific arguments:

**Base Command Structure**:
```bash
gitleaks detect [OPTIONS]
```

**Note**: The `detect` command is deprecated in newer gitleaks versions. Modern equivalents:
- `gitleaks git` (for git repositories)
- `gitleaks directory` (for directories/files)
- `gitleaks stdin` (for piped input)

However, the current gitleaks-action still uses `detect` with version 8.24.3 by default.

### 1.2 Command-Line Arguments

**Required Arguments** (as used by gitleaks-action):

```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug
```

**Argument Breakdown**:

| Argument | Purpose | Value | Required |
|----------|---------|-------|----------|
| `detect` | Subcommand | - | Yes |
| `--redact` | Redact secrets from logs/stdout | Flag | Yes |
| `-v` | Verbose output | Flag | Yes |
| `--exit-code` | Exit code when leaks detected | `2` | Yes |
| `--report-format` | Output format | `sarif` | Yes |
| `--report-path` | Report file location | `results.sarif` | Yes |
| `--log-level` | Logging verbosity | `debug` | Yes |
| `--log-opts` | Git log options (conditional) | Varies | Conditional |

**Conditional Arguments**:

**For Push Events (when baseRef != headRef)**:
```bash
--log-opts=--no-merges --first-parent {baseRef}^..{headRef}
```

**For Push Events (when baseRef == headRef)**:
```bash
--log-opts=-1
```

**For Pull Request Events**:
```bash
--log-opts=--no-merges --first-parent {baseRef}^..{headRef}
```

**For Workflow Dispatch / Schedule Events**:
No `--log-opts` argument is added (scans entire repository)

### 1.3 Exit Codes

The gitleaks binary uses specific exit codes to communicate results:

| Exit Code | Meaning | Action Behavior |
|-----------|---------|-----------------|
| `0` | No leaks detected | Success - continue workflow |
| `1` | Error occurred in gitleaks | Failure - exit with error |
| `2` | Leaks detected | Warning - create reports, comments, fail workflow |

**Implementation Note**: The action uses `ignoreReturnCode: true` when executing gitleaks to capture the exit code without immediately failing.

### 1.4 Binary Installation

**Version Management**:
- Default version: `8.24.3`
- Configurable via `GITLEAKS_VERSION` environment variable
- Special value: `latest` - fetches latest release from GitHub

**Download URL Pattern**:
```
https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{platform}_{arch}.tar.gz
```

**Platform Mapping**:
- `win32` → `windows`
- `linux` → `linux`
- `darwin` → `darwin`

**Caching**:
- Cache key format: `gitleaks-cache-{version}-{platform}-{arch}`
- Install path: `{tmpdir}/gitleaks-{version}`
- Binary is added to PATH after installation

### 1.5 Execution Environment

**Environment Variables Passed to Binary** (implicitly via process environment):

- `GITLEAKS_CONFIG` - Path to custom configuration file (optional)
- `GITLEAKS_LICENSE` - License key for validation (optional, organizations only)

**Execution Options**:
```javascript
exec.exec("gitleaks", args, {
  ignoreReturnCode: true,
  delay: 60 * 1000  // 60 second timeout
})
```

---

## 2. CONFIGURATION MANAGEMENT

### 2.1 Configuration File Discovery

**Configuration Precedence Order**:
1. `--config` / `-c` command-line flag
2. `GITLEAKS_CONFIG` environment variable (file path)
3. `GITLEAKS_CONFIG_TOML` environment variable (file content as string)
4. `.gitleaks.toml` in repository root (auto-detected)
5. Default gitleaks configuration (built-in)

**Implementation Note**: The current gitleaks-action does NOT explicitly pass `--config` flag. It relies on gitleaks' built-in precedence handling via environment variables.

### 2.2 Configuration File Format

**File Format**: TOML

**Basic Structure**:
```toml
title = "Gitleaks config"

# Option 1: Define custom rules (default rules don't apply)
[[rules]]
id = "rule-name"
description = "Human readable description"
regex = '''regex pattern'''

# Option 2: Extend default configuration
[extend]
useDefault = true
# path = "path/to/base/config.toml"  # Alternative: extend from file

# Disable specific inherited rules
# disabledRules = ["rule-id-1", "rule-id-2"]

# Allowlist (false positives)
[allowlist]
description = "Allowlisted secrets"
regexes = ['''pattern1''', '''pattern2''']
paths = ['''.gitleaksignore''']
```

**Key Configuration Sections**:

1. **Rules** - Define secret detection patterns
   - `id`: Unique identifier
   - `description`: Human-readable description
   - `regex`: Golang regex pattern
   - `entropy`: (optional) Entropy threshold
   - `keywords`: (optional) Required keywords
   - `secretGroup`: (optional) Capture group for secret

2. **Extend** - Inherit from other configs
   - `useDefault`: Boolean to extend default config
   - `path`: Path to base configuration
   - `disabledRules`: Array of rule IDs to disable

3. **Allowlist** - Ignore specific findings
   - `description`: Description of allowlist
   - `regexes`: Patterns to allowlist
   - `paths`: File paths to ignore
   - `commits`: Commit SHAs to ignore
   - `stopwords`: Words that indicate false positives

### 2.3 .gitleaksignore File

**Purpose**: Ignore specific findings (false positives)

**Format**: Plain text, one fingerprint per line

**Fingerprint Format**:
```
{commitSha}:{file}:{ruleID}:{startLine}
```

**Example**:
```
abc123def456:src/config.js:aws-access-token:42
7ea7f374bd15952df1176c1bdf6ab3568e620bd1:src/keygen.js:generic-api-key:10
```

**Location**: Repository root (`.gitleaksignore`)

**Implementation**: Gitleaks automatically reads this file if present

---

## 3. OUTPUT PARSING AND PROCESSING

### 3.1 SARIF Format Structure

**SARIF Version**: 2.1.0

**Schema**: `https://json.schemastore.org/sarif-2.1.0.json`

**Complete Structure**:
```json
{
  "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "gitleaks",
          "semanticVersion": "8.24.3",
          "informationUri": "https://github.com/gitleaks/gitleaks"
        }
      },
      "results": [
        {
          "ruleId": "aws-access-token",
          "message": {
            "text": "Identified an AWS Access Token"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "src/config.js"
                },
                "region": {
                  "startLine": 42,
                  "startColumn": 10,
                  "endLine": 42,
                  "endColumn": 50,
                  "snippet": {
                    "text": "const secret = 'AKIAIOSFODNN7EXAMPLE'"
                  }
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "abc123def456...",
            "author": "John Doe",
            "email": "john@example.com",
            "date": "2025-10-15T12:00:00Z",
            "commitMessage": "Add configuration"
          }
        }
      ]
    }
  ]
}
```

### 3.2 SARIF Field Mapping

**Required Fields for Processing**:

| Field Path | Type | Purpose | Used For |
|------------|------|---------|----------|
| `runs[0].results` | Array | List of findings | Iteration |
| `results[i].ruleId` | String | Rule identifier | Comments, summaries |
| `results[i].message.text` | String | Human-readable message | Display |
| `results[i].locations[0].physicalLocation.artifactLocation.uri` | String | File path | Comments, links |
| `results[i].locations[0].physicalLocation.region.startLine` | Integer | Line number | Comments, links |
| `results[i].locations[0].physicalLocation.region.startColumn` | Integer | Column number | Optional display |
| `results[i].locations[0].physicalLocation.region.snippet.text` | String | Code snippet | Display |
| `results[i].partialFingerprints.commitSha` | String | Commit SHA | Comments, links, fingerprint |
| `results[i].partialFingerprints.author` | String | Commit author | Summary table |
| `results[i].partialFingerprints.email` | String | Author email | Summary table |
| `results[i].partialFingerprints.date` | String | Commit date | Summary table |
| `results[i].partialFingerprints.commitMessage` | String | Commit message | Optional display |

### 3.3 Fingerprint Generation

**Algorithm**:
```javascript
const fingerprint = `${commitSha}:${filePath}:${ruleId}:${startLine}`
```

**Components**:
1. `commitSha` - Full commit SHA from `partialFingerprints.commitSha`
2. `filePath` - File URI from `locations[0].physicalLocation.artifactLocation.uri`
3. `ruleId` - Rule ID from `ruleId`
4. `startLine` - Line number from `locations[0].physicalLocation.region.startLine`

**Example**:
```
cd5226711335c68be1e720b318b7bc3135a30eb2:cmd/generate/config/rules/sidekiq.go:sidekiq-secret:23
```

**Usage**: Fingerprints are used for:
- Adding to `.gitleaksignore`
- Deduplicating findings
- Identifying specific secrets across scans

### 3.4 SARIF File Location

**Fixed Path**: `results.sarif` (in current working directory)

**Reading**: After gitleaks execution completes with exit code 2

**Error Handling**: If file doesn't exist or is malformed, log error and continue

---

## 4. ERROR HANDLING SPECIFICATIONS

### 4.1 Exit Code Handling

**Main Flow**:
```
gitleaks execution → capture exit code → process based on code
```

**Exit Code Processing**:

```javascript
if (exitCode == 0) {
  // Success path
  core.info("✅ No leaks detected");
  // Continue workflow without error
}
else if (exitCode == 2) {
  // Leaks detected path
  core.warning("🛑 Leaks detected, see job summary for details");
  // Parse SARIF, create comments, generate summary
  // THEN fail the workflow: process.exit(1)
}
else {
  // Error path (exit code 1 or other)
  core.error(`ERROR: Unexpected exit code [${exitCode}]`);
  process.exit(exitCode);
}
```

**Important**: When exit code is 2, the action still fails the workflow AFTER processing results.

### 4.2 SARIF Parsing Errors

**Scenario**: `results.sarif` doesn't exist or is invalid JSON

**Handling**:
- Log error message
- Skip comment creation
- Skip summary generation
- Fail workflow appropriately based on exit code

### 4.3 GitHub API Errors

**Comment Creation Failures**:
```javascript
try {
  await octokit.rest.pulls.createReviewComment(proposedComment);
} catch (error) {
  core.warning(`Error encountered when attempting to write a comment on PR #${eventJSON.number}: ${error}
Likely an issue with too large of a diff for the comment to be written.
All secrets that have been leaked will be reported in the summary and job artifact.`);
  // Continue processing other findings
}
```

**Comment API Errors** (common scenarios):
- Large diff - comment position out of range
- File not in PR diff
- Outdated commit SHA
- Permission issues

**Handling**: Log warning, continue with other comments and summary

### 4.4 Artifact Upload Errors

```javascript
const options = {
  continueOnError: true,  // Don't fail workflow if upload fails
};
```

**Note**: Artifact upload failures are non-fatal

### 4.5 Event Type Validation

**Supported Events**:
- `push`
- `pull_request`
- `workflow_dispatch`
- `schedule`

**Unsupported Event Handling**:
```javascript
if (!supportedEvents.includes(eventType)) {
  core.error(`ERROR: The [${eventType}] event is not yet supported`);
  process.exit(1);
}
```

### 4.6 License Validation Errors

**Scenario**: Organization repo without license key

```javascript
if (shouldValidate && !process.env.GITLEAKS_LICENSE) {
  core.error("🛑 missing gitleaks license...");
  process.exit(1);
}
```

**Note**: Currently disabled in code (commented out), but framework exists

---

## 5. GITHUB API INTEGRATION

### 5.1 Authentication

**Token**: `GITHUB_TOKEN` environment variable

**Octokit Configuration**:
```javascript
const octokit = new Octokit({
  auth: process.env.GITHUB_TOKEN,
  baseUrl: process.env.GITHUB_API_URL  // Supports GitHub Enterprise
});
```

**Required for**:
- Creating PR comments
- Fetching PR commits
- User type detection (org vs personal)

### 5.2 Pull Request Comments

**API Endpoint**: `POST /repos/{owner}/{repo}/pulls/{pull_number}/comments`

**API Method**: `octokit.rest.pulls.createReviewComment()`

**Comment Object Structure**:
```javascript
{
  owner: "owner-name",
  repo: "repo-name",
  pull_number: 123,
  body: "🛑 **Gitleaks** has detected a secret...",
  commit_id: "abc123...",
  path: "src/file.js",
  side: "RIGHT",
  line: 42
}
```

**Comment Body Template**:
```markdown
🛑 **Gitleaks** has detected a secret with rule-id `{ruleId}` in commit {commitSha}.
If this secret is a _true_ positive, please rotate the secret ASAP.

If this secret is a _false_ positive, you can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch.

```
echo {fingerprint} >> .gitleaksignore
```
```

**Optional Notification**:
If `GITLEAKS_NOTIFY_USER_LIST` is set:
```markdown
cc @user1,@user2
```

### 5.3 Comment Deduplication

**Algorithm**:
1. Fetch all existing PR review comments: `GET /repos/{owner}/{repo}/pulls/{pull_number}/comments`
2. For each proposed comment, check if it already exists
3. Match on: `body`, `path`, and `original_line`
4. Skip if duplicate found

**Implementation Note**: Current implementation iterates for each comment (O(n²)). Code has TODO to optimize with dictionary.

### 5.4 GitHub Actions Summary

**API**: GitHub Actions Core library (`@actions/core`)

**Method**: `core.summary`

**When Exit Code is 0** (No leaks):
```javascript
await core.summary
  .addHeading("No leaks detected ✅")
  .write();
```

**When Exit Code is 2** (Leaks detected):
```javascript
await core.summary
  .addHeading("🛑 Gitleaks detected secrets 🛑")
  .addTable([resultsHeader, ...resultsRows])
  .write();
```

**Table Structure**:

| Column | Data Source | Format |
|--------|-------------|--------|
| Rule ID | `ruleId` | Plain text |
| Commit | `partialFingerprints.commitSha` | HTML link (first 7 chars) |
| Secret URL | Constructed | HTML link "View Secret" |
| Start Line | `region.startLine` | Plain text number |
| Author | `partialFingerprints.author` | Plain text |
| Date | `partialFingerprints.date` | Plain text |
| Email | `partialFingerprints.email` | Plain text |
| File | `artifactLocation.uri` | HTML link |

**URL Construction**:
```javascript
const commitURL = `${repo_url}/commit/${commitSha}`;
const secretURL = `${repo_url}/blob/${commitSha}/${filePath}#L${startLine}`;
const fileURL = `${repo_url}/blob/${commitSha}/${filePath}`;
```

**When Exit Code is 1** (Error):
```javascript
await core.summary
  .addHeading(`❌ Gitleaks exited with error. Exit code [${exitCode}]`)
  .write();
```

**When Exit Code is Unexpected**:
```javascript
await core.summary
  .addHeading(`❌ Gitleaks exited with unexpected exit code [${exitCode}]`)
  .write();
```

### 5.5 Artifact Upload

**API**: GitHub Actions Artifact library (`@actions/artifact`)

**Method**: `DefaultArtifactClient.uploadArtifact()`

**Upload Configuration**:
```javascript
const artifactClient = new DefaultArtifactClient();
const artifactName = "gitleaks-results.sarif";
const files = ["results.sarif"];
const rootDirectory = process.env.HOME;
const options = {
  continueOnError: true,
};

await artifactClient.uploadArtifact(
  artifactName,
  files,
  rootDirectory,
  options
);
```

**Control Variable**: `GITLEAKS_ENABLE_UPLOAD_ARTIFACT` (default: `true`)

**Artifact Availability**: Downloadable from GitHub Actions UI

### 5.6 Action Outputs

**Set via**: `core.setOutput()`

**Output Name**: `exit-code`

**Value**: Exit code from gitleaks execution (0, 1, or 2)

**Usage**: Can be referenced in subsequent workflow steps

---

## 6. EVENT-SPECIFIC BEHAVIOR

### 6.1 Push Events

**Commit Range Detection**:
```javascript
const baseRef = eventJSON.commits[0].id;  // First commit
const headRef = eventJSON.commits[eventJSON.commits.length - 1].id;  // Last commit
```

**Override**: Environment variable `BASE_REF` can override `baseRef`

**Empty Commits Handling**:
```javascript
if (eventJSON.commits.length === 0) {
  core.info("No commits to scan");
  process.exit(0);
}
```

**Gitleaks Arguments**:
- If `baseRef == headRef`: `--log-opts=-1` (scan single commit)
- If `baseRef != headRef`: `--log-opts=--no-merges --first-parent {baseRef}^..{headRef}`

### 6.2 Pull Request Events

**Commit Fetching**:
```javascript
let commits = await octokit.request(
  "GET /repos/{owner}/{repo}/pulls/{pull_number}/commits",
  {
    owner: owner,
    repo: repo,
    pull_number: eventJSON.number,
  }
);

const baseRef = commits.data[0].sha;  // First commit in PR
const headRef = commits.data[commits.data.length - 1].sha;  // Last commit in PR
```

**Override**: Environment variable `BASE_REF` can override `baseRef`

**Gitleaks Arguments**:
```bash
--log-opts=--no-merges --first-parent {baseRef}^..{headRef}
```

**Additional Processing**:
- Create PR review comments (if enabled)
- Comment deduplication
- User notifications

**Comment Control**: `GITLEAKS_ENABLE_COMMENTS` environment variable (default: `true`)

### 6.3 Workflow Dispatch Events

**Behavior**: Full repository scan

**No Additional Arguments**: Scans entire Git history

**Use Case**: Manual/on-demand scans

### 6.4 Schedule Events

**Behavior**: Full repository scan

**Event JSON Handling**:
```javascript
// eventJSON.repository is undefined for scheduled events
githubUsername = process.env.GITHUB_REPOSITORY_OWNER;
eventJSON.repository = {
  owner: {
    login: process.env.GITHUB_REPOSITORY_OWNER,
  },
  full_name: process.env.GITHUB_REPOSITORY,
};
```

**No Additional Arguments**: Scans entire Git history

**Use Case**: Periodic/recurring scans (e.g., daily)

---

## 7. ENVIRONMENT VARIABLES

### 7.1 Required Variables

| Variable | Source | Purpose | Required For |
|----------|--------|---------|--------------|
| `GITHUB_TOKEN` | Auto-generated | GitHub API authentication | All PR events |
| `GITHUB_EVENT_PATH` | Auto-generated | Path to event JSON | All events |
| `GITHUB_EVENT_NAME` | Auto-generated | Event type | All events |
| `GITHUB_REPOSITORY` | Auto-generated | Repository name | All events |
| `GITHUB_REPOSITORY_OWNER` | Auto-generated | Repository owner | All events |
| `GITHUB_API_URL` | Auto-generated | GitHub API URL | All events |

### 7.2 Optional Configuration Variables

| Variable | Default | Purpose | Type |
|----------|---------|---------|------|
| `GITLEAKS_LICENSE` | None | License key (orgs only) | String |
| `GITLEAKS_VERSION` | `8.24.3` | Gitleaks version to use | String |
| `GITLEAKS_CONFIG` | None | Path to config file | String (path) |
| `GITLEAKS_ENABLE_SUMMARY` | `true` | Enable job summary | Boolean |
| `GITLEAKS_ENABLE_UPLOAD_ARTIFACT` | `true` | Upload SARIF artifact | Boolean |
| `GITLEAKS_ENABLE_COMMENTS` | `true` | Enable PR comments | Boolean |
| `GITLEAKS_NOTIFY_USER_LIST` | None | Users to notify | String (comma-separated) |
| `BASE_REF` | Auto-detected | Override base commit | String (SHA) |

### 7.3 Boolean Environment Variable Parsing

**False Values**:
```javascript
if (process.env.VAR == "false" || process.env.VAR == 0)
```

**True Values**: Any other value (or unset = true for defaults)

### 7.4 User Notification Format

**Variable**: `GITLEAKS_NOTIFY_USER_LIST`

**Format**: Comma-separated GitHub usernames with `@` prefix

**Example**: `@octocat,@zricethezav,@gitleaks`

**Spaces**: Allowed (will be trimmed)

---

## 8. SCANNING MODES AND OPTIONS

### 8.1 Incremental Scan (Push/PR)

**Mode**: Git commit range

**Command**: `gitleaks detect --log-opts=...`

**Efficiency**: Only scans commits in range

**Use Cases**:
- Push events
- Pull request events

### 8.2 Full Repository Scan

**Mode**: Entire Git history

**Command**: `gitleaks detect` (no --log-opts)

**Use Cases**:
- Workflow dispatch
- Schedule events

### 8.3 Git Log Options

**Purpose**: Control which commits are scanned

**Format**: Standard git log options

**Common Options**:
- `--no-merges`: Skip merge commits
- `--first-parent`: Follow only first parent on merge commits
- `{base}^..{head}`: Commit range (exclusive base, inclusive head)
- `-1`: Last commit only

### 8.4 Redaction

**Flag**: `--redact`

**Purpose**: Prevent secrets from appearing in logs/stdout

**Always Enabled**: Yes (security best practice)

### 8.5 Verbosity

**Flag**: `-v` (verbose)

**Log Level**: `--log-level=debug`

**Purpose**: Detailed output for debugging

**Always Enabled**: Yes (for troubleshooting)

---

## 9. REPORT FORMATTING AND FILTERING

### 9.1 Output Format

**Fixed Format**: SARIF (`--report-format=sarif`)

**Rationale**:
- Structured format
- Industry standard
- Rich metadata
- Easy to parse

**Alternative Formats** (not used by action):
- JSON
- CSV
- JUnit

### 9.2 Report File Path

**Fixed Path**: `results.sarif`

**Location**: Current working directory (repository root)

**Lifetime**: Temporary (per action run)

### 9.3 Result Filtering

**No Built-in Filtering**: Action processes all results from SARIF

**Filtering Mechanisms**:
1. `.gitleaksignore` file (pre-scan filtering by gitleaks)
2. Configuration allowlist (pre-scan filtering by gitleaks)
3. Comment deduplication (post-scan filtering by action)

### 9.4 Result Transformation

**SARIF → Summary Table**:
- Extract relevant fields
- Generate URLs
- Format as HTML table
- Add to GitHub Actions summary

**SARIF → PR Comments**:
- Generate fingerprint
- Create comment body with template
- Check for duplicates
- Post via GitHub API

**SARIF → Artifact**:
- Upload raw SARIF file
- No transformation

---

## 10. IMPLEMENTATION RECOMMENDATIONS FOR RUST

### 10.1 Binary Execution

**Rust Crates**:
- `std::process::Command` for execution
- `tokio::process::Command` for async execution

**Key Requirements**:
- Capture stdout/stderr
- Get exit code without failing
- Pass environment variables
- Set working directory
- Handle timeouts (60 seconds)

### 10.2 SARIF Parsing

**Rust Crates**:
- `serde` + `serde_json` for JSON parsing
- Consider `serde_sarif` crate for typed SARIF structures

**Struct Definitions** (minimal):
```rust
#[derive(Debug, Deserialize)]
struct SarifReport {
    runs: Vec<SarifRun>,
}

#[derive(Debug, Deserialize)]
struct SarifRun {
    results: Vec<SarifResult>,
}

#[derive(Debug, Deserialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
    #[serde(rename = "partialFingerprints")]
    partial_fingerprints: PartialFingerprints,
}

#[derive(Debug, Deserialize)]
struct PartialFingerprints {
    #[serde(rename = "commitSha")]
    commit_sha: String,
    author: String,
    email: String,
    date: String,
    #[serde(rename = "commitMessage")]
    commit_message: Option<String>,
}
```

### 10.3 GitHub API Integration

**Rust Crates**:
- `octocrab` - Official GitHub API client
- `reqwest` - HTTP client (if custom API calls needed)

**Authentication**:
```rust
let token = std::env::var("GITHUB_TOKEN")?;
let octocrab = octocrab::Octobrab::builder()
    .personal_token(token)
    .base_url(github_api_url)?
    .build()?;
```

### 10.4 Configuration Management

**Rust Crates**:
- `toml` - TOML parsing (for .gitleaks.toml)
- `serde` - Serialization/deserialization

**Configuration Loading**:
```rust
fn find_config() -> Option<PathBuf> {
    // 1. Check GITLEAKS_CONFIG env var
    if let Ok(path) = env::var("GITLEAKS_CONFIG") {
        return Some(PathBuf::from(path));
    }

    // 2. Check for .gitleaks.toml in repo root
    let default_path = PathBuf::from(".gitleaks.toml");
    if default_path.exists() {
        return Some(default_path);
    }

    // 3. Return None (use gitleaks default)
    None
}
```

### 10.5 Error Handling

**Rust Patterns**:
- Use `Result<T, E>` for all fallible operations
- Define custom error types with `thiserror`
- Use `anyhow` for application-level error handling

**Exit Code Mapping**:
```rust
enum ScanResult {
    Success,           // exit code 0
    LeaksDetected,     // exit code 2
    Error(String),     // exit code 1 or other
}
```

### 10.6 Environment Variable Parsing

**Rust Stdlib**: `std::env`

**Boolean Parsing**:
```rust
fn parse_bool_env(var: &str, default: bool) -> bool {
    match env::var(var).as_deref() {
        Ok("false") | Ok("0") => false,
        Ok(_) => true,
        Err(_) => default,
    }
}
```

### 10.7 Logging and Output

**Rust Crates**:
- `log` - Logging facade
- `env_logger` - Logger implementation
- Consider GitHub Actions annotations format

**GitHub Actions Output**:
```rust
// Set output
println!("::set-output name=exit-code::{}", exit_code);

// Log levels
println!("::debug::Debug message");
println!("::warning::Warning message");
println!("::error::Error message");
```

### 10.8 Async/Await

**Recommended Runtime**: `tokio`

**Key Async Operations**:
- HTTP requests (GitHub API)
- File I/O (SARIF reading)
- Process execution (gitleaks binary)

### 10.9 Testing Strategy

**Unit Tests**:
- SARIF parsing logic
- Fingerprint generation
- URL construction
- Configuration loading

**Integration Tests**:
- Mock GitHub API responses
- Test with sample SARIF files
- Event-specific behavior

**Test Fixtures**:
- Sample SARIF files
- Sample .gitleaks.toml configs
- Sample GitHub event JSONs

---

## 11. SECURITY CONSIDERATIONS

### 11.1 Secret Redaction

**Requirement**: Secrets must NEVER appear in logs

**Implementation**:
- Always use `--redact` flag
- Sanitize error messages
- Don't log SARIF contents directly

### 11.2 Token Security

**GITHUB_TOKEN**:
- Auto-generated per workflow
- Limited scope (repository access)
- Automatically expires

**GITLEAKS_LICENSE**:
- Should be stored as encrypted secret
- Never logged
- Transmitted only to license validation API

### 11.3 SARIF File Handling

**Security**:
- SARIF file contains sensitive information
- Stored temporarily
- Uploaded as artifact (access controlled)
- Deleted after action completes (by runner cleanup)

### 11.4 Comment Content

**Review Comments**:
- Use redacted format
- Don't include full secret in comment
- Include fingerprint for .gitleaksignore
- Provide remediation instructions

---

## 12. PERFORMANCE CONSIDERATIONS

### 12.1 Caching

**Gitleaks Binary**:
- Cache key: `gitleaks-cache-{version}-{platform}-{arch}`
- Significant time savings (avoid download)

**Future**: Consider caching scan results between runs

### 12.2 Incremental Scanning

**Optimization**: Use commit ranges for push/PR events

**Full Scans**: Reserved for scheduled/dispatch events

### 12.3 API Rate Limiting

**GitHub API**:
- Comment deduplication requires API call
- Fetching PR commits requires API call
- Consider batching/pagination for large PRs

### 12.4 Timeout Handling

**Current**: 60 second delay parameter

**Recommendation**: Make configurable for large repos

---

## 13. FEATURE FLAGS AND TOGGLES

| Feature | Variable | Default | Impact When Disabled |
|---------|----------|---------|---------------------|
| PR Comments | `GITLEAKS_ENABLE_COMMENTS` | `true` | Skip comment creation |
| Job Summary | `GITLEAKS_ENABLE_SUMMARY` | `true` | Skip summary generation |
| Artifact Upload | `GITLEAKS_ENABLE_UPLOAD_ARTIFACT` | `true` | Skip SARIF upload |

**Implementation**: Check before executing feature

```rust
if parse_bool_env("GITLEAKS_ENABLE_COMMENTS", true) {
    create_pr_comments()?;
}
```

---

## 14. COMPATIBILITY NOTES

### 14.1 GitHub Enterprise Support

**API URL**: `GITHUB_API_URL` environment variable

**Default**: `https://api.github.com`

**Enterprise**: `https://github.enterprise.com/api/v3`

**Requirement**: Support custom API URLs in Octokit/octocrab

### 14.2 Gitleaks Version Compatibility

**Tested Version**: 8.24.3

**`detect` Command**: Deprecated but still used

**Future Migration**: Plan for `gitleaks git` command

### 14.3 Node.js vs Rust

**Current Runtime**: Node.js 20

**Future Runtime**: Native Rust binary

**Benefits**:
- Faster startup
- Lower memory
- Single binary distribution
- Better performance

---

## 15. WORKFLOWS AND USE CASES

### 15.1 Standard PR Workflow

1. Developer creates PR
2. Action triggers on `pull_request` event
3. Fetch PR commits from GitHub API
4. Execute gitleaks with commit range
5. Parse SARIF results
6. Create PR review comments (if leaks found)
7. Generate job summary
8. Upload SARIF artifact
9. Fail workflow if leaks detected

### 15.2 Push to Main Workflow

1. Developer pushes to main branch
2. Action triggers on `push` event
3. Extract commit range from event JSON
4. Execute gitleaks with commit range
5. Parse SARIF results
6. Generate job summary
7. Upload SARIF artifact
8. Fail workflow if leaks detected
9. No PR comments (not a PR)

### 15.3 Scheduled Full Scan

1. Schedule triggers (e.g., daily at 4 AM)
2. Action triggers on `schedule` event
3. Execute gitleaks (full repo scan)
4. Parse SARIF results
5. Generate job summary
6. Upload SARIF artifact
7. Fail workflow if leaks detected
8. No PR comments (not a PR)

### 15.4 Manual On-Demand Scan

1. User triggers via workflow_dispatch
2. Action triggers on `workflow_dispatch` event
3. Execute gitleaks (full repo scan)
4. Parse SARIF results
5. Generate job summary
6. Upload SARIF artifact
7. Fail workflow if leaks detected
8. No PR comments (not a PR)

---

## 16. EDGE CASES AND SPECIAL HANDLING

### 16.1 Empty Commit List

**Scenario**: Push event with no commits (e.g., tag creation)

**Handling**: Exit early with success (no scan needed)

### 16.2 Large Diffs

**Scenario**: PR comment fails due to large diff

**Handling**: Log warning, continue with other comments and summary

### 16.3 Outdated Commits

**Scenario**: Commit in PR is no longer at tip

**Handling**: GitHub API may reject comment; log warning and continue

### 16.4 Fork PRs

**Scenario**: PR from fork may have limited `GITHUB_TOKEN` permissions

**Handling**: Comments may fail; summary and artifact still work

### 16.5 Multiple Secrets in Same File/Line

**Scenario**: Two rules trigger on same line

**Handling**: Create separate findings, separate comments

### 16.6 Binary Files

**Scenario**: Gitleaks scans binary files

**Handling**: Gitleaks has built-in binary file handling; action just processes results

---

## 17. MIGRATION PATH (Node.js → Rust)

### 17.1 Phase 1: Core Binary Execution
- Execute gitleaks binary
- Capture exit codes
- Parse SARIF output
- Basic logging

### 17.2 Phase 2: GitHub Integration
- GitHub API authentication
- PR comment creation
- Job summary generation
- Artifact upload

### 17.3 Phase 3: Event Handling
- Parse GitHub event JSON
- Event-specific logic (push, PR, schedule)
- Commit range detection

### 17.4 Phase 4: Configuration
- Configuration file discovery
- Environment variable parsing
- Feature flags

### 17.5 Phase 5: Advanced Features
- Comment deduplication
- User notifications
- Performance optimizations
- Comprehensive error handling

### 17.6 Testing and Validation
- Unit tests for all components
- Integration tests with mock APIs
- Real-world testing with sample repos
- Performance benchmarking

---

## 18. APPENDICES

### Appendix A: Complete Command Examples

**Push Event (Multiple Commits)**:
```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug \
  --log-opts=--no-merges --first-parent abc123^..def456
```

**Push Event (Single Commit)**:
```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug \
  --log-opts=-1
```

**Full Scan**:
```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug
```

### Appendix B: Sample Event JSONs

**Push Event**:
```json
{
  "repository": {
    "owner": { "login": "octocat" },
    "full_name": "octocat/repo",
    "html_url": "https://github.com/octocat/repo"
  },
  "commits": [
    { "id": "abc123..." },
    { "id": "def456..." }
  ]
}
```

**Pull Request Event**:
```json
{
  "repository": {
    "owner": { "login": "octocat" },
    "full_name": "octocat/repo",
    "html_url": "https://github.com/octocat/repo"
  },
  "number": 123,
  "pull_request": {
    "head": { "sha": "def456..." },
    "base": { "sha": "abc123..." }
  }
}
```

### Appendix C: GitHub API Endpoints Used

| Method | Endpoint | Purpose |
|--------|----------|---------|
| GET | `/users/{username}` | Detect org vs personal |
| GET | `/repos/{owner}/{repo}/pulls/{pull_number}/commits` | Get PR commits |
| GET | `/repos/{owner}/{repo}/pulls/{pull_number}/comments` | Get existing comments |
| POST | `/repos/{owner}/{repo}/pulls/{pull_number}/comments` | Create review comment |
| GET | `/repos/{owner}/{repo}/releases/latest` | Get latest gitleaks release |

### Appendix D: Key File Locations

| File | Location | Purpose |
|------|----------|---------|
| `results.sarif` | Current working directory | Scan results |
| `.gitleaks.toml` | Repository root | Configuration |
| `.gitleaksignore` | Repository root | Ignore list |
| `gitleaks` binary | Temp dir / PATH | Scanner binary |

### Appendix E: Dependencies (Rust Equivalents)

| Node.js Package | Rust Equivalent | Purpose |
|----------------|-----------------|---------|
| `@actions/core` | Custom implementation | GitHub Actions integration |
| `@actions/exec` | `std::process::Command` | Process execution |
| `@actions/cache` | GitHub Actions cache API | Caching |
| `@actions/artifact` | GitHub Actions artifact API | Artifact upload |
| `@actions/tool-cache` | Custom implementation | Tool download/extraction |
| `@octokit/rest` | `octocrab` | GitHub API client |
| `fs` | `std::fs` | File system operations |
| `https` | `reqwest` | HTTP client |

---

## CONCLUSION

This specification documents the complete behavior of gitleaks-action v2 as implemented in Node.js. It provides comprehensive requirements for a Rust port, covering:

- Binary execution and argument passing
- Configuration management
- SARIF parsing and processing
- Error handling strategies
- GitHub API integration patterns
- Event-specific workflows
- Security and performance considerations

The Rust implementation should maintain behavioral compatibility while taking advantage of Rust's performance, safety, and type system features.

**Key Success Criteria**:
1. ✅ Execute gitleaks binary correctly for all event types
2. ✅ Parse SARIF output accurately
3. ✅ Create PR comments with proper formatting
4. ✅ Generate GitHub Actions summaries
5. ✅ Handle errors gracefully
6. ✅ Support all configuration options
7. ✅ Maintain security best practices
8. ✅ Match or exceed Node.js performance

**Document Version**: 1.0
**Last Updated**: 2025-10-15
