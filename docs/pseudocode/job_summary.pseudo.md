# Job Summary Generation Pseudocode

**Project:** SecretScout - Rust Port of gitleaks-action
**Component:** GitHub Actions Job Summary (FR-5)
**Phase:** Pseudocode
**Date:** October 16, 2025
**Version:** 1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Core Algorithms](#core-algorithms)
4. [URL Generation](#url-generation)
5. [HTML Formatting](#html-formatting)
6. [Error Handling](#error-handling)
7. [Usage Examples](#usage-examples)

---

## Overview

This document provides detailed pseudocode algorithms for generating GitHub Actions job summaries. The algorithms handle:

- Three summary types: success, secrets detected, and error
- HTML table generation for detected secrets
- Repository URL generation for commits and files
- HTML escaping for security
- Feature toggle via GITLEAKS_ENABLE_SUMMARY

**Reference:** SPARC_SPECIFICATION.md Section 3.1, FR-5

---

## Data Structures

### 1. SummaryConfig

```pseudo
STRUCTURE SummaryConfig
    enabled: Boolean                   // GITLEAKS_ENABLE_SUMMARY flag
    repositoryUrl: String              // Base repository URL (e.g., "https://github.com/owner/repo")
    repositoryOwner: String            // Repository owner
    repositoryName: String             // Repository name
END STRUCTURE
```

### 2. SummaryResult

```pseudo
STRUCTURE SummaryResult
    content: String                    // Markdown/HTML content for job summary
    secretCount: Integer               // Number of secrets in summary
    wasGenerated: Boolean              // Whether summary was actually generated
    error: String                      // Error message if generation failed
END STRUCTURE
```

### 3. SecretTableRow

```pseudo
STRUCTURE SecretTableRow
    ruleId: String                     // Detection rule (plain text)
    commitSha: String                  // Full commit SHA (40 chars)
    commitShortSha: String             // First 7 chars of commit
    commitUrl: String                  // Full URL to commit
    filePath: String                   // Relative file path
    secretUrl: String                  // URL to file line with secret
    startLine: Integer                 // Line number
    author: String                     // Commit author (HTML-escaped)
    date: String                       // Commit date (HTML-escaped)
    email: String                      // Author email (HTML-escaped)
    fileUrl: String                    // URL to file (without line anchor)
END STRUCTURE
```

### 4. ExitCode

```pseudo
ENUM ExitCode
    SUCCESS = 0                        // No secrets detected
    ERROR = 1                          // Gitleaks error
    SECRETS_FOUND = 2                  // Secrets detected
END ENUM
```

---

## Core Algorithms

### 1. GenerateSummary

**Purpose:** Main entry point for job summary generation. Routes to appropriate summary type based on exit code.

**Inputs:**
- `exitCode`: ExitCode - Gitleaks exit code (0, 1, or 2)
- `secrets`: Array<DetectedSecret> - List of detected secrets (from SARIF)
- `config`: SummaryConfig - Configuration including repository URL

**Outputs:**
- `SummaryResult` - Generated summary content or error

**Algorithm:**

```pseudo
FUNCTION GenerateSummary(
    exitCode: ExitCode,
    secrets: Array<DetectedSecret>,
    config: SummaryConfig
) -> SummaryResult
    // Initialize result
    result = new SummaryResult()
    result.wasGenerated = FALSE
    result.content = NULL
    result.error = NULL
    result.secretCount = 0

    // Step 1: Check if summary generation is enabled
    IF NOT config.enabled THEN
        LogInfo("Job summary generation is disabled (GITLEAKS_ENABLE_SUMMARY=false)")
        result.wasGenerated = FALSE
        RETURN result
    END IF

    // Step 2: Validate repository URL
    IF config.repositoryUrl IS NULL OR config.repositoryUrl == "" THEN
        LogWarning("Repository URL not available, cannot generate links")
        // Continue anyway, generate summary without links
    END IF

    // Step 3: Route to appropriate summary generator based on exit code
    TRY
        SWITCH exitCode
            CASE ExitCode.SUCCESS:
                // No secrets detected
                result.content = GenerateSuccessSummary()
                result.secretCount = 0
                result.wasGenerated = TRUE

            CASE ExitCode.SECRETS_FOUND:
                // Secrets detected - generate table
                IF secrets IS NULL OR secrets.length == 0 THEN
                    LogWarning("Exit code 2 but no secrets provided, generating error summary")
                    result.content = GenerateErrorSummary(2)
                ELSE
                    result.content = GenerateSecretsSummary(secrets, config)
                    result.secretCount = secrets.length
                END IF
                result.wasGenerated = TRUE

            CASE ExitCode.ERROR:
                // Gitleaks error
                result.content = GenerateErrorSummary(1)
                result.secretCount = 0
                result.wasGenerated = TRUE

            DEFAULT:
                // Unexpected exit code
                LogWarning("Unexpected exit code: " + exitCode)
                result.content = GenerateErrorSummary(exitCode)
                result.wasGenerated = TRUE
        END SWITCH

    CATCH Exception e
        LogError("Failed to generate job summary: " + e.message)
        result.error = "Summary generation failed: " + e.message
        result.wasGenerated = FALSE
        RETURN result
    END TRY

    // Step 4: Validate generated content
    IF result.content IS NULL OR result.content == "" THEN
        LogError("Generated summary content is empty")
        result.error = "Summary content is empty"
        result.wasGenerated = FALSE
    END IF

    RETURN result
END FUNCTION
```

---

### 2. GenerateSuccessSummary

**Purpose:** Generate summary for successful scan with no secrets detected (exit code 0).

**Inputs:** None

**Outputs:**
- `String` - Markdown content for success message

**Algorithm:**

```pseudo
FUNCTION GenerateSuccessSummary() -> String
    // Simple success message with checkmark emoji
    // Format per specification: "## No leaks detected ✅"

    summary = "## No leaks detected ✅\n"

    RETURN summary
END FUNCTION
```

**Output Example:**
```markdown
## No leaks detected ✅
```

---

### 3. GenerateSecretsSummary

**Purpose:** Generate summary table for detected secrets (exit code 2).

**Inputs:**
- `secrets`: Array<DetectedSecret> - List of detected secrets
- `config`: SummaryConfig - Configuration with repository URL

**Outputs:**
- `String` - Markdown/HTML content with secrets table

**Algorithm:**

```pseudo
FUNCTION GenerateSecretsSummary(
    secrets: Array<DetectedSecret>,
    config: SummaryConfig
) -> String
    // Step 1: Start with heading
    summary = "## 🛑 Gitleaks detected secrets 🛑\n\n"

    // Step 2: Generate HTML table header
    summary += GenerateTableHeader()

    // Step 3: Generate table rows for each secret
    FOR EACH secret IN secrets DO
        // Build table row structure
        tableRow = new SecretTableRow()

        // Basic fields
        tableRow.ruleId = EscapeHTML(secret.ruleId)
        tableRow.commitSha = secret.commitSha
        tableRow.startLine = secret.startLine
        tableRow.author = EscapeHTML(secret.author)
        tableRow.date = EscapeHTML(secret.date)
        tableRow.email = EscapeHTML(secret.email)
        tableRow.filePath = secret.filePath

        // Generate commit short SHA (first 7 chars)
        tableRow.commitShortSha = GenerateShortSha(secret.commitSha)

        // Generate URLs (only if repository URL is available)
        IF config.repositoryUrl IS NOT NULL AND config.repositoryUrl != "" THEN
            tableRow.commitUrl = GenerateCommitURL(
                config.repositoryUrl,
                secret.commitSha
            )

            tableRow.secretUrl = GenerateSecretURL(
                config.repositoryUrl,
                secret.commitSha,
                secret.filePath,
                secret.startLine
            )

            tableRow.fileUrl = GenerateFileURL(
                config.repositoryUrl,
                secret.commitSha,
                secret.filePath
            )
        ELSE
            // No URLs if repository URL not available
            tableRow.commitUrl = NULL
            tableRow.secretUrl = NULL
            tableRow.fileUrl = NULL
        END IF

        // Generate table row HTML
        rowHtml = GenerateTableRow(tableRow)
        summary += rowHtml
    END FOR

    // Step 4: Close table
    summary += GenerateTableFooter()

    RETURN summary
END FUNCTION
```

---

### 4. GenerateErrorSummary

**Purpose:** Generate summary for gitleaks error (exit code 1 or other errors).

**Inputs:**
- `exitCode`: Integer - The exit code returned by gitleaks

**Outputs:**
- `String` - Markdown content for error message

**Algorithm:**

```pseudo
FUNCTION GenerateErrorSummary(exitCode: Integer) -> String
    // Format per specification: "## ❌ Gitleaks exited with error. Exit code [1]"

    summary = "## ❌ Gitleaks exited with error. Exit code [" +
              ToString(exitCode) + "]\n"

    RETURN summary
END FUNCTION
```

**Output Example:**
```markdown
## ❌ Gitleaks exited with error. Exit code [1]
```

---

### 5. GenerateTableHeader

**Purpose:** Generate HTML table header with column names.

**Inputs:** None

**Outputs:**
- `String` - HTML table opening tag and header row

**Algorithm:**

```pseudo
FUNCTION GenerateTableHeader() -> String
    // Build HTML table with 8 columns per specification

    html = "<table>\n"
    html += "  <thead>\n"
    html += "    <tr>\n"
    html += "      <th>Rule ID</th>\n"
    html += "      <th>Commit</th>\n"
    html += "      <th>Secret URL</th>\n"
    html += "      <th>Start Line</th>\n"
    html += "      <th>Author</th>\n"
    html += "      <th>Date</th>\n"
    html += "      <th>Email</th>\n"
    html += "      <th>File</th>\n"
    html += "    </tr>\n"
    html += "  </thead>\n"
    html += "  <tbody>\n"

    RETURN html
END FUNCTION
```

---

### 6. GenerateTableRow

**Purpose:** Generate single HTML table row for one detected secret.

**Inputs:**
- `row`: SecretTableRow - Table row data structure

**Outputs:**
- `String` - HTML table row (<tr>...</tr>)

**Algorithm:**

```pseudo
FUNCTION GenerateTableRow(row: SecretTableRow) -> String
    html = "    <tr>\n"

    // Column 1: Rule ID (plain text, HTML-escaped)
    html += "      <td>" + row.ruleId + "</td>\n"

    // Column 2: Commit (hyperlink to commit, showing first 7 chars)
    IF row.commitUrl IS NOT NULL THEN
        html += "      <td><a href=\"" + row.commitUrl + "\">" +
                row.commitShortSha + "</a></td>\n"
    ELSE
        // No link if URL not available
        html += "      <td>" + row.commitShortSha + "</td>\n"
    END IF

    // Column 3: Secret URL (hyperlink to file:line, showing "View")
    IF row.secretUrl IS NOT NULL THEN
        html += "      <td><a href=\"" + row.secretUrl + "\">View</a></td>\n"
    ELSE
        html += "      <td>-</td>\n"
    END IF

    // Column 4: Start Line (plain integer)
    html += "      <td>" + ToString(row.startLine) + "</td>\n"

    // Column 5: Author (plain text, HTML-escaped)
    html += "      <td>" + row.author + "</td>\n"

    // Column 6: Date (plain text, HTML-escaped)
    html += "      <td>" + row.date + "</td>\n"

    // Column 7: Email (plain text, HTML-escaped)
    html += "      <td>" + row.email + "</td>\n"

    // Column 8: File (hyperlink to file, showing file path)
    IF row.fileUrl IS NOT NULL THEN
        escapedPath = EscapeHTML(row.filePath)
        html += "      <td><a href=\"" + row.fileUrl + "\">" +
                escapedPath + "</a></td>\n"
    ELSE
        escapedPath = EscapeHTML(row.filePath)
        html += "      <td>" + escapedPath + "</td>\n"
    END IF

    html += "    </tr>\n"

    RETURN html
END FUNCTION
```

**Output Example:**
```html
<tr>
  <td>aws-access-token</td>
  <td><a href="https://github.com/owner/repo/commit/abc123d">abc123d</a></td>
  <td><a href="https://github.com/owner/repo/blob/abc123d/src/config.js#L42">View</a></td>
  <td>42</td>
  <td>John Doe</td>
  <td>2025-10-15T14:30:00Z</td>
  <td>john@example.com</td>
  <td><a href="https://github.com/owner/repo/blob/abc123d/src/config.js">src/config.js</a></td>
</tr>
```

---

### 7. GenerateTableFooter

**Purpose:** Generate HTML table closing tags.

**Inputs:** None

**Outputs:**
- `String` - HTML table closing tags

**Algorithm:**

```pseudo
FUNCTION GenerateTableFooter() -> String
    html = "  </tbody>\n"
    html += "</table>\n"

    RETURN html
END FUNCTION
```

---

## URL Generation

### 1. GenerateCommitURL

**Purpose:** Generate GitHub URL for a commit.

**Inputs:**
- `repositoryUrl`: String - Base repository URL (e.g., "https://github.com/owner/repo")
- `commitSha`: String - Full commit SHA (40 hex chars)

**Outputs:**
- `String` - Full commit URL

**Algorithm:**

```pseudo
FUNCTION GenerateCommitURL(
    repositoryUrl: String,
    commitSha: String
) -> String
    // Step 1: Validate inputs
    IF repositoryUrl IS NULL OR repositoryUrl == "" THEN
        LogWarning("Repository URL is empty, cannot generate commit URL")
        RETURN NULL
    END IF

    IF commitSha IS NULL OR commitSha == "" THEN
        LogWarning("Commit SHA is empty, cannot generate commit URL")
        RETURN NULL
    END IF

    // Step 2: Remove trailing slash from repository URL if present
    cleanUrl = repositoryUrl
    IF cleanUrl.endsWith("/") THEN
        cleanUrl = cleanUrl.substring(0, cleanUrl.length - 1)
    END IF

    // Step 3: Validate commit SHA format
    IF NOT IsValidCommitSha(commitSha) THEN
        LogWarning("Invalid commit SHA format: " + commitSha)
        // Use it anyway but log warning
    END IF

    // Step 4: Build commit URL
    // Pattern: {repo_url}/commit/{commitSha}
    commitUrl = cleanUrl + "/commit/" + commitSha

    RETURN commitUrl
END FUNCTION
```

**Example:**
```
Input:
  repositoryUrl = "https://github.com/gitleaks/gitleaks-action"
  commitSha = "abc123def456789"

Output:
  "https://github.com/gitleaks/gitleaks-action/commit/abc123def456789"
```

---

### 2. GenerateSecretURL

**Purpose:** Generate GitHub URL for file line where secret was detected.

**Inputs:**
- `repositoryUrl`: String - Base repository URL
- `commitSha`: String - Full commit SHA
- `filePath`: String - Relative file path
- `startLine`: Integer - Line number

**Outputs:**
- `String` - Full URL to file line

**Algorithm:**

```pseudo
FUNCTION GenerateSecretURL(
    repositoryUrl: String,
    commitSha: String,
    filePath: String,
    startLine: Integer
) -> String
    // Step 1: Validate inputs
    IF repositoryUrl IS NULL OR repositoryUrl == "" THEN
        LogWarning("Repository URL is empty, cannot generate secret URL")
        RETURN NULL
    END IF

    IF commitSha IS NULL OR commitSha == "" THEN
        LogWarning("Commit SHA is empty, cannot generate secret URL")
        RETURN NULL
    END IF

    IF filePath IS NULL OR filePath == "" THEN
        LogWarning("File path is empty, cannot generate secret URL")
        RETURN NULL
    END IF

    IF startLine < 1 THEN
        LogWarning("Invalid line number: " + startLine + ", using 1")
        startLine = 1
    END IF

    // Step 2: Remove trailing slash from repository URL
    cleanUrl = repositoryUrl
    IF cleanUrl.endsWith("/") THEN
        cleanUrl = cleanUrl.substring(0, cleanUrl.length - 1)
    END IF

    // Step 3: Normalize file path
    // - Ensure forward slashes (not backslashes)
    // - Remove leading slash if present
    normalizedPath = filePath.replace("\\", "/")
    IF normalizedPath.startsWith("/") THEN
        normalizedPath = normalizedPath.substring(1)
    END IF

    // Step 4: URL-encode file path (handle spaces, special chars)
    encodedPath = URLEncode(normalizedPath)

    // Step 5: Build secret URL
    // Pattern: {repo_url}/blob/{commitSha}/{filePath}#L{startLine}
    secretUrl = cleanUrl + "/blob/" + commitSha + "/" + encodedPath + "#L" + ToString(startLine)

    RETURN secretUrl
END FUNCTION
```

**Example:**
```
Input:
  repositoryUrl = "https://github.com/gitleaks/gitleaks-action"
  commitSha = "abc123def456789"
  filePath = "src/config.js"
  startLine = 42

Output:
  "https://github.com/gitleaks/gitleaks-action/blob/abc123def456789/src/config.js#L42"
```

---

### 3. GenerateFileURL

**Purpose:** Generate GitHub URL for file (without line anchor).

**Inputs:**
- `repositoryUrl`: String - Base repository URL
- `commitSha`: String - Full commit SHA
- `filePath`: String - Relative file path

**Outputs:**
- `String` - Full URL to file

**Algorithm:**

```pseudo
FUNCTION GenerateFileURL(
    repositoryUrl: String,
    commitSha: String,
    filePath: String
) -> String
    // Step 1: Validate inputs
    IF repositoryUrl IS NULL OR repositoryUrl == "" THEN
        RETURN NULL
    END IF

    IF commitSha IS NULL OR commitSha == "" THEN
        RETURN NULL
    END IF

    IF filePath IS NULL OR filePath == "" THEN
        RETURN NULL
    END IF

    // Step 2: Remove trailing slash from repository URL
    cleanUrl = repositoryUrl
    IF cleanUrl.endsWith("/") THEN
        cleanUrl = cleanUrl.substring(0, cleanUrl.length - 1)
    END IF

    // Step 3: Normalize file path
    normalizedPath = filePath.replace("\\", "/")
    IF normalizedPath.startsWith("/") THEN
        normalizedPath = normalizedPath.substring(1)
    END IF

    // Step 4: URL-encode file path
    encodedPath = URLEncode(normalizedPath)

    // Step 5: Build file URL (same as secret URL but without #L{line})
    // Pattern: {repo_url}/blob/{commitSha}/{filePath}
    fileUrl = cleanUrl + "/blob/" + commitSha + "/" + encodedPath

    RETURN fileUrl
END FUNCTION
```

---

### 4. GenerateShortSha

**Purpose:** Generate short commit SHA (first 7 characters).

**Inputs:**
- `commitSha`: String - Full commit SHA

**Outputs:**
- `String` - Short SHA (7 chars) or original if invalid

**Algorithm:**

```pseudo
FUNCTION GenerateShortSha(commitSha: String) -> String
    // Step 1: Validate input
    IF commitSha IS NULL OR commitSha == "" THEN
        RETURN "unknown"
    END IF

    // Step 2: Check if valid SHA format
    IF NOT IsValidCommitSha(commitSha) THEN
        // If not valid, return as-is (might be "unknown" or other placeholder)
        RETURN commitSha
    END IF

    // Step 3: Extract first 7 characters
    IF commitSha.length >= 7 THEN
        RETURN commitSha.substring(0, 7)
    ELSE
        // SHA is shorter than 7 chars (unusual but possible)
        RETURN commitSha
    END IF
END FUNCTION
```

**Example:**
```
Input: "abc123def456789012345678901234567890"
Output: "abc123d"
```

---

## HTML Formatting

### 1. EscapeHTML

**Purpose:** Escape HTML special characters to prevent injection attacks and rendering issues.

**Inputs:**
- `text`: String - Raw text to escape

**Outputs:**
- `String` - HTML-escaped text

**Algorithm:**

```pseudo
FUNCTION EscapeHTML(text: String) -> String
    // Step 1: Handle null or empty
    IF text IS NULL THEN
        RETURN ""
    END IF

    IF text == "" THEN
        RETURN ""
    END IF

    // Step 2: Escape special HTML characters
    // Order matters: & must be first to avoid double-escaping
    escaped = text

    // Ampersand: & → &amp;
    escaped = escaped.replace("&", "&amp;")

    // Less than: < → &lt;
    escaped = escaped.replace("<", "&lt;")

    // Greater than: > → &gt;
    escaped = escaped.replace(">", "&gt;")

    // Double quote: " → &quot;
    escaped = escaped.replace("\"", "&quot;")

    // Single quote: ' → &#39; (more compatible than &apos;)
    escaped = escaped.replace("'", "&#39;")

    RETURN escaped
END FUNCTION
```

**Example:**
```
Input: "Author <john@example.com> & Co."
Output: "Author &lt;john@example.com&gt; &amp; Co."
```

---

### 2. URLEncode

**Purpose:** Encode file path for use in URL.

**Inputs:**
- `path`: String - File path to encode

**Outputs:**
- `String` - URL-encoded path

**Algorithm:**

```pseudo
FUNCTION URLEncode(path: String) -> String
    // Step 1: Handle null or empty
    IF path IS NULL OR path == "" THEN
        RETURN ""
    END IF

    // Step 2: Encode special characters for URLs
    // Use standard URL encoding (RFC 3986)
    encoded = ""

    FOR EACH char IN path DO
        IF IsAlphanumeric(char) OR char IN ['-', '_', '.', '~', '/'] THEN
            // Safe characters - no encoding needed
            // Note: '/' is safe for path segments
            encoded += char
        ELSE
            // Encode as %XX where XX is hex value
            hexValue = ToHexString(GetCharCode(char))
            encoded += "%" + hexValue
        END IF
    END FOR

    RETURN encoded
END FUNCTION
```

**Example:**
```
Input: "src/my config/file.js"
Output: "src/my%20config/file.js"
```

---

### 3. ValidateRepositoryURL

**Purpose:** Validate repository URL format.

**Inputs:**
- `url`: String - Repository URL to validate

**Outputs:**
- `Boolean` - TRUE if valid, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION ValidateRepositoryURL(url: String) -> Boolean
    // Step 1: Check null or empty
    IF url IS NULL OR url == "" THEN
        RETURN FALSE
    END IF

    // Step 2: Check starts with http:// or https://
    IF NOT (url.startsWith("http://") OR url.startsWith("https://")) THEN
        LogWarning("Repository URL should start with http:// or https://")
        RETURN FALSE
    END IF

    // Step 3: Check contains github.com (or other supported hosts)
    supportedHosts = ["github.com", "gitlab.com", "bitbucket.org"]
    containsValidHost = FALSE

    FOR EACH host IN supportedHosts DO
        IF url.contains(host) THEN
            containsValidHost = TRUE
            BREAK
        END IF
    END FOR

    IF NOT containsValidHost THEN
        LogWarning("Repository URL does not contain recognized host: " + url)
        // Allow anyway - might be self-hosted instance
    END IF

    // Step 4: Check for obvious malformations
    IF url.contains(" ") THEN
        LogWarning("Repository URL contains spaces")
        RETURN FALSE
    END IF

    RETURN TRUE
END FUNCTION
```

---

## Error Handling

### Error Categories

#### 1. Configuration Errors

```pseudo
ERROR HANDLING ConfigurationErrors
    WHEN SummaryDisabled THEN
        LogInfo("Job summary generation is disabled")
        RETURN empty SummaryResult with wasGenerated = FALSE

    WHEN MissingRepositoryURL THEN
        LogWarning("Repository URL not available, generating summary without links")
        CONTINUE with NULL URLs

    WHEN InvalidRepositoryURL THEN
        LogWarning("Invalid repository URL format: " + url)
        CONTINUE with NULL URLs
END ERROR HANDLING
```

---

#### 2. Data Errors

```pseudo
ERROR HANDLING DataErrors
    WHEN EmptySecretsArray THEN
        LogWarning("Exit code 2 but secrets array is empty")
        RETURN GenerateErrorSummary(2)

    WHEN NullSecretsArray THEN
        LogWarning("Exit code 2 but secrets array is null")
        RETURN GenerateErrorSummary(2)

    WHEN MissingSecretFields THEN
        LogWarning("Secret missing required fields, using defaults")
        USE default values ("unknown", 1, etc.)
END ERROR HANDLING
```

---

#### 3. Generation Errors

```pseudo
ERROR HANDLING GenerationErrors
    WHEN HTMLGenerationFailed THEN
        LogError("Failed to generate HTML table: " + error.message)
        RETURN SummaryResult with error message

    WHEN URLGenerationFailed THEN
        LogWarning("Failed to generate URL: " + error.message)
        USE plain text instead of hyperlink

    WHEN StringConcatenationFailed THEN
        LogError("Failed to build summary string: " + error.message)
        RETURN SummaryResult with error message
END ERROR HANDLING
```

---

### Defensive Programming Patterns

#### 1. Null-Safe Field Access

```pseudo
FUNCTION SafeGetField(secret: DetectedSecret, fieldName: String, default: Any) -> Any
    // Safely access fields with fallback defaults

    IF secret IS NULL THEN
        RETURN default
    END IF

    SWITCH fieldName
        CASE "ruleId":
            RETURN IF secret.ruleId IS NOT NULL THEN secret.ruleId ELSE default

        CASE "commitSha":
            RETURN IF secret.commitSha IS NOT NULL THEN secret.commitSha ELSE default

        CASE "filePath":
            RETURN IF secret.filePath IS NOT NULL THEN secret.filePath ELSE default

        CASE "startLine":
            RETURN IF secret.startLine >= 1 THEN secret.startLine ELSE default

        CASE "author":
            RETURN IF secret.author IS NOT NULL THEN secret.author ELSE default

        CASE "date":
            RETURN IF secret.date IS NOT NULL THEN secret.date ELSE default

        CASE "email":
            RETURN IF secret.email IS NOT NULL THEN secret.email ELSE default

        DEFAULT:
            RETURN default
    END SWITCH
END FUNCTION
```

---

#### 2. Safe String Building

```pseudo
FUNCTION SafeAppend(builder: StringBuilder, content: String) -> Boolean
    // Safely append content with validation

    TRY
        IF content IS NULL THEN
            LogWarning("Attempting to append null content, skipping")
            RETURN FALSE
        END IF

        builder.append(content)
        RETURN TRUE

    CATCH Exception e
        LogError("Failed to append content: " + e.message)
        RETURN FALSE
    END TRY
END FUNCTION
```

---

## Usage Examples

### Example 1: Generating Success Summary

```pseudo
FUNCTION ExampleSuccessSummary()
    // Configuration
    config = new SummaryConfig()
    config.enabled = TRUE
    config.repositoryUrl = "https://github.com/gitleaks/gitleaks-action"

    // Generate summary for no secrets (exit code 0)
    result = GenerateSummary(
        ExitCode.SUCCESS,
        [],  // Empty secrets array
        config
    )

    // Check result
    IF result.wasGenerated THEN
        LogInfo("Summary generated successfully:")
        LogInfo(result.content)

        // Write to GitHub Actions summary file
        WriteSummaryToFile(result.content, GetEnv("GITHUB_STEP_SUMMARY"))
    ELSE
        LogWarning("Summary was not generated")
        IF result.error IS NOT NULL THEN
            LogError("Error: " + result.error)
        END IF
    END IF
END FUNCTION
```

**Expected Output:**
```markdown
## No leaks detected ✅
```

---

### Example 2: Generating Secrets Summary

```pseudo
FUNCTION ExampleSecretsSummary()
    // Configuration
    config = new SummaryConfig()
    config.enabled = TRUE
    config.repositoryUrl = "https://github.com/gitleaks/gitleaks-action"

    // Create sample detected secrets
    secrets = []

    secret1 = new DetectedSecret()
    secret1.ruleId = "aws-access-token"
    secret1.commitSha = "abc123def456789012345678901234567890"
    secret1.filePath = "src/config/aws.js"
    secret1.startLine = 42
    secret1.author = "John Doe"
    secret1.date = "2025-10-15T14:30:00Z"
    secret1.email = "john@example.com"
    secrets.append(secret1)

    secret2 = new DetectedSecret()
    secret2.ruleId = "generic-api-key"
    secret2.commitSha = "def789abc123456789012345678901234567890"
    secret2.filePath = "config.yml"
    secret2.startLine = 23
    secret2.author = "Jane Smith"
    secret2.date = "2025-10-14T09:15:00Z"
    secret2.email = "jane@example.com"
    secrets.append(secret2)

    // Generate summary for detected secrets (exit code 2)
    result = GenerateSummary(
        ExitCode.SECRETS_FOUND,
        secrets,
        config
    )

    // Output result
    IF result.wasGenerated THEN
        LogInfo("Summary generated with " + result.secretCount + " secrets")
        WriteSummaryToFile(result.content, GetEnv("GITHUB_STEP_SUMMARY"))
    END IF
END FUNCTION
```

**Expected Output:**
```markdown
## 🛑 Gitleaks detected secrets 🛑

<table>
  <thead>
    <tr>
      <th>Rule ID</th>
      <th>Commit</th>
      <th>Secret URL</th>
      <th>Start Line</th>
      <th>Author</th>
      <th>Date</th>
      <th>Email</th>
      <th>File</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>aws-access-token</td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/commit/abc123def456789012345678901234567890">abc123d</a></td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/blob/abc123def456789012345678901234567890/src/config/aws.js#L42">View</a></td>
      <td>42</td>
      <td>John Doe</td>
      <td>2025-10-15T14:30:00Z</td>
      <td>john@example.com</td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/blob/abc123def456789012345678901234567890/src/config/aws.js">src/config/aws.js</a></td>
    </tr>
    <tr>
      <td>generic-api-key</td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/commit/def789abc123456789012345678901234567890">def789a</a></td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/blob/def789abc123456789012345678901234567890/config.yml#L23">View</a></td>
      <td>23</td>
      <td>Jane Smith</td>
      <td>2025-10-14T09:15:00Z</td>
      <td>jane@example.com</td>
      <td><a href="https://github.com/gitleaks/gitleaks-action/blob/def789abc123456789012345678901234567890/config.yml">config.yml</a></td>
    </tr>
  </tbody>
</table>
```

---

### Example 3: Generating Error Summary

```pseudo
FUNCTION ExampleErrorSummary()
    // Configuration
    config = new SummaryConfig()
    config.enabled = TRUE
    config.repositoryUrl = "https://github.com/gitleaks/gitleaks-action"

    // Generate summary for gitleaks error (exit code 1)
    result = GenerateSummary(
        ExitCode.ERROR,
        NULL,  // No secrets for error case
        config
    )

    // Output result
    IF result.wasGenerated THEN
        LogInfo("Error summary generated:")
        LogInfo(result.content)
        WriteSummaryToFile(result.content, GetEnv("GITHUB_STEP_SUMMARY"))
    END IF
END FUNCTION
```

**Expected Output:**
```markdown
## ❌ Gitleaks exited with error. Exit code [1]
```

---

### Example 4: Summary Disabled

```pseudo
FUNCTION ExampleSummaryDisabled()
    // Configuration with summary disabled
    config = new SummaryConfig()
    config.enabled = FALSE  // GITLEAKS_ENABLE_SUMMARY=false
    config.repositoryUrl = "https://github.com/gitleaks/gitleaks-action"

    // Create sample secrets
    secrets = [...]  // Some detected secrets

    // Attempt to generate summary
    result = GenerateSummary(
        ExitCode.SECRETS_FOUND,
        secrets,
        config
    )

    // Check result
    IF NOT result.wasGenerated THEN
        LogInfo("Summary generation was skipped (disabled)")
        // No summary file written
    END IF

    // Continue with other post-processing (PR comments, artifacts, etc.)
END FUNCTION
```

---

### Example 5: Summary Without Repository URL

```pseudo
FUNCTION ExampleSummaryNoURL()
    // Configuration without repository URL
    config = new SummaryConfig()
    config.enabled = TRUE
    config.repositoryUrl = NULL  // URL not available

    // Create sample secret
    secret = new DetectedSecret()
    secret.ruleId = "aws-access-token"
    secret.commitSha = "abc123def"
    secret.filePath = "src/config.js"
    secret.startLine = 42
    secret.author = "John Doe"
    secret.date = "2025-10-15"
    secret.email = "john@example.com"

    secrets = [secret]

    // Generate summary (will work but without hyperlinks)
    result = GenerateSummary(
        ExitCode.SECRETS_FOUND,
        secrets,
        config
    )

    // Summary will be generated with plain text instead of links
    LogInfo("Summary generated without hyperlinks:")
    LogInfo(result.content)
END FUNCTION
```

**Expected Output (without links):**
```markdown
## 🛑 Gitleaks detected secrets 🛑

<table>
  <thead>
    <tr>
      <th>Rule ID</th>
      <th>Commit</th>
      <th>Secret URL</th>
      <th>Start Line</th>
      <th>Author</th>
      <th>Date</th>
      <th>Email</th>
      <th>File</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>aws-access-token</td>
      <td>abc123d</td>
      <td>-</td>
      <td>42</td>
      <td>John Doe</td>
      <td>2025-10-15</td>
      <td>john@example.com</td>
      <td>src/config.js</td>
    </tr>
  </tbody>
</table>
```

---

### Example 6: HTML Escaping Test

```pseudo
FUNCTION ExampleHTMLEscaping()
    // Test HTML escaping with special characters

    secret = new DetectedSecret()
    secret.ruleId = "test-rule"
    secret.commitSha = "abc123def"
    secret.filePath = "<script>alert('xss')</script>.js"
    secret.startLine = 1
    secret.author = "John <Hacker> Doe"
    secret.date = "2025-10-15"
    secret.email = "john+test@example.com"

    secrets = [secret]

    config = new SummaryConfig()
    config.enabled = TRUE
    config.repositoryUrl = "https://github.com/test/repo"

    // Generate summary - should escape HTML characters
    result = GenerateSummary(
        ExitCode.SECRETS_FOUND,
        secrets,
        config
    )

    // Verify HTML is escaped
    IF result.content.contains("&lt;script&gt;") THEN
        LogInfo("✓ HTML escaping working correctly")
    ELSE
        LogError("✗ HTML escaping failed - XSS vulnerability!")
    END IF
END FUNCTION
```

---

## Integration with GitHub Actions

### Writing Summary to GitHub Actions

```pseudo
FUNCTION WriteSummaryToFile(content: String, summaryFilePath: String) -> Boolean
    // Write summary content to GITHUB_STEP_SUMMARY file

    // Step 1: Validate inputs
    IF content IS NULL OR content == "" THEN
        LogWarning("Summary content is empty, nothing to write")
        RETURN FALSE
    END IF

    IF summaryFilePath IS NULL OR summaryFilePath == "" THEN
        LogError("GITHUB_STEP_SUMMARY environment variable not set")
        RETURN FALSE
    END IF

    // Step 2: Write to file (append mode)
    TRY
        // Open file in append mode (GitHub Actions expects append)
        file = OpenFile(summaryFilePath, AppendMode)

        // Write content
        file.write(content)

        // Add newline at end
        file.write("\n")

        // Close file
        file.close()

        LogInfo("Job summary written to: " + summaryFilePath)
        RETURN TRUE

    CATCH IOException e
        LogError("Failed to write job summary: " + e.message)
        RETURN FALSE
    END TRY
END FUNCTION
```

---

### Getting Repository URL from Environment

```pseudo
FUNCTION GetRepositoryURLFromEnvironment() -> String
    // Extract repository URL from GitHub Actions environment variables

    // Step 1: Try to get from GITHUB_SERVER_URL and GITHUB_REPOSITORY
    serverUrl = GetEnv("GITHUB_SERVER_URL")  // e.g., "https://github.com"
    repository = GetEnv("GITHUB_REPOSITORY")  // e.g., "owner/repo"

    IF serverUrl IS NOT NULL AND repository IS NOT NULL THEN
        // Build URL
        repositoryUrl = serverUrl

        // Remove trailing slash
        IF repositoryUrl.endsWith("/") THEN
            repositoryUrl = repositoryUrl.substring(0, repositoryUrl.length - 1)
        END IF

        // Add repository
        repositoryUrl = repositoryUrl + "/" + repository

        RETURN repositoryUrl
    END IF

    // Step 2: Fallback - try GITHUB_REPOSITORY_URL (if available)
    repositoryUrl = GetEnv("GITHUB_REPOSITORY_URL")
    IF repositoryUrl IS NOT NULL THEN
        RETURN repositoryUrl
    END IF

    // Step 3: No URL available
    LogWarning("Could not determine repository URL from environment")
    RETURN NULL
END FUNCTION
```

---

## Implementation Notes

### 1. Performance Considerations

**For Large Number of Secrets:**
- Streaming HTML generation (don't build entire string in memory)
- Limit maximum number of rows in table
- Consider pagination or truncation for very large results

```pseudo
FUNCTION GenerateSecretsTableWithLimit(
    secrets: Array<DetectedSecret>,
    config: SummaryConfig,
    maxRows: Integer
) -> String
    // Limit table to maxRows to prevent excessive HTML size

    actualSecrets = secrets
    wasTruncated = FALSE

    IF secrets.length > maxRows THEN
        actualSecrets = secrets.slice(0, maxRows)
        wasTruncated = TRUE
        LogWarning("Truncating table to " + maxRows + " rows (total: " + secrets.length + ")")
    END IF

    // Generate table normally
    summary = GenerateSecretsSummary(actualSecrets, config)

    // Add truncation notice if needed
    IF wasTruncated THEN
        summary += "\n\n**Note:** Showing first " + maxRows + " of " +
                   secrets.length + " detected secrets. " +
                   "See full SARIF report for complete results.\n"
    END IF

    RETURN summary
END FUNCTION
```

---

### 2. Rust-Specific Implementation Hints

**String Building:**
```rust
// Use format! macro for string building
let summary = format!("## {} Gitleaks detected secrets {}\n\n", "🛑", "🛑");

// Use String builder for performance
let mut html = String::with_capacity(1024);
html.push_str("<table>\n");
html.push_str("  <thead>\n");
// ... etc

// HTML escaping
use html_escape::encode_text;
let escaped = encode_text(&author);
```

**URL Building:**
```rust
// URL construction with proper escaping
use url::Url;

fn generate_secret_url(
    repo_url: &str,
    commit_sha: &str,
    file_path: &str,
    line: u32,
) -> Result<String, url::ParseError> {
    let base = Url::parse(repo_url)?;
    let path = format!("/blob/{}/{}", commit_sha, file_path);
    let mut url = base.join(&path)?;
    url.set_fragment(Some(&format!("L{}", line)));
    Ok(url.to_string())
}
```

---

### 3. Testing Considerations

**Test Cases to Implement:**

1. **Success Summary**
   - Input: Exit code 0, empty secrets
   - Expected: "## No leaks detected ✅"

2. **Error Summary**
   - Input: Exit code 1
   - Expected: "## ❌ Gitleaks exited with error. Exit code [1]"

3. **Secrets Summary - Single Secret**
   - Input: Exit code 2, one secret
   - Expected: Complete HTML table with one row

4. **Secrets Summary - Multiple Secrets**
   - Input: Exit code 2, multiple secrets
   - Expected: Complete HTML table with all rows

5. **HTML Escaping**
   - Input: Secret with HTML special chars in fields
   - Expected: All special chars properly escaped

6. **URL Generation**
   - Input: Valid repo URL, commit, file path
   - Expected: Correct GitHub URLs

7. **Missing Repository URL**
   - Input: NULL repository URL
   - Expected: Table without hyperlinks

8. **Summary Disabled**
   - Input: config.enabled = FALSE
   - Expected: wasGenerated = FALSE, no content

9. **Invalid Input Data**
   - Input: NULL secrets array with exit code 2
   - Expected: Error summary generated

10. **Special Characters in Paths**
    - Input: File paths with spaces, unicode
    - Expected: Properly URL-encoded paths

---

## Appendix: Output Format Examples

### Complete Success Summary Example

```markdown
## No leaks detected ✅
```

---

### Complete Secrets Summary Example

```markdown
## 🛑 Gitleaks detected secrets 🛑

<table>
  <thead>
    <tr>
      <th>Rule ID</th>
      <th>Commit</th>
      <th>Secret URL</th>
      <th>Start Line</th>
      <th>Author</th>
      <th>Date</th>
      <th>Email</th>
      <th>File</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>aws-access-token</td>
      <td><a href="https://github.com/owner/repo/commit/abc123def456789012345678901234567890">abc123d</a></td>
      <td><a href="https://github.com/owner/repo/blob/abc123def456789012345678901234567890/src/config/aws.js#L42">View</a></td>
      <td>42</td>
      <td>John Doe</td>
      <td>2025-10-15T14:30:00Z</td>
      <td>john@example.com</td>
      <td><a href="https://github.com/owner/repo/blob/abc123def456789012345678901234567890/src/config/aws.js">src/config/aws.js</a></td>
    </tr>
    <tr>
      <td>generic-api-key</td>
      <td><a href="https://github.com/owner/repo/commit/def789abc123456789012345678901234567890">def789a</a></td>
      <td><a href="https://github.com/owner/repo/blob/def789abc123456789012345678901234567890/config.yml#L23">View</a></td>
      <td>23</td>
      <td>Jane Smith</td>
      <td>2025-10-14T09:15:00Z</td>
      <td>jane@example.com</td>
      <td><a href="https://github.com/owner/repo/blob/def789abc123456789012345678901234567890/config.yml">config.yml</a></td>
    </tr>
  </tbody>
</table>
```

---

### Complete Error Summary Example

```markdown
## ❌ Gitleaks exited with error. Exit code [1]
```

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-16 | Job Summary Specialist | Initial pseudocode document |

---

**END OF JOB SUMMARY PSEUDOCODE**

**Status:** ✅ COMPLETE

**Next Steps:**
1. Review pseudocode algorithms
2. Implement in Rust with HTML generation
3. Create unit tests for all functions
4. Test with real gitleaks SARIF output
5. Verify rendering in GitHub Actions UI
