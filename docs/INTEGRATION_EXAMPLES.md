# Gitleaks Integration Examples and Test Cases
## Practical Reference for Rust Implementation

**Companion Documents**:
- GITLEAKS_INTEGRATION_SPEC.md (detailed specifications)
- GITLEAKS_INTEGRATION_FLOWS.md (visual flow diagrams)

**Date**: 2025-10-15

---

## TABLE OF CONTENTS

1. [Command-Line Invocation Examples](#1-command-line-invocation-examples)
2. [SARIF File Examples](#2-sarif-file-examples)
3. [GitHub Event JSON Examples](#3-github-event-json-examples)
4. [Configuration File Examples](#4-configuration-file-examples)
5. [PR Comment Examples](#5-pr-comment-examples)
6. [Summary Output Examples](#6-summary-output-examples)
7. [Environment Variable Examples](#7-environment-variable-examples)
8. [Test Cases](#8-test-cases)
9. [Edge Cases and Error Scenarios](#9-edge-cases-and-error-scenarios)
10. [API Request/Response Examples](#10-api-requestresponse-examples)

---

## 1. COMMAND-LINE INVOCATION EXAMPLES

### 1.1 Push Event - Multiple Commits

```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug \
  --log-opts='--no-merges --first-parent abc123def456^..789ghijkl012'
```

**Context**: Push event with commits from `abc123def456` to `789ghijkl012`

**Expected Behavior**: Scan only the specified commit range

### 1.2 Push Event - Single Commit

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

**Context**: Push event where baseRef equals headRef

**Expected Behavior**: Scan only the most recent commit

### 1.3 Pull Request Event

```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug \
  --log-opts='--no-merges --first-parent main^..feature-branch'
```

**Context**: PR from feature-branch into main

**Expected Behavior**: Scan commits unique to the PR

### 1.4 Full Repository Scan (Scheduled/Dispatch)

```bash
gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug
```

**Context**: Scheduled or manual workflow dispatch

**Expected Behavior**: Scan entire git history

### 1.5 With Custom Configuration

```bash
export GITLEAKS_CONFIG=/path/to/custom.toml

gitleaks detect \
  --redact \
  -v \
  --exit-code=2 \
  --report-format=sarif \
  --report-path=results.sarif \
  --log-level=debug
```

**Context**: Using custom configuration file

**Expected Behavior**: Gitleaks reads config from specified path

---

## 2. SARIF FILE EXAMPLES

### 2.1 Complete SARIF with Multiple Findings

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
          "informationUri": "https://github.com/gitleaks/gitleaks",
          "rules": [
            {
              "id": "aws-access-token",
              "shortDescription": {
                "text": "Identified an AWS Access Token"
              }
            },
            {
              "id": "generic-api-key",
              "shortDescription": {
                "text": "Identified a Generic API Key"
              }
            }
          ]
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
                  "uri": "src/config/aws.js"
                },
                "region": {
                  "startLine": 12,
                  "startColumn": 18,
                  "endLine": 12,
                  "endColumn": 58,
                  "snippet": {
                    "text": "const accessKey = 'AKIAIOSFODNN7EXAMPLE';"
                  }
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "abc123def4567890abcdef1234567890abcdef12",
            "author": "John Doe",
            "email": "john.doe@example.com",
            "date": "2025-10-15T14:30:00Z",
            "commitMessage": "Add AWS configuration"
          }
        },
        {
          "ruleId": "generic-api-key",
          "message": {
            "text": "Identified a Generic API Key"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "src/services/api.js"
                },
                "region": {
                  "startLine": 45,
                  "startColumn": 15,
                  "endLine": 45,
                  "endColumn": 79,
                  "snippet": {
                    "text": "const apiKey = 'sk_live_REDACTED_EXAMPLE_KEY';"
                  }
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "def456abc7890123def4567890abcdef1234567",
            "author": "Jane Smith",
            "email": "jane.smith@example.com",
            "date": "2025-10-15T15:45:00Z",
            "commitMessage": "Integrate payment API"
          }
        }
      ]
    }
  ]
}
```

**Usage**: Test multi-result parsing, table generation, comment creation

### 2.2 Empty SARIF (No Leaks)

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
          "informationUri": "https://github.com/gitleaks/gitleaks",
          "rules": []
        }
      },
      "results": []
    }
  ]
}
```

**Usage**: Test exit code 0 handling, success summary generation

### 2.3 Minimal SARIF (Single Finding)

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
          "ruleId": "github-pat",
          "message": {
            "text": "Identified a GitHub Personal Access Token"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": ".env"
                },
                "region": {
                  "startLine": 5,
                  "startColumn": 1,
                  "endLine": 5,
                  "endColumn": 45,
                  "snippet": {
                    "text": "GITHUB_TOKEN=ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxx"
                  }
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
            "author": "Developer",
            "email": "dev@company.com",
            "date": "2025-10-14T10:20:30Z",
            "commitMessage": "Initial commit"
          }
        }
      ]
    }
  ]
}
```

**Usage**: Test basic parsing, single comment creation, basic summary

---

## 3. GITHUB EVENT JSON EXAMPLES

### 3.1 Push Event

```json
{
  "ref": "refs/heads/main",
  "before": "abc123def4567890abcdef1234567890abcdef12",
  "after": "def456abc7890123def4567890abcdef1234567",
  "repository": {
    "id": 123456789,
    "name": "my-repo",
    "full_name": "octocat/my-repo",
    "owner": {
      "login": "octocat",
      "type": "User"
    },
    "html_url": "https://github.com/octocat/my-repo"
  },
  "commits": [
    {
      "id": "abc123def4567890abcdef1234567890abcdef12",
      "message": "Add configuration",
      "timestamp": "2025-10-15T14:30:00Z",
      "author": {
        "name": "John Doe",
        "email": "john.doe@example.com"
      }
    },
    {
      "id": "def456abc7890123def4567890abcdef1234567",
      "message": "Update API integration",
      "timestamp": "2025-10-15T15:45:00Z",
      "author": {
        "name": "Jane Smith",
        "email": "jane.smith@example.com"
      }
    }
  ],
  "pusher": {
    "name": "octocat",
    "email": "octocat@github.com"
  }
}
```

**Usage**: Test push event handling, commit range extraction

### 3.2 Pull Request Event

```json
{
  "action": "opened",
  "number": 42,
  "pull_request": {
    "id": 987654321,
    "number": 42,
    "state": "open",
    "title": "Add new feature",
    "user": {
      "login": "contributor",
      "type": "User"
    },
    "head": {
      "ref": "feature/new-feature",
      "sha": "feature123abc456def789ghi012jkl345mno678pqr"
    },
    "base": {
      "ref": "main",
      "sha": "main456def789abc012ghi345jkl678mno901pqr234"
    }
  },
  "repository": {
    "id": 123456789,
    "name": "my-repo",
    "full_name": "octocat/my-repo",
    "owner": {
      "login": "octocat",
      "type": "Organization"
    },
    "html_url": "https://github.com/octocat/my-repo"
  }
}
```

**Usage**: Test PR event handling, commit fetching, comment creation

### 3.3 Schedule Event

**Note**: Schedule events have undefined `repository` field initially

```json
{
  "schedule": "0 4 * * *",
  "repository": null
}
```

**After Processing** (action reconstructs):
```json
{
  "schedule": "0 4 * * *",
  "repository": {
    "owner": {
      "login": "octocat"
    },
    "full_name": "octocat/my-repo"
  }
}
```

**Environment Variables Required**:
- `GITHUB_REPOSITORY_OWNER=octocat`
- `GITHUB_REPOSITORY=octocat/my-repo`

**Usage**: Test schedule event handling, repository reconstruction

### 3.4 Workflow Dispatch Event

```json
{
  "inputs": {},
  "ref": "refs/heads/main",
  "workflow": ".github/workflows/gitleaks.yml",
  "repository": {
    "id": 123456789,
    "name": "my-repo",
    "full_name": "octocat/my-repo",
    "owner": {
      "login": "octocat",
      "type": "User"
    },
    "html_url": "https://github.com/octocat/my-repo"
  },
  "sender": {
    "login": "octocat",
    "type": "User"
  }
}
```

**Usage**: Test manual dispatch handling, full repo scan

---

## 4. CONFIGURATION FILE EXAMPLES

### 4.1 Basic .gitleaks.toml

```toml
title = "Gitleaks Config"

