# Gitleaks-Action Technical Specification
## Comprehensive Analysis for Rust Porting

**Repository:** gitleaks-action
**Current Implementation:** Node.js (TypeScript/JavaScript)
**Target:** Rust reimplementation
**Document Version:** 1.0
**Date:** 2025-10-15

---

## Table of Contents
1. [Executive Summary](#executive-summary)
2. [Action.yml Specification](#actionyml-specification)
3. [Environment Variables](#environment-variables)
4. [Execution Flow](#execution-flow)
5. [GitHub Actions Runtime Integration](#github-actions-runtime-integration)
6. [Gitleaks Binary Management](#gitleaks-binary-management)
7. [Scanning Modes & Logic](#scanning-modes--logic)
8. [Output Formats & Reporting](#output-formats--reporting)
9. [GitHub API Interactions](#github-api-interactions)
10. [License Validation System](#license-validation-system)
11. [Error Handling & Exit Codes](#error-handling--exit-codes)
12. [Platform & Architecture Support](#platform--architecture-support)
13. [Critical Dependencies](#critical-dependencies)
14. [Security Considerations](#security-considerations)

---

## Executive Summary

**What the Action Does:**

Gitleaks-Action is a GitHub Actions wrapper that:
1. Downloads and caches the gitleaks binary
2. Executes secret scanning on git commits/pull requests
3. Generates SARIF-formatted reports
4. Comments on pull requests when secrets are detected
5. Creates GitHub Actions job summaries
6. Uploads scan results as workflow artifacts
7. Validates commercial licenses for organization accounts

**Key Characteristics:**
- **Runtime:** Node.js 20 (specified in action.yml)
- **Entry Point:** `dist/index.js` (compiled from `src/index.js`)
- **Event Support:** push, pull_request, workflow_dispatch, schedule
- **Exit Code:** 0 (success), 1 (error/license failure), 2 (leaks detected)
- **SARIF Output:** Primary format for findings
- **Caching:** Tool cache and @actions/cache for gitleaks binary

---

## Action.yml Specification

### Current action.yml Structure

```yaml
name: Gitleaks
author: zricethezav
branding:
  icon: "git-pull-request"
  color: "purple"
description: run gitleaks on push and pull-request events
runs:
  using: "node20"
  main: "dist/index.js"
```

### Analysis

**Inputs:** NONE defined in action.yml (all configuration via environment variables)

**Outputs:**
- **Implicit Output:** `exit-code` (set via `core.setOutput("exit-code", exitCode)`)
  - Type: Number
  - Values: 0 (no leaks), 1 (error), 2 (leaks detected)
  - Set in: `/workspaces/SecretScout/gitleaks-action/src/gitleaks.js:124`

**Runtime Requirements:**
- Node.js 20 runtime
- Access to filesystem (for results.sarif, caching)
- Network access (for downloading gitleaks, GitHub API, license validation)
- Git repository context

**Branding:**
- Icon: "git-pull-request"
- Color: "purple"

---

## Environment Variables

### Required Variables

#### 1. GITHUB_TOKEN
- **Type:** Secret (GitHub-provided)
- **Purpose:** Authenticate GitHub API calls
- **Used For:**
  - Creating PR review comments
  - Fetching PR commits
  - Fetching user information (to determine org vs personal account)
- **Auto-provided:** Yes (by GitHub Actions runtime)
- **Required When:** Always for PR scanning, user type detection
- **Default:** None (must be explicitly passed)

#### 2. GITLEAKS_LICENSE
- **Type:** Secret (user-provided)
- **Purpose:** License key validation for organization repositories
- **Used For:**
  - License validation via Keygen.sh API
  - Repository activation/fingerprinting
- **Required When:** Repository owner type is "Organization"
- **Default:** None
- **Validation:** Disabled in current code (see line 124-130 in index.js)

### Optional Configuration Variables

#### 3. GITLEAKS_VERSION
- **Type:** String
- **Purpose:** Specify gitleaks binary version
- **Format:** Semantic version without 'v' prefix (e.g., "8.24.3") OR "latest"
- **Default:** "8.24.3" (hardcoded in index.js:138)
- **Behavior:**
  - If "latest": Fetches latest release from GitHub API
  - Otherwise: Downloads specified version

#### 4. GITLEAKS_CONFIG
- **Type:** String (file path)
- **Purpose:** Path to custom gitleaks TOML configuration
- **Default:** None (gitleaks uses default detection or `gitleaks.toml` at repo root)
- **Note:** NOT explicitly passed to gitleaks CLI; relies on gitleaks' auto-detection
- **Mentioned In:** README.md only, not implemented in code

#### 5. GITLEAKS_ENABLE_SUMMARY
- **Type:** Boolean (string or number)
- **Purpose:** Enable/disable GitHub Actions job summary
- **Default:** true
- **Accepted Values:** "false", 0 (to disable)
- **Used In:** index.js:14-20

#### 6. GITLEAKS_ENABLE_UPLOAD_ARTIFACT
- **Type:** Boolean (string or number)
- **Purpose:** Enable/disable SARIF artifact upload
- **Default:** true
- **Accepted Values:** "false", 0 (to disable)
- **Used In:** index.js:22-29

#### 7. GITLEAKS_ENABLE_COMMENTS
- **Type:** Boolean (string)
- **Purpose:** Enable/disable PR comments
- **Default:** true
- **Accepted Values:** "false" (to disable)
- **Used In:** gitleaks.js:187

#### 8. GITLEAKS_NOTIFY_USER_LIST
- **Type:** String (comma-separated usernames)
- **Purpose:** Mention specific GitHub users in PR comments
- **Format:** "@username1,@username2,@username3" (spaces allowed)
- **Default:** None
- **Used In:** gitleaks.js:228-229

#### 9. BASE_REF
- **Type:** String (git ref)
- **Purpose:** Override the base reference for git diff scanning
- **Default:** None (uses auto-detected base from commit history)
- **Used In:** index.js:166-169, gitleaks.js:175-178
- **Context:** Undocumented environment variable for advanced use cases

### GitHub Actions Standard Variables (Read-Only)

#### 10. GITHUB_EVENT_PATH
- **Type:** String (file path)
- **Purpose:** Path to webhook event JSON payload
- **Auto-provided:** Yes
- **Used In:** index.js:32

#### 11. GITHUB_EVENT_NAME
- **Type:** String
- **Purpose:** Type of event that triggered workflow
- **Supported Values:** "push", "pull_request", "workflow_dispatch", "schedule"
- **Auto-provided:** Yes
- **Used In:** index.js:35

#### 12. GITHUB_REPOSITORY_OWNER
- **Type:** String
- **Purpose:** Repository owner username/org name
- **Auto-provided:** Yes
- **Used In:** index.js:53, 56, 61 (for schedule events)

#### 13. GITHUB_REPOSITORY
- **Type:** String
- **Purpose:** Full repository name (owner/repo)
- **Auto-provided:** Yes
- **Used In:** index.js:58, 60-63

#### 14. GITHUB_API_URL
- **Type:** String (URL)
- **Purpose:** Base URL for GitHub API
- **Default:** https://api.github.com
- **Auto-provided:** Yes
- **Used In:** index.js:70 (Octokit initialization)

#### 15. HOME
- **Type:** String (directory path)
- **Purpose:** Home directory for artifact uploads
- **Auto-provided:** Yes
- **Used In:** gitleaks.js:136

---

## Execution Flow

### Entry Point: `src/index.js`

#### Phase 1: Initialization & Configuration Parsing

```
1. Parse GITLEAKS_ENABLE_SUMMARY (default: true)
2. Parse GITLEAKS_ENABLE_UPLOAD_ARTIFACT (default: true)
3. Read GITHUB_EVENT_PATH JSON file
4. Extract GITHUB_EVENT_NAME
5. Validate event type (push, pull_request, workflow_dispatch, schedule)
6. If unsupported event: error and exit(1)
```

#### Phase 2: Repository Owner Type Detection

```
7. Extract githubUsername from event payload or GITHUB_REPOSITORY_OWNER
8. Handle schedule events (special case: populate eventJSON.repository)
9. Initialize Octokit with GITHUB_TOKEN and GITHUB_API_URL
10. Call GitHub API: GET /users/{username}
11. Determine user type:
    - "Organization" → License required (shouldValidate = true)
    - "User" → No license required (shouldValidate = false)
    - Other → Default to requiring license
12. Handle API errors gracefully (warn but continue)
```

#### Phase 3: License Validation Check

```
13. If shouldValidate AND !GITLEAKS_LICENSE:
    - Log error message
    - Exit(1)
14. Note: License validation via Keygen is DISABLED (line 124-130 commented out)
```

#### Phase 4: Gitleaks Binary Installation

```
15. Determine version: GITLEAKS_VERSION || "8.24.3"
16. If version === "latest":
    - Call octokit.rest.repos.getLatestRelease (gitleaks/gitleaks)
    - Extract version from tag_name (strip 'v' prefix)
17. Call gitleaks.Install(version):
    - Generate cache key: "gitleaks-cache-{version}-{platform}-{arch}"
    - Try to restore from @actions/cache
    - If not cached:
      * Download from GitHub releases
      * Extract .tar.gz or .zip
      * Save to cache
    - Add binary to PATH
```

#### Phase 5: Event-Specific Scanning

##### For "push" Events:
```
18. Extract commits array from eventJSON
19. If commits.length === 0: info("No commits to scan"), exit(0)
20. scanInfo = {
      baseRef: eventJSON.commits[0].id,
      headRef: eventJSON.commits[last].id
    }
21. If BASE_REF env set: override scanInfo.baseRef
22. Call gitleaks.Scan(gitleaksEnableUploadArtifact, scanInfo, "push")
```

##### For "pull_request" Events:
```
23. Call gitleaks.ScanPullRequest(gitleaksEnableUploadArtifact, octokit, eventJSON, "pull_request")
```

##### For "workflow_dispatch" or "schedule" Events:
```
24. scanInfo = { gitleaksPath: gitleaksPath }
25. Call gitleaks.Scan(gitleaksEnableUploadArtifact, scanInfo, eventType)
```

#### Phase 6: Summary & Exit Handling

```
26. If gitleaksEnableSummary: Call summary.Write(exitCode, eventJSON)
27. Handle exitCode:
    - 0: info("✅ No leaks detected"), exit(0)
    - 2: warning("🛑 Leaks detected, see job summary"), exit(1)
    - Other: error("Unexpected exit code"), exit(exitCode)
```

---

## GitHub Actions Runtime Integration

### @actions/core Usage

#### Logging Functions
- **core.info(message)**: Informational messages
- **core.warning(message)**: Warnings (yellow in UI)
- **core.error(message)**: Errors (red in UI)
- **core.debug(message)**: Debug logs (only shown in debug mode)

#### Outputs
- **core.setOutput("exit-code", exitCode)**: Set action output
  - Location: gitleaks.js:124

#### Job Summary
- **core.summary**: Object for creating job summaries
  - Methods used:
    - `.addHeading(text)`
    - `.addTable([header, ...rows])`
    - `.write()`
  - HTML support: Yes (used for links in summary tables)

### @actions/exec Usage

#### Command Execution
```javascript
await exec.exec("gitleaks", args, {
  ignoreReturnCode: true,
  delay: 60 * 1000,  // 60 second timeout
})
```
- **Purpose:** Execute gitleaks binary
- **Options:**
  - `ignoreReturnCode: true`: Don't throw on non-zero exit
  - `delay: 60000`: 60-second timeout
- **Returns:** Exit code (0, 1, or 2)

### @actions/cache Usage

#### Binary Caching
- **cache.restoreCache([pathToInstall], cacheKey)**
  - Restore previously cached gitleaks binary
  - Returns: cache key if found, undefined otherwise

- **cache.saveCache([pathToInstall], cacheKey)**
  - Save downloaded gitleaks binary to cache
  - Key format: `gitleaks-cache-{version}-{platform}-{arch}`

### @actions/tool-cache Usage

#### Download & Extract
- **tc.downloadTool(url, destPath)**
  - Download gitleaks binary from GitHub releases
  - Returns: downloaded file path

- **tc.extractTar(file, dest)**
  - Extract .tar.gz archives

- **tc.extractZip(file, dest)**
  - Extract .zip archives (Windows support)

### @actions/artifact Usage

#### Artifact Upload
```javascript
const { DefaultArtifactClient } = require("@actions/artifact");
const artifactClient = new DefaultArtifactClient();

await artifactClient.uploadArtifact(
  "gitleaks-results.sarif",      // artifact name
  ["results.sarif"],              // files to upload
  process.env.HOME,               // root directory
  { continueOnError: true }       // options
);
```
- **Purpose:** Upload SARIF report as workflow artifact
- **Conditional:** Only if GITLEAKS_ENABLE_UPLOAD_ARTIFACT !== false

---

## Gitleaks Binary Management

### Download Source
- **Base URL:** `https://github.com/zricethezav/gitleaks/releases/download`
- **Format:** `{baseURL}/v{version}/gitleaks_{version}_{platform}_{arch}.tar.gz`

### Platform Mapping
- **win32** → **windows**
- All other platforms use Node.js `process.platform` directly
  - linux, darwin, etc.

### Architecture Support
- Uses `process.arch` directly
  - Common values: x64, arm64, arm

### Installation Path
- **Location:** `os.tmpdir()/gitleaks-{version}`
- **Caching:** Via @actions/cache with platform/arch-specific keys
- **PATH Addition:** Binary directory added to system PATH

### Version Management
- **Default:** 8.24.3 (hardcoded)
- **Latest:** Fetches from GitHub API `repos/zricethezav/gitleaks/getLatestRelease`
- **Version Extraction:** Strips 'v' prefix from tag_name

---

## Scanning Modes & Logic

### Common Gitleaks Arguments (All Scans)
```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug
```

### Push Event Scanning

**Scenario 1: Single Commit (baseRef == headRef)**
```bash
--log-opts=-1
```
- Scans only the most recent commit

**Scenario 2: Commit Range**
```bash
--log-opts=--no-merges --first-parent {baseRef}^..{headRef}
```
- Scans from baseRef (exclusive) to headRef (inclusive)
- Skips merge commits
- Follows only first parent (main branch commits)

### Pull Request Scanning

**Step 1: Fetch PR Commits**
```
GET /repos/{owner}/{repo}/pulls/{pull_number}/commits
```

**Step 2: Determine Scan Range**
```javascript
scanInfo = {
  baseRef: commits.data[0].sha,
  headRef: commits.data[last].sha
}
```

**Step 3: Execute Scan**
```bash
--log-opts=--no-merges --first-parent {baseRef}^..{headRef}
```

**Step 4: Create PR Comments** (if GITLEAKS_ENABLE_COMMENTS != "false")
- Iterate through SARIF results
- Generate fingerprint for each finding
- Check if comment already exists
- Create review comment at specific line/commit

### Workflow Dispatch / Schedule Scanning

**Arguments:** Standard arguments only (no --log-opts)
- Scans entire repository history
- No commit range filtering

### BASE_REF Override

**Purpose:** Advanced users can override base reference
- Applies to both push and pull_request events
- Set via environment variable: `BASE_REF=<git-ref>`
- Overrides auto-detected baseRef in scanInfo

---

## Output Formats & Reporting

### Primary Output: SARIF Format

**File:** `results.sarif`
**Format:** SARIF 2.1.0 (Static Analysis Results Interchange Format)
**Generated By:** Gitleaks CLI

#### SARIF Structure Used by Action
```json
{
  "runs": [
    {
      "results": [
        {
          "ruleId": "<detection-rule-id>",
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "<file-path>"
                },
                "region": {
                  "startLine": <line-number>
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "<commit-sha>",
            "author": "<author-name>",
            "date": "<commit-date>",
            "email": "<author-email>"
          }
        }
      ]
    }
  ]
}
```

#### SARIF Fields Accessed
- `sarif.runs[0].results[]`: Array of findings
- `result.ruleId`: Detection rule identifier
- `result.locations[0].physicalLocation.artifactLocation.uri`: File path
- `result.locations[0].physicalLocation.region.startLine`: Line number
- `result.partialFingerprints.commitSha`: Commit SHA
- `result.partialFingerprints.author`: Commit author
- `result.partialFingerprints.date`: Commit date
- `result.partialFingerprints.email`: Author email

### GitHub Actions Job Summary

**Generated By:** `src/summary.js`
**Conditional:** GITLEAKS_ENABLE_SUMMARY !== false

#### Exit Code 0 (No Leaks)
```
# No leaks detected ✅
```

#### Exit Code 2 (Leaks Detected)
```
# 🛑 Gitleaks detected secrets 🛑

| Rule ID | Commit | Secret URL | Start Line | Author | Date | Email | File |
|---------|--------|------------|------------|--------|------|-------|------|
| <rule>  | <link> | <link>     | <line>     | <auth> | <dt> | <em>  | <link> |
```

**Table Details:**
- **Commit Column:** Hyperlink to commit (first 7 chars of SHA)
- **Secret URL Column:** Direct link to file + line number
- **File Column:** Hyperlink to file at commit SHA

#### Exit Code 1 (Error)
```
# ❌ Gitleaks exited with error. Exit code [1]
```

#### Other Exit Codes
```
# ❌ Gitleaks exited with unexpected exit code [<code>]
```

### Workflow Artifact Upload

**Generated By:** `src/gitleaks.js`
**Conditional:** GITLEAKS_ENABLE_UPLOAD_ARTIFACT !== false

**Artifact Details:**
- **Name:** `gitleaks-results.sarif`
- **Files:** `["results.sarif"]`
- **Root Directory:** `$HOME`
- **Options:** `{ continueOnError: true }`

**Upload Behavior:**
- Always attempts upload (even if no leaks found)
- Non-blocking (continues on upload failure)

---

## GitHub API Interactions

### API Client: Octokit (@octokit/rest v18)

**Initialization:**
```javascript
const octokit = new Octokit({
  auth: process.env.GITHUB_TOKEN,
  baseUrl: process.env.GITHUB_API_URL,
});
```

### API Calls

#### 1. Get User Information
**Endpoint:** `GET /users/{username}`
**Purpose:** Determine if repository owner is Organization or User
**Used In:** index.js:76-106
**Response Fields:**
- `user.data.type`: "Organization" | "User" | other

**Error Handling:**
- Catch errors, log warning, default to requiring license

---

#### 2. Get Latest Gitleaks Release
**Endpoint:** `GET /repos/zricethezav/gitleaks/releases/latest`
**Method:** `octokit.rest.repos.getLatestRelease({ owner, repo })`
**Purpose:** Fetch latest gitleaks version when GITLEAKS_VERSION="latest"
**Used In:** gitleaks.js:82-90
**Response Fields:**
- `latest.data.tag_name`: Version tag (e.g., "v8.24.3")

**Processing:**
- Strip 'v' prefix: `tag_name.replace(/^v/, "")`

---

#### 3. Get Pull Request Commits
**Endpoint:** `GET /repos/{owner}/{repo}/pulls/{pull_number}/commits`
**Method:** `octokit.request(...)`
**Purpose:** Fetch commit list for PR scanning
**Used In:** gitleaks.js:160-167
**Response Fields:**
- `commits.data[]`: Array of commit objects
- `commits.data[i].sha`: Commit SHA

**Usage:**
- First commit SHA → baseRef
- Last commit SHA → headRef

---

#### 4. Get Pull Request Comments
**Endpoint:** `GET /repos/{owner}/{repo}/pulls/{pull_number}/comments`
**Method:** `octokit.request(...)`
**Purpose:** Check for existing review comments (avoid duplicates)
**Used In:** gitleaks.js:233-240
**Response Fields:**
- `comments.data[]`: Array of review comments
- `comment.body`: Comment text
- `comment.path`: File path
- `comment.original_line`: Line number

**Deduplication Logic:**
```javascript
if (
  comment.body == proposedComment.body &&
  comment.path == proposedComment.path &&
  comment.original_line == proposedComment.line
) {
  skip = true;  // Comment already exists
}
```

---

#### 5. Create Pull Request Review Comment
**Endpoint:** `POST /repos/{owner}/{repo}/pulls/{pull_number}/comments`
**Method:** `octokit.rest.pulls.createReviewComment(proposedComment)`
**Purpose:** Add inline comment on PR at specific line/commit
**Used In:** gitleaks.js:263

**Comment Payload:**
```javascript
{
  owner: "<owner>",
  repo: "<repo>",
  pull_number: <number>,
  body: `🛑 **Gitleaks** has detected a secret with rule-id \`<ruleId>\` in commit <sha>.
If this secret is a _true_ positive, please rotate the secret ASAP.

If this secret is a _false_ positive, you can add the fingerprint below to your \`.gitleaksignore\` file and commit the change to this branch.

\`\`\`
echo <fingerprint> >> .gitleaksignore
\`\`\`

cc <user-list>`,
  commit_id: "<commit-sha>",
  path: "<file-path>",
  side: "RIGHT",
  line: <line-number>
}
```

**Fingerprint Format:**
```
<commitSha>:<file-path>:<ruleId>:<startLine>
```

**GITLEAKS_NOTIFY_USER_LIST:**
- Appended to comment body as: `\n\ncc ${user-list}`
- Format: "@user1,@user2,@user3"

**Error Handling:**
- Try/catch wrapper
- On error: Log warning (likely due to large diff)
- Continue processing other findings

---

## License Validation System

### Keygen Integration

**Service:** Keygen.sh (third-party license validation)
**Current Status:** DISABLED (commented out in code)
**Affected Code:** index.js:124-130

#### Validation Endpoint
**URL:** `https://api.keygen.sh/v1/accounts/{account}/licenses/actions/validate-key`
**Method:** POST
**Account ID (hex-encoded):** `64626262306364622d353538332d343662392d613563302d346337653865326634623032`

**Request Payload:**
```json
{
  "meta": {
    "key": "<GITLEAKS_LICENSE>",
    "scope": {
      "fingerprint": "<repository-full-name>"
    }
  }
}
```

**Response Constants:**
- `VALID`: License is valid for this repository
- `TOO_MANY_MACHINES`: License limit reached
- `FINGERPRINT_SCOPE_MISMATCH`: Repository not activated
- `NO_MACHINES`: No repositories associated with license
- `NO_MACHINE`: Repository not found for license

#### Activation Endpoint (Auto-activation)
**URL:** `https://api.keygen.sh/v1/accounts/{account}/machines`
**Method:** POST
**Triggered:** When validation returns NO_MACHINES/NO_MACHINE/FINGERPRINT_SCOPE_MISMATCH

**Request Payload:**
```json
{
  "data": {
    "type": "machines",
    "attributes": {
      "fingerprint": "<repository-full-name>",
      "platform": "github-actions",
      "name": "<repository-full-name>"
    },
    "relationships": {
      "license": {
        "data": {
          "type": "licenses",
          "id": "<license-id-from-validation>"
        }
      }
    }
  }
}
```

**Success Response:**
- Status: 201
- Logs: "Successfully added repo [name] to license"

#### License Requirement Logic

**Personal Accounts (type: "User"):**
- No license required
- `shouldValidate = false`

**Organization Accounts (type: "Organization"):**
- License required
- `shouldValidate = true`
- If GITLEAKS_LICENSE not set: Error and exit(1)

**Unknown Account Types:**
- Default to requiring license
- Log warning

**API Errors:**
- Log warning
- Default to requiring license (fail-safe)

---

## Error Handling & Exit Codes

### Exit Code Definitions

#### Exit Code 0: Success (No Leaks)
- Gitleaks scan completed successfully
- No secrets detected
- Action exits with 0
- Message: "✅ No leaks detected"

#### Exit Code 1: Error/Failure
**Triggered By:**
- Unsupported event type (index.js:44-45)
- Missing GITLEAKS_LICENSE for organization (index.js:110-113)
- Missing GITHUB_TOKEN for PR scanning (gitleaks.js:153-157)
- License validation failure (keygen.js:53, 113, 135)
- Unexpected exit code from gitleaks (index.js:202-203)
- Leaks detected (translated from exit code 2) (index.js:199-200)

#### Exit Code 2: Leaks Detected
- Returned by gitleaks CLI (hardcoded via --exit-code=2)
- Constant: `EXIT_CODE_LEAKS_DETECTED = 2`
- Action translates this to exit(1) for GitHub Actions failure
- Message: "🛑 Leaks detected, see job summary for details"

### Error Handling Patterns

#### Graceful Degradation
- User type API failure → warn, assume org (require license)
- Cache restore failure → warn, download binary
- Cache save failure → warn, continue without caching
- Artifact upload error → continue (continueOnError: true)
- PR comment creation error → warn, continue to next finding

#### Hard Failures (exit immediately)
- Unsupported event type
- Missing required environment variables
- License validation errors (when enabled)
- Gitleaks binary download failure

#### Logging Levels
- **core.info()**: Normal operational messages
- **core.debug()**: Verbose debugging (requires Actions debug mode)
- **core.warning()**: Non-fatal issues, degraded functionality
- **core.error()**: Fatal errors before exit

---

## Platform & Architecture Support

### Supported Platforms
- **linux** (most common for GitHub Actions)
- **darwin** (macOS runners)
- **win32** → mapped to "windows" (Windows runners)

### Supported Architectures
- **x64** (most common)
- **arm64** (Apple Silicon, ARM runners)
- **arm** (32-bit ARM)

### Binary Naming Convention
```
gitleaks_{version}_{platform}_{arch}.tar.gz
```

**Examples:**
- `gitleaks_8.24.3_linux_x64.tar.gz`
- `gitleaks_8.24.3_darwin_arm64.tar.gz`
- `gitleaks_8.24.3_windows_x64.tar.gz`

### Archive Format Support
- **tar.gz**: Primary format (all platforms)
- **zip**: Fallback for Windows (if URL ends with .zip)

### Cache Key Format
```
gitleaks-cache-{version}-{platform}-{arch}
```
- Ensures separate caches per platform/architecture
- Prevents cache misses on multi-platform workflows

---

## Critical Dependencies

### Node.js Packages (package.json)

```json
{
  "dependencies": {
    "@actions/artifact": "^2.3.2",
    "@actions/cache": "^4.0.0",
    "@actions/core": "1.10.0",
    "@actions/exec": "^1.1.1",
    "@actions/tool-cache": "^1.7.2",
    "@octokit/rest": "^18.12.0"
  },
  "devDependencies": {
    "@vercel/ncc": "^0.34.0"
  }
}
```

### Dependency Purposes

#### @actions/artifact (^2.3.2)
- **Purpose:** Upload SARIF report as workflow artifact
- **Class:** `DefaultArtifactClient`
- **Critical Method:** `uploadArtifact(name, files, rootDir, options)`

#### @actions/cache (^4.0.0)
- **Purpose:** Cache gitleaks binary across workflow runs
- **Methods:** `restoreCache()`, `saveCache()`

#### @actions/core (1.10.0)
- **Purpose:** Core GitHub Actions functionality
- **Methods:**
  - Logging: `info()`, `warning()`, `error()`, `debug()`
  - Outputs: `setOutput()`
  - Summary: `summary.addHeading()`, `summary.addTable()`, `summary.write()`
  - PATH: `addPath()`

#### @actions/exec (^1.1.1)
- **Purpose:** Execute gitleaks binary
- **Method:** `exec(command, args, options)`

#### @actions/tool-cache (^1.7.2)
- **Purpose:** Download and extract gitleaks binary
- **Methods:** `downloadTool()`, `extractTar()`, `extractZip()`

#### @octokit/rest (^18.12.0)
- **Purpose:** GitHub API interactions
- **Version:** 18.x (specific version for compatibility)
- **Methods:**
  - `request()`: Generic API requests
  - `rest.repos.getLatestRelease()`
  - `rest.pulls.createReviewComment()`

#### @vercel/ncc (^0.34.0)
- **Purpose:** Bundle JavaScript into single file (dev-time)
- **Usage:** `npx ncc build src/index.js`
- **Output:** `dist/index.js` (5MB bundled file)

### External Services

#### GitHub Releases (gitleaks binary)
- **URL:** `https://github.com/zricethezav/gitleaks/releases`
- **Dependency:** Network access to download binaries

#### Keygen.sh (license validation)
- **URL:** `https://api.keygen.sh`
- **Status:** Currently disabled in code
- **Dependency:** HTTPS access for license validation

#### GitHub API
- **URL:** `GITHUB_API_URL` (usually https://api.github.com)
- **Dependency:** GITHUB_TOKEN for authentication

---

## Security Considerations

### Secrets Handling

#### GITHUB_TOKEN
- **Scope:** Minimal required permissions
  - `pull_request: write` (for review comments)
  - `contents: read` (for repository access)
- **Auto-provided:** GitHub Actions automatically generates
- **Lifetime:** Valid only during workflow execution
- **Exposure:** Passed to Octokit (encrypted in transit)

#### GITLEAKS_LICENSE
- **Scope:** License key validation only
- **Storage:** GitHub Secrets (encrypted at rest)
- **Transmission:** Sent to Keygen.sh API (HTTPS)
- **Data Sent:** License key + repository full name

### Data Exfiltration Points

#### To Keygen.sh:
- License key (GITLEAKS_LICENSE)
- Repository full name (fingerprint)
- Platform: "github-actions"

#### To GitHub API:
- GITHUB_TOKEN (for authentication)
- PR comments (public on pull request)
- Review comment metadata (commit SHA, file path, line number)

#### To GitHub Releases:
- None (read-only binary download)

### Secrets in Output

#### SARIF Report:
- **Redacted:** Yes (gitleaks --redact flag)
- **Storage:** Workflow artifact (GitHub's artifact storage)
- **Visibility:** Same as workflow logs (repo contributors)

#### PR Comments:
- **Redacted:** Yes (no secret values shown)
- **Content:** Rule ID, commit SHA, file path, line number, fingerprint
- **Visibility:** Public (on public repos) or repo members (private repos)

#### Job Summary:
- **Redacted:** Yes
- **Content:** Same as SARIF (no secret values)
- **Visibility:** Same as workflow logs

### Potential Security Issues

#### 1. License Validation Disabled
- **Current State:** Keygen validation commented out (index.js:124-130)
- **Impact:** License keys not actually validated
- **Risk:** Organizations could run without valid license

#### 2. Secrets in Cache
- **Issue:** Gitleaks binary cached globally
- **Risk:** Low (binary is public and read-only)

#### 3. SARIF Artifact Permissions
- **Issue:** results.sarif uploaded as artifact
- **Access:** Anyone with workflow read access
- **Mitigation:** Gitleaks uses --redact flag

#### 4. BASE_REF Override
- **Issue:** Undocumented environment variable
- **Risk:** Could be misused to scan unintended commit ranges
- **Mitigation:** Requires workflow modification (not exploitable by PRs)

---

## Configuration Files

### .gitleaksignore
**Purpose:** Ignore false positives
**Format:** One fingerprint per line
**Fingerprint Format:** `<commit>:<file>:<rule>:<line>`
**Location:** Repository root
**Auto-detected:** By gitleaks CLI (not passed explicitly)

**Example:**
```
030c71fd5ce438619fbc3a56663b58baed00afeb:src/gitleaks.js:discord-client-secret:103
7ea7f374bd15952df1176c1bdf6ab3568e620bd1:src/keygen.js:generic-api-key:10
```

### gitleaks.toml
**Purpose:** Custom gitleaks detection rules
**Format:** TOML configuration
**Location:** Repository root (auto-detected) OR path in GITLEAKS_CONFIG
**Note:** GITLEAKS_CONFIG mentioned in README but not implemented in code
**Detection:** Handled by gitleaks CLI, not the action

---

## Recommended Action.yml for Rust Port

```yaml
name: Gitleaks
author: <your-author>
branding:
  icon: "git-pull-request"
  color: "purple"
description: run gitleaks on push and pull-request events

inputs:
  github-token:
    description: 'GitHub token for API access'
    required: false
  license-key:
    description: 'Gitleaks license key (required for organizations)'
    required: false
  version:
    description: 'Gitleaks version to use (default: 8.24.3, or "latest")'
    required: false
    default: '8.24.3'
  config-path:
    description: 'Path to gitleaks configuration file'
    required: false
  enable-summary:
    description: 'Enable GitHub Actions job summary'
    required: false
    default: 'true'
  enable-upload-artifact:
    description: 'Enable SARIF artifact upload'
    required: false
    default: 'true'
  enable-comments:
    description: 'Enable PR comments'
    required: false
    default: 'true'
  notify-users:
    description: 'Comma-separated list of GitHub users to notify (@user1,@user2)'
    required: false
  base-ref:
    description: 'Override base git reference for scanning'
    required: false

outputs:
  exit-code:
    description: 'Exit code from gitleaks scan (0=no leaks, 1=error, 2=leaks detected)'

runs:
  using: 'composite'
  main: 'dist/secretscout'  # or whatever your Rust binary is named
```

**Note:** Current action.yml has NO inputs/outputs defined. This is a recommended improvement for the Rust port to make configuration more explicit and discoverable.

---

## File Structure Summary

```
gitleaks-action/
├── action.yml                 # Action metadata (minimal)
├── package.json               # Node.js dependencies
├── package-lock.json          # Dependency lock file
├── .nvmrc                     # Node version (20.x)
├── .gitleaksignore            # False positive suppressions
├── README.md                  # User documentation
├── v2.md                      # v2 migration guide
├── CONTRIBUTING.md            # Contribution guidelines
├── LICENSE.txt                # Commercial license
├── src/
│   ├── index.js              # Main entry point (205 lines)
│   ├── gitleaks.js           # Binary management, scanning logic (282 lines)
│   ├── keygen.js             # License validation (163 lines)
│   └── summary.js            # GitHub Actions summary generation (56 lines)
├── dist/
│   └── index.js              # Bundled output (5MB, single file)
└── .github/
    └── workflows/
        ├── example.yml        # Usage example
        └── gitleaks-action-HEAD.yml  # Self-test workflow
```

**Total Source Code:** 706 lines (excluding dist/)

---

## Key Behavioral Notes for Rust Implementation

### 1. Event Payload Handling
- Read JSON from `GITHUB_EVENT_PATH` file
- Parse and extract event-specific fields
- Handle missing fields gracefully (especially for schedule events)

### 2. Schedule Event Special Case
- `eventJSON.repository` is undefined for schedule events
- Must manually construct from `GITHUB_REPOSITORY_OWNER` and `GITHUB_REPOSITORY`

### 3. Commit Range Logic
- Push events: Use first commit as base, last commit as head
- PR events: Fetch commits via API first, then use same logic
- Special case: If baseRef == headRef, use `--log-opts=-1`

### 4. Caching Strategy
- Cache key must include version + platform + arch
- Try restore first, download only if cache miss
- Save cache after download (ignore save errors)

### 5. Binary PATH Addition
- Must add binary directory to PATH (not just execute absolute path)
- This allows gitleaks to find its own resources

### 6. SARIF Parsing
- Expects specific SARIF structure from gitleaks
- Must handle `partialFingerprints` extension (not standard SARIF)
- Array access: Always use `[0]` for locations (gitleaks emits single location per result)

### 7. Comment Deduplication
- Must check existing comments before posting
- Compare: body + path + original_line
- Performance note: Could be optimized with HashMap (current: O(n*m))

### 8. Error Handling Philosophy
- Warn and continue for non-critical errors (cache, artifacts, comments)
- Fail fast for critical errors (missing token, unsupported events)
- Always exit(1) if leaks detected (for GitHub Actions failure)

### 9. Output Redaction
- Gitleaks CLI handles redaction (--redact flag)
- Action never displays actual secret values
- Trust gitleaks' redaction implementation

### 10. License Validation
- Currently disabled but code exists
- If re-enabled: Must handle async HTTP to Keygen.sh
- Auto-activation on first use (NO_MACHINES response)

---

## Testing Recommendations for Rust Port

### Unit Tests
1. Environment variable parsing (boolean coercion)
2. Platform/arch mapping for download URLs
3. SARIF parsing and field extraction
4. Fingerprint generation
5. Comment deduplication logic
6. Event type validation

### Integration Tests
1. Binary download and extraction
2. Gitleaks execution with various args
3. SARIF file reading and parsing
4. GitHub API interactions (mocked)
5. Cache save/restore (mocked)

### End-to-End Tests
1. Push event workflow
2. Pull request event workflow
3. Workflow dispatch event
4. Schedule event
5. License validation flow (if re-enabled)

### Edge Cases
1. Empty commit list (push event)
2. Single commit scan (baseRef == headRef)
3. Large SARIF output (many findings)
4. PR comment API failures (large diffs)
5. Cache corruption/unavailability
6. Network failures (binary download, API calls)
7. Unsupported platform/architecture

---

## Rust Implementation Considerations

### Async Runtime
- Recommended: **tokio** (compatible with GitHub Actions environment)
- Alternatives: async-std, smol

### HTTP Client
- Recommended: **reqwest** (for GitHub API, binary downloads, Keygen)
- Features needed: JSON, async, TLS

### GitHub API
- Recommended: **octocrab** (idiomatic Rust GitHub API client)
- Alternatives: Manual reqwest calls with serde

### Command Execution
- Recommended: **tokio::process::Command** (async process spawning)
- Alternatives: std::process::Command (blocking)

### JSON Parsing
- Recommended: **serde_json** (ubiquitous Rust JSON library)
- Needed for: Event payload, SARIF, API responses

### Archive Extraction
- **tar** crate (for .tar.gz)
- **zip** crate (for Windows .zip)

### File System
- **tokio::fs** (async file I/O)
- **std::fs** (sync I/O for smaller files)

### Environment Variables
- **std::env** (native Rust)

### Caching
- **GitHub Actions Cache API** (requires HTTP client)
- Custom implementation (no direct Rust SDK)

### Artifact Upload
- **GitHub Actions Artifact API** (requires HTTP client)
- Custom implementation (no direct Rust SDK)

### Error Handling
- **anyhow** or **thiserror** (error management)
- Custom error types for different failure modes

### Platform Detection
- **std::env::consts::OS** (platform)
- **std::env::consts::ARCH** (architecture)

### Logging
- **log** + **env_logger** (compatible with Actions core logging format)
- Custom formatters for `::info::`, `::warning::`, `::error::` annotations

---

## GitHub Actions Annotations Format

### Log Commands (for core.info, core.error, etc.)
```
::info::Message here
::warning::Message here
::error::Message here
::debug::Message here
```

### Set Output
```
::set-output name=exit-code::0
```

### Add to PATH
```
{path}
```
Append to `GITHUB_PATH` file

### Job Summary
Write markdown to file specified in `GITHUB_STEP_SUMMARY` environment variable

---

## Performance Characteristics

### Typical Execution Times (from v2.md)
- **v1 (Docker-based):** 60-90 seconds (includes Docker build)
- **v2 (Node-based):** 10-30 seconds (no Docker overhead)
- **Rust Port Target:** 5-20 seconds (faster startup, parallel operations)

### Bottlenecks
1. **Binary Download:** ~5-10 seconds (first run)
   - Mitigated by caching
2. **Gitleaks Scan:** ~5-30 seconds (depends on repo size)
   - Controlled by commit range
3. **GitHub API Calls:** ~1-5 seconds total
   - User lookup, PR commits, existing comments
4. **Artifact Upload:** ~2-5 seconds
   - Depends on SARIF size

### Optimization Opportunities
1. **Parallel API Calls:** Fetch user info + latest release simultaneously
2. **Concurrent Comment Checking:** Build HashMap of existing comments once
3. **Streaming SARIF Parsing:** Don't load entire file into memory
4. **Early Exit:** Stop processing on first critical error

---

## Appendix: Complete Environment Variable Reference

| Variable | Type | Required | Default | Source |
|----------|------|----------|---------|--------|
| GITHUB_TOKEN | Secret | Yes (PR) | - | User-provided |
| GITLEAKS_LICENSE | Secret | Conditional* | - | User-provided |
| GITLEAKS_VERSION | String | No | "8.24.3" | User-provided |
| GITLEAKS_CONFIG | String | No | - | User-provided |
| GITLEAKS_ENABLE_SUMMARY | Boolean | No | true | User-provided |
| GITLEAKS_ENABLE_UPLOAD_ARTIFACT | Boolean | No | true | User-provided |
| GITLEAKS_ENABLE_COMMENTS | Boolean | No | true | User-provided |
| GITLEAKS_NOTIFY_USER_LIST | String | No | - | User-provided |
| BASE_REF | String | No | - | User-provided |
| GITHUB_EVENT_PATH | String | Yes | - | GitHub Actions |
| GITHUB_EVENT_NAME | String | Yes | - | GitHub Actions |
| GITHUB_REPOSITORY_OWNER | String | Yes | - | GitHub Actions |
| GITHUB_REPOSITORY | String | Yes | - | GitHub Actions |
| GITHUB_API_URL | String | Yes | - | GitHub Actions |
| GITHUB_STEP_SUMMARY | String | Yes | - | GitHub Actions |
| GITHUB_PATH | String | Yes | - | GitHub Actions |
| HOME | String | Yes | - | System |

\* Required for organization repositories only

---

## Appendix: Exit Code Translation Matrix

| Gitleaks Exit | Action Exit | GitHub Status | Message |
|---------------|-------------|---------------|---------|
| 0 | 0 | ✅ Success | "✅ No leaks detected" |
| 1 | 1 | ❌ Failure | "❌ Gitleaks exited with error" |
| 2 | 1 | ❌ Failure | "🛑 Leaks detected, see job summary" |
| Other | Other | ❌ Failure | "ERROR: Unexpected exit code [X]" |

---

## Appendix: SARIF Example (from gitleaks)

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
          "rules": [
            {
              "id": "aws-access-token",
              "name": "AWS Access Token",
              "shortDescription": {
                "text": "Detected AWS Access Token"
              }
            }
          ]
        }
      },
      "results": [
        {
          "ruleId": "aws-access-token",
          "ruleIndex": 0,
          "level": "error",
          "message": {
            "text": "AWS Access Token detected"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "src/config.js"
                },
                "region": {
                  "startLine": 42,
                  "startColumn": 15,
                  "endLine": 42,
                  "endColumn": 55
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "abc123def456",
            "author": "John Doe",
            "email": "john@example.com",
            "date": "2025-10-15T12:00:00Z"
          }
        }
      ]
    }
  ]
}
```

---

## Document Revision History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-10-15 | Initial comprehensive analysis |

---

**END OF TECHNICAL SPECIFICATION**