[[rules]]
id = "github-pat"
description = "GitHub Personal Access Token"
regex = '''ghp_[0-9a-zA-Z]{36}'''

[[rules]]
id = "aws-access-key"
description = "AWS Access Key ID"
regex = '''AKIA[0-9A-Z]{16}'''

[[rules]]
id = "generic-api-key"
description = "Generic API Key"
regex = '''(?i)(api[_-]?key|apikey)['\"\s]*[:=]['\"\s]*[0-9a-zA-Z\-_]{20,}'''
```

**Usage**: Test basic configuration loading

### 4.2 Extended Configuration (Extending Defaults)

```toml
title = "Custom Gitleaks Config"

[extend]
useDefault = true

# Disable noisy rules
disabledRules = [
  "generic-api-key",
  "private-key"
]

# Add custom rule
[[rules]]
id = "company-secret"
description = "Company Internal Secret"
regex = '''COMPANY_SECRET_[0-9a-zA-Z]{32}'''
```

**Usage**: Test configuration extension, rule disabling

### 4.3 Configuration with Allowlist

```toml
title = "Gitleaks Config with Allowlist"

[extend]
useDefault = true

[allowlist]
description = "Allowlist for false positives"

# Ignore specific file paths
paths = [
  '''.gitleaksignore''',
  '''.*_test\.go''',
  '''testdata/.*'''
]

# Ignore specific patterns
regexes = [
  '''EXAMPLE_KEY''',
  '''test@example\.com'''
]

# Ignore specific commits
commits = [
  "abc123def456",
  "789ghi012jkl"
]

# Stopwords that indicate false positives
stopwords = [
  "example",
  "test",
  "sample"
]
```

**Usage**: Test allowlist functionality, path exclusions

### 4.4 .gitleaksignore File

```
# Format: {commitSha}:{file}:{ruleID}:{startLine}

# False positive in test file
abc123def4567890abcdef1234567890abcdef12:tests/fixtures/sample.js:generic-api-key:15

# Example API key in documentation
def456abc7890123def4567890abcdef1234567:docs/api-guide.md:aws-access-token:42

# Legacy test credential
1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t:src/test/legacy.py:github-pat:7
```

**Usage**: Test fingerprint-based ignoring

---

## 5. PR COMMENT EXAMPLES

### 5.1 Standard Secret Detection Comment

```markdown
🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123def4567890abcdef1234567890abcdef12.
If this secret is a _true_ positive, please rotate the secret ASAP.

If this secret is a _false_ positive, you can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch.

```
echo abc123def4567890abcdef1234567890abcdef12:src/config/aws.js:aws-access-token:12 >> .gitleaksignore
```
```

**Posted on**:
- Commit: `abc123def4567890abcdef1234567890abcdef12`
- File: `src/config/aws.js`
- Line: `12`
- Side: `RIGHT`

### 5.2 Comment with User Notifications

```markdown
🛑 **Gitleaks** has detected a secret with rule-id `github-pat` in commit 1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t.
If this secret is a _true_ positive, please rotate the secret ASAP.

If this secret is a _false_ positive, you can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch.

```
echo 1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t:.env:github-pat:5 >> .gitleaksignore
```

cc @security-team,@tech-lead
```

**Environment Variable**: `GITLEAKS_NOTIFY_USER_LIST=@security-team,@tech-lead`

### 5.3 Comment API Request

```json
{
  "owner": "octocat",
  "repo": "my-repo",
  "pull_number": 42,
  "body": "🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123...",
  "commit_id": "abc123def4567890abcdef1234567890abcdef12",
  "path": "src/config/aws.js",
  "side": "RIGHT",
  "line": 12
}
```

**API Endpoint**: `POST /repos/octocat/my-repo/pulls/42/comments`

---

## 6. SUMMARY OUTPUT EXAMPLES

### 6.1 Success Summary (No Leaks)

**Markdown**:
```markdown
# No leaks detected ✅
```

**Rendered**:
# No leaks detected ✅

### 6.2 Leaks Detected Summary

**Markdown**:
```markdown
# 🛑 Gitleaks detected secrets 🛑

| Rule ID | Commit | Secret URL | Start Line | Author | Date | Email | File |
|---------|--------|------------|------------|--------|------|-------|------|
| aws-access-token | <a href="https://github.com/octocat/my-repo/commit/abc123def4567890abcdef1234567890abcdef12">abc123d</a> | <a href="https://github.com/octocat/my-repo/blob/abc123def4567890abcdef1234567890abcdef12/src/config/aws.js#L12">View Secret</a> | 12 | John Doe | 2025-10-15T14:30:00Z | john.doe@example.com | <a href="https://github.com/octocat/my-repo/blob/abc123def4567890abcdef1234567890abcdef12/src/config/aws.js">src/config/aws.js</a> |
| generic-api-key | <a href="https://github.com/octocat/my-repo/commit/def456abc7890123def4567890abcdef1234567">def456a</a> | <a href="https://github.com/octocat/my-repo/blob/def456abc7890123def4567890abcdef1234567/src/services/api.js#L45">View Secret</a> | 45 | Jane Smith | 2025-10-15T15:45:00Z | jane.smith@example.com | <a href="https://github.com/octocat/my-repo/blob/def456abc7890123def4567890abcdef1234567/src/services/api.js">src/services/api.js</a> |
```

**Rendered**:
# 🛑 Gitleaks detected secrets 🛑

| Rule ID | Commit | Secret URL | Start Line | Author | Date | Email | File |
|---------|--------|------------|------------|--------|------|-------|------|
| aws-access-token | [abc123d](https://github.com/octocat/my-repo/commit/abc123def4567890abcdef1234567890abcdef12) | [View Secret](https://github.com/octocat/my-repo/blob/abc123def4567890abcdef1234567890abcdef12/src/config/aws.js#L12) | 12 | John Doe | 2025-10-15T14:30:00Z | john.doe@example.com | [src/config/aws.js](https://github.com/octocat/my-repo/blob/abc123def4567890abcdef1234567890abcdef12/src/config/aws.js) |
| generic-api-key | [def456a](https://github.com/octocat/my-repo/commit/def456abc7890123def4567890abcdef1234567) | [View Secret](https://github.com/octocat/my-repo/blob/def456abc7890123def4567890abcdef1234567/src/services/api.js#L45) | 45 | Jane Smith | 2025-10-15T15:45:00Z | jane.smith@example.com | [src/services/api.js](https://github.com/octocat/my-repo/blob/def456abc7890123def4567890abcdef1234567/src/services/api.js) |

### 6.3 Error Summary

**Markdown**:
```markdown
# ❌ Gitleaks exited with error. Exit code [1]
```

**Rendered**:
# ❌ Gitleaks exited with error. Exit code [1]

### 6.4 Unexpected Exit Code Summary

**Markdown**:
```markdown
# ❌ Gitleaks exited with unexpected exit code [127]
```

**Rendered**:
# ❌ Gitleaks exited with unexpected exit code [127]

---

## 7. ENVIRONMENT VARIABLE EXAMPLES

### 7.1 Minimal Configuration (Personal Account, Push Event)

```bash
# Auto-generated by GitHub Actions
export GITHUB_TOKEN="ghs_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export GITHUB_EVENT_PATH="/home/runner/work/_temp/_github_workflow/event.json"
export GITHUB_EVENT_NAME="push"
export GITHUB_REPOSITORY="johndoe/my-project"
export GITHUB_REPOSITORY_OWNER="johndoe"
export GITHUB_API_URL="https://api.github.com"
```

**Expected Behavior**:
- No license required (personal account)
- Use default gitleaks version (8.24.3)
- All features enabled (comments, summary, artifacts)

### 7.2 Full Configuration (Organization, PR Event)

```bash
# Auto-generated
export GITHUB_TOKEN="ghs_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export GITHUB_EVENT_PATH="/home/runner/work/_temp/_github_workflow/event.json"
export GITHUB_EVENT_NAME="pull_request"
export GITHUB_REPOSITORY="acme-corp/web-app"
export GITHUB_REPOSITORY_OWNER="acme-corp"
export GITHUB_API_URL="https://api.github.com"

# User-configured
export GITLEAKS_LICENSE="your-license-key-here"
export GITLEAKS_VERSION="8.24.3"
export GITLEAKS_CONFIG=".gitleaks-custom.toml"
export GITLEAKS_NOTIFY_USER_LIST="@security-team,@tech-lead"
export GITLEAKS_ENABLE_COMMENTS="true"
export GITLEAKS_ENABLE_SUMMARY="true"
export GITLEAKS_ENABLE_UPLOAD_ARTIFACT="true"
```

**Expected Behavior**:
- License validation required (organization)
- Use specified gitleaks version
- Use custom configuration file
- Notify specific users in comments
- All features enabled

### 7.3 Minimal Output Configuration

```bash
# Auto-generated
export GITHUB_TOKEN="ghs_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export GITHUB_EVENT_PATH="/home/runner/work/_temp/_github_workflow/event.json"
export GITHUB_EVENT_NAME="push"
export GITHUB_REPOSITORY="johndoe/my-project"
export GITHUB_REPOSITORY_OWNER="johndoe"
export GITHUB_API_URL="https://api.github.com"

# Disable optional features
export GITLEAKS_ENABLE_COMMENTS="false"
export GITLEAKS_ENABLE_SUMMARY="false"
export GITLEAKS_ENABLE_UPLOAD_ARTIFACT="false"
```

**Expected Behavior**:
- No PR comments
- No job summary
- No artifact upload
- Only exit code matters

### 7.4 GitHub Enterprise Configuration

```bash
# GitHub Enterprise URLs
export GITHUB_TOKEN="ghs_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export GITHUB_API_URL="https://github.enterprise.com/api/v3"
export GITHUB_SERVER_URL="https://github.enterprise.com"

# Standard vars
export GITHUB_EVENT_PATH="/home/runner/work/_temp/_github_workflow/event.json"
export GITHUB_EVENT_NAME="pull_request"
export GITHUB_REPOSITORY="myorg/myrepo"
export GITHUB_REPOSITORY_OWNER="myorg"
```

**Expected Behavior**: Use enterprise GitHub instance for API calls

### 7.5 Custom Base Reference Override

```bash
# Auto-generated
export GITHUB_TOKEN="ghs_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
export GITHUB_EVENT_PATH="/home/runner/work/_temp/_github_workflow/event.json"
export GITHUB_EVENT_NAME="push"
export GITHUB_REPOSITORY="johndoe/my-project"
export GITHUB_REPOSITORY_OWNER="johndoe"
export GITHUB_API_URL="https://api.github.com"

# Override base reference
export BASE_REF="abc123def456"
```

**Expected Behavior**: Use `abc123def456` as baseRef instead of auto-detected value

---

## 8. TEST CASES

### 8.1 Unit Tests

#### Test: Parse SARIF with Single Finding

**Input**: `sarif_single_finding.json` (see section 2.3)

**Expected Output**:
```rust
SarifReport {
    runs: vec![
        SarifRun {
            results: vec![
                SarifResult {
                    rule_id: "github-pat",
                    message: SarifMessage {
                        text: "Identified a GitHub Personal Access Token"
                    },
                    locations: vec![...],
                    partial_fingerprints: PartialFingerprints {
                        commit_sha: "1a2b3c4d5e6f7g8h9i0j1k2l3m4n5o6p7q8r9s0t",
                        author: "Developer",
                        email: "dev@company.com",
                        date: "2025-10-14T10:20:30Z",
                        commit_message: Some("Initial commit")
                    }
                }
            ]
        }
    ]
}
```

#### Test: Generate Fingerprint

**Input**:
```rust
let commit_sha = "abc123def4567890abcdef1234567890abcdef12";
let file_path = "src/config/aws.js";
let rule_id = "aws-access-token";
let start_line = 12;
```

**Expected Output**:
```
"abc123def4567890abcdef1234567890abcdef12:src/config/aws.js:aws-access-token:12"
```

#### Test: Build Comment Body

**Input**:
```rust
let rule_id = "aws-access-token";
let commit_sha = "abc123def456";
let fingerprint = "abc123def456:src/file.js:aws-access-token:10";
```

**Expected Output**:
```markdown
🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123def456.
If this secret is a _true_ positive, please rotate the secret ASAP.

If this secret is a _false_ positive, you can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch.

```
echo abc123def456:src/file.js:aws-access-token:10 >> .gitleaksignore
```
```

#### Test: Parse Boolean Environment Variable

**Input/Output**:
| Input Value | Expected Output |
|-------------|----------------|
| `"false"` | `false` |
| `"0"` | `false` |
| `"true"` | `true` |
| `"1"` | `true` |
| `"yes"` | `true` |
| (not set) | (default value) |

#### Test: Extract Commit Range from Push Event

**Input**: Push event JSON (see section 3.1)

**Expected Output**:
```rust
CommitRange {
    base_ref: "abc123def4567890abcdef1234567890abcdef12",
    head_ref: "def456abc7890123def4567890abcdef1234567"
}
```

#### Test: Build Gitleaks Arguments for Push Event

**Input**:
```rust
let event_type = EventType::Push;
let base_ref = "abc123";
let head_ref = "def456";
```

**Expected Output**:
```rust
vec![
    "detect",
    "--redact",
    "-v",
    "--exit-code=2",
    "--report-format=sarif",
    "--report-path=results.sarif",
    "--log-level=debug",
    "--log-opts=--no-merges --first-parent abc123^..def456"
]
```

#### Test: Build Gitleaks Arguments for Full Scan

**Input**:
```rust
let event_type = EventType::Schedule;
```

**Expected Output**:
```rust
vec![
    "detect",
    "--redact",
    "-v",
    "--exit-code=2",
    "--report-format=sarif",
    "--report-path=results.sarif",
    "--log-level=debug"
]
// No --log-opts
```

### 8.2 Integration Tests

#### Test: End-to-End Push Event with Leaks

**Setup**:
1. Create test repository with secrets
2. Mock GitHub event JSON (push event)
3. Execute action

**Expected Behavior**:
1. Gitleaks executes successfully
2. Exit code = 2
3. SARIF file created with findings
4. Summary generated with table
5. Artifact uploaded
6. Action fails with exit code 1

**Assertions**:
- SARIF file exists and is valid JSON
- Summary contains correct number of findings
- Artifact uploaded successfully
- Final exit code = 1

#### Test: End-to-End PR Event with Comments

**Setup**:
1. Create test repository with secrets in PR
2. Mock GitHub event JSON (PR event)
3. Mock GitHub API responses
4. Execute action

**Expected Behavior**:
1. Fetch PR commits via API
2. Execute gitleaks with commit range
3. Parse SARIF results
4. Create PR review comments
5. Generate summary
6. Upload artifact
7. Fail workflow

**Assertions**:
- API calls made correctly
- Comments created (verify via mock)
- Comment deduplication works
- Summary generated
- Artifact uploaded
- Final exit code = 1

#### Test: Schedule Event with No Leaks

**Setup**:
1. Create clean test repository
2. Mock GitHub event JSON (schedule event)
3. Execute action

**Expected Behavior**:
1. Full repository scan
2. Exit code = 0
3. Success summary generated
4. No artifact created (no findings)
5. Action succeeds

**Assertions**:
- Gitleaks scanned entire repo
- Exit code = 0
- Summary shows success message
- No SARIF file or empty SARIF
- Final exit code = 0

#### Test: Configuration File Discovery

**Setup**:
1. Create `.gitleaks.toml` in repository root
2. Add custom rule to config
3. Create test file matching custom rule
4. Execute action

**Expected Behavior**:
1. Gitleaks loads custom config
2. Custom rule triggers
3. Finding appears in SARIF

**Assertions**:
- SARIF contains finding from custom rule
- Config precedence respected

### 8.3 Performance Tests

#### Test: Large Repository Scan

**Setup**:
- Repository with 10,000+ commits
- Schedule event (full scan)

**Expected Behavior**:
- Scan completes within timeout (60s)
- Memory usage reasonable

**Metrics**:
- Execution time
- Memory consumption
- CPU usage

#### Test: Many Findings

**Setup**:
- Repository with 100+ secrets
- Push event

**Expected Behavior**:
- All findings parsed correctly
- Summary table generated
- All comments created (deduplicated)

**Metrics**:
- SARIF parsing time
- Comment creation time
- Total execution time

---

## 9. EDGE CASES AND ERROR SCENARIOS

### 9.1 Empty Commit List (Push Event)

**Input**:
```json
{
  "repository": {...},
  "commits": []
}
```

**Expected Behavior**:
- Log: "No commits to scan"
- Exit code: 0
- No gitleaks execution

### 9.2 SARIF File Not Found

**Scenario**: Gitleaks exits with code 2 but no SARIF file

**Expected Behavior**:
- Log error: "SARIF file not found"
- Skip comment creation
- Skip summary generation
- Exit code: 1

### 9.3 Malformed SARIF JSON

**Input**: Invalid JSON in `results.sarif`

**Expected Behavior**:
- JSON parse error logged
- Skip processing
- Exit code: 1

### 9.4 GitHub API Rate Limit

**Scenario**: Comment creation fails with 429 (rate limit)

**Expected Behavior**:
- Log warning with error
- Continue with other findings
- Summary still generated
- Exit code: 1 (from gitleaks exit code 2)

### 9.5 Comment on Outdated Commit

**Scenario**: PR comment fails because commit is no longer at tip

**Expected Behavior**:
- Log warning
- Continue with other comments
- Don't fail the action
- Summary still generated

### 9.6 Large Diff Comment Failure

**Scenario**: Comment creation fails due to large PR diff

**Expected Behavior**:
- Log warning: "Likely an issue with too large of a diff"
- Continue with other comments
- All secrets reported in summary and artifact

### 9.7 Duplicate Comment Detection

**Setup**:
- Existing PR comment for same secret
- Re-run action

**Expected Behavior**:
- Fetch existing comments
- Detect duplicate (same body, path, line)
- Skip comment creation
- Log: "Comment already exists, skipping"

### 9.8 Base Ref Equals Head Ref

**Input**:
```json
{
  "commits": [
    {"id": "abc123..."}
  ]
}
```

**Expected Behavior**:
- Detect base == head
- Use `--log-opts=-1`
- Scan only single commit

### 9.9 Unsupported Event Type

**Input**:
```json
{
  "repository": {...}
}
```

**Environment**: `GITHUB_EVENT_NAME=release`

**Expected Behavior**:
- Log error: "The [release] event is not yet supported"
- Exit code: 1
- No scan executed

### 9.10 Missing GITHUB_TOKEN (PR Event)

**Scenario**: PR event but no GITHUB_TOKEN set

**Expected Behavior**:
- Log error: "GITHUB_TOKEN is now required to scan pull requests"
- Exit code: 1
- No scan executed

### 9.11 Organization Without License

**Scenario**: Organization repo without GITLEAKS_LICENSE

**Expected Behavior**:
- Detect organization type
- Check for license
- Log error: "missing gitleaks license"
- Exit code: 1

**Note**: Currently disabled in code but framework exists

### 9.12 Gitleaks Binary Not Found

**Scenario**: Gitleaks installation fails or binary missing

**Expected Behavior**:
- Command execution fails
- Log error
- Exit code: 127 (command not found)

### 9.13 Gitleaks Timeout

**Scenario**: Gitleaks runs longer than 60 seconds

**Expected Behavior**:
- Process killed after timeout
- Log timeout error
- Exit code: non-zero

---

## 10. API REQUEST/RESPONSE EXAMPLES

### 10.1 Get User Type

**Request**:
```
GET /users/octocat
Accept: application/vnd.github.v3+json
Authorization: token ghs_xxxxx
```

**Response** (Personal User):
```json
{
  "login": "octocat",
  "id": 583231,
  "type": "User",
  "name": "The Octocat",
  "email": "octocat@github.com"
}
```

**Response** (Organization):
```json
{
  "login": "github",
  "id": 9919,
  "type": "Organization",
  "name": "GitHub",
  "email": null
}
```

### 10.2 Get PR Commits

**Request**:
```
GET /repos/octocat/my-repo/pulls/42/commits
Accept: application/vnd.github.v3+json
Authorization: token ghs_xxxxx
```

**Response**:
```json
[
  {
    "sha": "abc123def4567890abcdef1234567890abcdef12",
    "commit": {
      "message": "Add feature",
      "author": {
        "name": "John Doe",
        "email": "john@example.com",
        "date": "2025-10-15T14:30:00Z"
      }
    }
  },
  {
    "sha": "def456abc7890123def4567890abcdef1234567",
    "commit": {
      "message": "Fix bug",
      "author": {
        "name": "Jane Smith",
        "email": "jane@example.com",
        "date": "2025-10-15T15:45:00Z"
      }
    }
  }
]
```

**Usage**: Extract first and last SHA for commit range

### 10.3 Get PR Review Comments

**Request**:
```
GET /repos/octocat/my-repo/pulls/42/comments
Accept: application/vnd.github.v3+json
Authorization: token ghs_xxxxx
```

**Response**:
```json
[
  {
    "id": 987654321,
    "body": "🛑 **Gitleaks** has detected a secret...",
    "path": "src/config/aws.js",
    "line": null,
    "original_line": 12,
    "commit_id": "abc123def456",
    "created_at": "2025-10-15T16:00:00Z",
    "user": {
      "login": "github-actions[bot]"
    }
  }
]
```

**Usage**: Check for duplicate comments before posting

### 10.4 Create PR Review Comment

**Request**:
```
POST /repos/octocat/my-repo/pulls/42/comments
Accept: application/vnd.github.v3+json
Authorization: token ghs_xxxxx
Content-Type: application/json
```

**Body**:
```json
{
  "body": "🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123def456...",
  "commit_id": "abc123def4567890abcdef1234567890abcdef12",
  "path": "src/config/aws.js",
  "side": "RIGHT",
  "line": 12
}
```

**Success Response** (201 Created):
```json
{
  "id": 123456789,
  "body": "🛑 **Gitleaks** has detected a secret...",
  "path": "src/config/aws.js",
  "line": null,
  "original_line": 12,
  "commit_id": "abc123def456",
  "created_at": "2025-10-15T16:30:00Z"
}
```

**Error Response** (422 Unprocessable Entity):
```json
{
  "message": "Validation Failed",
  "errors": [
    {
      "resource": "PullRequestReviewComment",
      "code": "custom",
      "message": "pull_request_review_comment.diff_hunk can't be blank"
    }
  ]
}
```

**Common Error Reasons**:
- Line not in PR diff
- Commit no longer in PR
- File not changed in PR
- Large diff (position out of range)

### 10.5 Get Latest Gitleaks Release

**Request**:
```
GET /repos/zricethezav/gitleaks/releases/latest
Accept: application/vnd.github.v3+json
```

**Response**:
```json
{
  "tag_name": "v8.24.3",
  "name": "v8.24.3",
  "draft": false,
  "prerelease": false,
  "created_at": "2025-09-15T10:00:00Z",
  "published_at": "2025-09-15T11:00:00Z",
  "assets": [
    {
      "name": "gitleaks_8.24.3_linux_amd64.tar.gz",
      "browser_download_url": "https://github.com/zricethezav/gitleaks/releases/download/v8.24.3/gitleaks_8.24.3_linux_amd64.tar.gz"
    }
  ]
}
```

**Usage**: Extract version number when `GITLEAKS_VERSION=latest`

---

## 11. RUST IMPLEMENTATION EXAMPLES

### 11.1 SARIF Parsing Example

```rust
use serde::{Deserialize, Serialize};
use std::fs;

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
struct SarifMessage {
    text: String,
}

#[derive(Debug, Deserialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: PhysicalLocation,
}

#[derive(Debug, Deserialize)]
struct PhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: ArtifactLocation,
    region: Region,
}

#[derive(Debug, Deserialize)]
struct ArtifactLocation {
    uri: String,
}

#[derive(Debug, Deserialize)]
struct Region {
    #[serde(rename = "startLine")]
    start_line: u32,
    #[serde(rename = "startColumn")]
    start_column: Option<u32>,
    snippet: Option<Snippet>,
}

#[derive(Debug, Deserialize)]
struct Snippet {
    text: String,
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

fn parse_sarif(path: &str) -> Result<SarifReport, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let report: SarifReport = serde_json::from_str(&content)?;
    Ok(report)
}

fn main() {
    match parse_sarif("results.sarif") {
        Ok(report) => {
            for run in &report.runs {
                for result in &run.results {
                    println!("Found secret: {}", result.rule_id);
                    println!("  File: {}", result.locations[0].physical_location.artifact_location.uri);
                    println!("  Line: {}", result.locations[0].physical_location.region.start_line);
                    println!("  Commit: {}", result.partial_fingerprints.commit_sha);
                }
            }
        }
        Err(e) => eprintln!("Failed to parse SARIF: {}", e),
    }
}
```

### 11.2 Fingerprint Generation Example

```rust
fn generate_fingerprint(
    commit_sha: &str,
    file_path: &str,
    rule_id: &str,
    start_line: u32,
) -> String {
    format!("{}:{}:{}:{}", commit_sha, file_path, rule_id, start_line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprint_generation() {
        let fingerprint = generate_fingerprint(
            "abc123def456",
            "src/config.js",
            "aws-access-token",
            42,
        );
        assert_eq!(
            fingerprint,
            "abc123def456:src/config.js:aws-access-token:42"
        );
    }
}
```

### 11.3 Boolean Environment Variable Parsing

```rust
use std::env;

fn parse_bool_env(var_name: &str, default: bool) -> bool {
    match env::var(var_name) {
        Ok(val) => match val.as_str() {
            "false" | "0" => false,
            _ => true,
        },
        Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bool_env() {
        env::set_var("TEST_VAR", "false");
        assert_eq!(parse_bool_env("TEST_VAR", true), false);

        env::set_var("TEST_VAR", "0");
        assert_eq!(parse_bool_env("TEST_VAR", true), false);

        env::set_var("TEST_VAR", "true");
        assert_eq!(parse_bool_env("TEST_VAR", false), true);

        env::set_var("TEST_VAR", "1");
        assert_eq!(parse_bool_env("TEST_VAR", false), true);

        env::remove_var("TEST_VAR");
        assert_eq!(parse_bool_env("TEST_VAR", true), true);
    }
}
```

### 11.4 Execute Gitleaks Binary

```rust
use std::process::Command;

struct GitLeaksResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn execute_gitleaks(args: Vec<&str>) -> Result<GitLeaksResult, Box<dyn std::error::Error>> {
    let output = Command::new("gitleaks")
        .args(&args)
        .output()?;

    Ok(GitLeaksResult {
        exit_code: output.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

fn main() {
    let args = vec![
        "detect",
        "--redact",
        "-v",
        "--exit-code=2",
        "--report-format=sarif",
        "--report-path=results.sarif",
        "--log-level=debug",
    ];

    match execute_gitleaks(args) {
        Ok(result) => {
            println!("Exit code: {}", result.exit_code);
            match result.exit_code {
                0 => println!("✅ No leaks detected"),
                2 => println!("🛑 Leaks detected"),
                _ => eprintln!("❌ Error occurred"),
            }
        }
        Err(e) => eprintln!("Failed to execute gitleaks: {}", e),
    }
}
```

### 11.5 GitHub API Client Example (using octocrab)

```rust
use octocrab::{Octocrab, models::pulls::Comment};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = std::env::var("GITHUB_TOKEN")?;
    let octocrab = Octocrab::builder()
        .personal_token(token)
        .build()?;

    // Get PR commits
    let commits = octocrab
        .pulls("octocat", "my-repo")
        .list_commits(42)
        .await?;

    println!("PR has {} commits", commits.len());

    // Create review comment
    let comment_body = "🛑 **Gitleaks** has detected a secret...";

    let comment = octocrab
        .pulls("octocat", "my-repo")
        .create_review_comment(
            42,
            comment_body,
            "abc123def456",
            "src/config.js",
            12,
        )
        .await?;

    println!("Created comment: {}", comment.id);

    Ok(())
}
```

---

## CONCLUSION

This document provides practical examples, test cases, and implementation patterns for the Rust port of gitleaks-action. Use these examples to:

1. **Validate** implementation correctness
2. **Test** edge cases and error scenarios
3. **Understand** data formats and structures
4. **Implement** parsing and processing logic
5. **Integrate** with GitHub APIs

**Related Documents**:
- **GITLEAKS_INTEGRATION_SPEC.md**: Detailed specifications
- **GITLEAKS_INTEGRATION_FLOWS.md**: Visual flow diagrams

**Document Version**: 1.0
**Last Updated**: 2025-10-15
