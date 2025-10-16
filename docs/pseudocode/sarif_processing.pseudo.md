# SARIF Processing Pseudocode

**Project:** SecretScout - Rust Port of gitleaks-action
**Component:** SARIF Parsing and Processing (FR-3)
**Phase:** Pseudocode
**Date:** October 16, 2025
**Version:** 1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Core Algorithms](#core-algorithms)
4. [Error Handling](#error-handling)
5. [Validation Functions](#validation-functions)
6. [Usage Examples](#usage-examples)

---

## Overview

This document provides detailed pseudocode algorithms for parsing and processing SARIF v2 output from gitleaks. The algorithms handle:

- SARIF file parsing and JSON validation
- Results extraction from nested structures
- Field extraction with null-safe navigation
- Fingerprint generation for .gitleaksignore
- Comprehensive error handling for malformed data

**Reference:** SPARC_SPECIFICATION.md Section 3.1, FR-3

---

## Data Structures

### 1. SARIFReport

```pseudo
STRUCTURE SARIFReport
    version: String                    // SARIF version (e.g., "2.1.0")
    runs: Array<SARIFRun>              // Array of tool runs
END STRUCTURE
```

### 2. SARIFRun

```pseudo
STRUCTURE SARIFRun
    tool: ToolInfo                     // Tool metadata (gitleaks)
    results: Array<SARIFResult>        // Array of detected secrets
    invocations: Array<Invocation>     // Optional execution metadata
END STRUCTURE
```

### 3. SARIFResult

```pseudo
STRUCTURE SARIFResult
    ruleId: String                     // Detection rule name (e.g., "aws-access-token")
    message: Message                   // Human-readable description
    locations: Array<Location>         // Where the secret was found
    partialFingerprints: Map<String, String>  // Custom gitleaks metadata
    level: String                      // Optional: "error", "warning", "note"
END STRUCTURE
```

### 4. Location

```pseudo
STRUCTURE Location
    physicalLocation: PhysicalLocation  // File and line information
END STRUCTURE
```

### 5. PhysicalLocation

```pseudo
STRUCTURE PhysicalLocation
    artifactLocation: ArtifactLocation  // File path
    region: Region                      // Line/column information
END STRUCTURE
```

### 6. ArtifactLocation

```pseudo
STRUCTURE ArtifactLocation
    uri: String                        // Relative file path (e.g., "src/config.js")
    uriBaseId: String                  // Optional base ID
END STRUCTURE
```

### 7. Region

```pseudo
STRUCTURE Region
    startLine: Integer                 // Line number where secret starts (1-indexed)
    startColumn: Integer               // Optional column number
    endLine: Integer                   // Optional end line
    endColumn: Integer                 // Optional end column
    snippet: Snippet                   // Optional code snippet
END STRUCTURE
```

### 8. PartialFingerprints

```pseudo
STRUCTURE PartialFingerprints
    commitSha: String                  // Git commit SHA (40 hex chars)
    author: String                     // Commit author name
    email: String                      // Commit author email
    date: String                       // Commit date (ISO 8601)
END STRUCTURE
```

### 9. DetectedSecret

```pseudo
// Extracted and processed result
STRUCTURE DetectedSecret
    ruleId: String                     // Detection rule ID
    filePath: String                   // Relative file path
    startLine: Integer                 // Line number
    commitSha: String                  // Commit containing secret
    author: String                     // Commit author
    email: String                      // Author email
    date: String                       // Commit date
    fingerprint: String                // Generated fingerprint
    message: String                    // Detection message
END STRUCTURE
```

### 10. ParseResult

```pseudo
STRUCTURE ParseResult
    success: Boolean                   // Whether parsing succeeded
    secrets: Array<DetectedSecret>     // Extracted secrets (if success)
    error: String                      // Error message (if failure)
    errorDetails: String               // Detailed error info (if failure)
END STRUCTURE
```

---

## Core Algorithms

### 1. ParseSARIFFile

**Purpose:** Parse SARIF JSON file and extract all detected secrets.

**Inputs:**
- `filePath`: String - Path to SARIF file

**Outputs:**
- `ParseResult` - Parsing results with secrets or error

**Algorithm:**

```pseudo
FUNCTION ParseSARIFFile(filePath: String) -> ParseResult
    // Initialize result
    result = new ParseResult()
    result.success = FALSE
    result.secrets = []
    result.error = NULL
    result.errorDetails = NULL

    // Step 1: Validate file exists
    IF NOT FileExists(filePath) THEN
        result.error = "SARIF file not found"
        result.errorDetails = "File path: " + filePath
        RETURN result
    END IF

    // Step 2: Read file contents
    TRY
        fileContent = ReadFileToString(filePath)
    CATCH IOException e
        result.error = "Failed to read SARIF file"
        result.errorDetails = e.message
        RETURN result
    END TRY

    // Step 3: Parse JSON
    sarifReport = NULL
    TRY
        sarifReport = ParseJSON(fileContent) AS SARIFReport
    CATCH JSONParseException e
        result.error = "Invalid JSON in SARIF file"
        result.errorDetails = e.message + " at position " + e.position
        RETURN result
    END TRY

    // Step 4: Validate SARIF structure
    validationResult = ValidateSARIFStructure(sarifReport)
    IF NOT validationResult.isValid THEN
        result.error = "Invalid SARIF structure"
        result.errorDetails = validationResult.errorMessage
        RETURN result
    END IF

    // Step 5: Extract results
    TRY
        secrets = ExtractResults(sarifReport)
        result.success = TRUE
        result.secrets = secrets
    CATCH Exception e
        result.error = "Failed to extract secrets from SARIF"
        result.errorDetails = e.message
        RETURN result
    END TRY

    RETURN result
END FUNCTION
```

---

### 2. ExtractResults

**Purpose:** Extract all detected secrets from SARIF report structure.

**Inputs:**
- `sarifReport`: SARIFReport - Parsed SARIF structure

**Outputs:**
- `Array<DetectedSecret>` - List of extracted secrets

**Algorithm:**

```pseudo
FUNCTION ExtractResults(sarifReport: SARIFReport) -> Array<DetectedSecret>
    // Initialize secrets array
    allSecrets = []

    // Step 1: Null-safe check for runs array
    IF sarifReport IS NULL OR sarifReport.runs IS NULL THEN
        RETURN allSecrets  // Empty array
    END IF

    // Step 2: Check if runs array is empty
    IF sarifReport.runs.length == 0 THEN
        RETURN allSecrets  // Empty array
    END IF

    // Step 3: Extract from first run (gitleaks uses runs[0])
    firstRun = sarifReport.runs[0]

    // Step 4: Null-safe check for results array
    IF firstRun.results IS NULL THEN
        RETURN allSecrets  // Empty array
    END IF

    // Step 5: Iterate through all results
    FOR EACH result IN firstRun.results DO
        // Extract individual secret
        secret = ExtractSecret(result)

        // Only add if extraction succeeded (non-null)
        IF secret IS NOT NULL THEN
            allSecrets.append(secret)
        END IF
    END FOR

    RETURN allSecrets
END FUNCTION
```

---

### 3. ExtractSecret

**Purpose:** Extract a single secret from a SARIF result object.

**Inputs:**
- `result`: SARIFResult - Single SARIF result

**Outputs:**
- `DetectedSecret` - Extracted secret or NULL if extraction fails

**Algorithm:**

```pseudo
FUNCTION ExtractSecret(result: SARIFResult) -> DetectedSecret
    // Initialize secret object
    secret = new DetectedSecret()

    // Step 1: Extract rule ID (REQUIRED)
    IF result.ruleId IS NULL OR result.ruleId == "" THEN
        LogWarning("SARIF result missing ruleId, skipping")
        RETURN NULL
    END IF
    secret.ruleId = result.ruleId

    // Step 2: Extract message (with fallback)
    IF result.message IS NOT NULL AND result.message.text IS NOT NULL THEN
        secret.message = result.message.text
    ELSE
        secret.message = "Secret detected: " + secret.ruleId
    END IF

    // Step 3: Extract location (REQUIRED)
    location = ExtractLocation(result)
    IF location IS NULL THEN
        LogWarning("SARIF result missing valid location, skipping")
        RETURN NULL
    END IF
    secret.filePath = location.filePath
    secret.startLine = location.startLine

    // Step 4: Extract partial fingerprints (commit metadata)
    fingerprints = ExtractFingerprints(result)
    IF fingerprints IS NOT NULL THEN
        secret.commitSha = fingerprints.commitSha
        secret.author = fingerprints.author
        secret.email = fingerprints.email
        secret.date = fingerprints.date
    ELSE
        // Use defaults if fingerprints missing
        secret.commitSha = "unknown"
        secret.author = "unknown"
        secret.email = "unknown"
        secret.date = "unknown"
    END IF

    // Step 5: Generate fingerprint string
    secret.fingerprint = GenerateFingerprintString(
        secret.commitSha,
        secret.filePath,
        secret.ruleId,
        secret.startLine
    )

    RETURN secret
END FUNCTION
```

---

### 4. ExtractLocation

**Purpose:** Extract file path and line number from SARIF location.

**Inputs:**
- `result`: SARIFResult - SARIF result containing locations

**Outputs:**
- `Object{filePath: String, startLine: Integer}` - Extracted location or NULL

**Algorithm:**

```pseudo
FUNCTION ExtractLocation(result: SARIFResult) -> Object
    // Step 1: Null-safe check for locations array
    IF result.locations IS NULL OR result.locations.length == 0 THEN
        RETURN NULL
    END IF

    // Step 2: Get first location (gitleaks uses locations[0])
    firstLocation = result.locations[0]
    IF firstLocation IS NULL THEN
        RETURN NULL
    END IF

    // Step 3: Navigate to physical location
    IF firstLocation.physicalLocation IS NULL THEN
        RETURN NULL
    END IF
    physicalLoc = firstLocation.physicalLocation

    // Step 4: Extract artifact location (file path)
    filePath = NULL
    IF physicalLoc.artifactLocation IS NOT NULL THEN
        IF physicalLoc.artifactLocation.uri IS NOT NULL THEN
            filePath = physicalLoc.artifactLocation.uri
        END IF
    END IF

    // File path is required
    IF filePath IS NULL OR filePath == "" THEN
        RETURN NULL
    END IF

    // Step 5: Extract region (line number)
    startLine = 1  // Default to line 1 if not specified
    IF physicalLoc.region IS NOT NULL THEN
        IF physicalLoc.region.startLine IS NOT NULL THEN
            startLine = physicalLoc.region.startLine
        END IF
    END IF

    // Step 6: Validate line number
    IF startLine < 1 THEN
        LogWarning("Invalid line number " + startLine + ", using 1")
        startLine = 1
    END IF

    // Return location object
    RETURN {
        filePath: filePath,
        startLine: startLine
    }
END FUNCTION
```

---

### 5. ExtractFingerprints

**Purpose:** Extract commit metadata from partialFingerprints.

**Inputs:**
- `result`: SARIFResult - SARIF result containing fingerprints

**Outputs:**
- `PartialFingerprints` - Extracted fingerprints or NULL

**Algorithm:**

```pseudo
FUNCTION ExtractFingerprints(result: SARIFResult) -> PartialFingerprints
    // Step 1: Null-safe check for partialFingerprints
    IF result.partialFingerprints IS NULL THEN
        RETURN NULL
    END IF

    fingerprints = result.partialFingerprints

    // Step 2: Extract individual fields with null-safe navigation
    commitSha = fingerprints.get("commitSha")
    author = fingerprints.get("author")
    email = fingerprints.get("email")
    date = fingerprints.get("date")

    // Step 3: Validate at least commitSha is present
    IF commitSha IS NULL OR commitSha == "" THEN
        LogWarning("partialFingerprints missing commitSha")
        RETURN NULL
    END IF

    // Step 4: Create fingerprints object with defaults
    result = new PartialFingerprints()
    result.commitSha = commitSha
    result.author = IF author IS NOT NULL THEN author ELSE "unknown"
    result.email = IF email IS NOT NULL THEN email ELSE "unknown"
    result.date = IF date IS NOT NULL THEN date ELSE "unknown"

    RETURN result
END FUNCTION
```

---

### 6. GenerateFingerprintString

**Purpose:** Generate fingerprint string for .gitleaksignore file.

**Inputs:**
- `commitSha`: String - Commit SHA
- `filePath`: String - File path
- `ruleId`: String - Rule ID
- `startLine`: Integer - Line number

**Outputs:**
- `String` - Fingerprint in format: {commit}:{file}:{rule}:{line}

**Algorithm:**

```pseudo
FUNCTION GenerateFingerprintString(
    commitSha: String,
    filePath: String,
    ruleId: String,
    startLine: Integer
) -> String
    // Step 1: Validate inputs (use safe defaults)
    safeCommit = IF commitSha IS NOT NULL THEN commitSha ELSE "unknown"
    safeFile = IF filePath IS NOT NULL THEN filePath ELSE "unknown"
    safeRule = IF ruleId IS NOT NULL THEN ruleId ELSE "unknown"
    safeLine = IF startLine >= 1 THEN startLine ELSE 1

    // Step 2: Normalize file path (ensure forward slashes)
    normalizedPath = safeFile.replace("\\", "/")

    // Step 3: Truncate commit SHA to first 7 characters (if valid)
    IF safeCommit.length >= 7 AND IsValidCommitSha(safeCommit) THEN
        shortCommit = safeCommit.substring(0, 7)
    ELSE
        shortCommit = safeCommit  // Use full string if invalid
    END IF

    // Step 4: Build fingerprint string
    fingerprint = shortCommit + ":" +
                  normalizedPath + ":" +
                  safeRule + ":" +
                  ToString(safeLine)

    RETURN fingerprint
END FUNCTION
```

**Example Output:**
```
abc123d:src/config.js:aws-access-token:42
```

---

### 7. ValidateSARIFStructure

**Purpose:** Validate SARIF report has required structure.

**Inputs:**
- `sarifReport`: SARIFReport - Parsed SARIF object

**Outputs:**
- `Object{isValid: Boolean, errorMessage: String}` - Validation result

**Algorithm:**

```pseudo
FUNCTION ValidateSARIFStructure(sarifReport: SARIFReport) -> Object
    errors = []

    // Step 1: Check root object exists
    IF sarifReport IS NULL THEN
        errors.append("SARIF report is null")
        RETURN {isValid: FALSE, errorMessage: JoinStrings(errors, "; ")}
    END IF

    // Step 2: Check SARIF version
    IF sarifReport.version IS NULL THEN
        errors.append("Missing 'version' field")
    ELSE IF NOT sarifReport.version.startsWith("2.") THEN
        errors.append("Unsupported SARIF version: " + sarifReport.version)
    END IF

    // Step 3: Check runs array exists
    IF sarifReport.runs IS NULL THEN
        errors.append("Missing 'runs' array")
        RETURN {isValid: FALSE, errorMessage: JoinStrings(errors, "; ")}
    END IF

    // Step 4: Check runs array is not empty
    IF sarifReport.runs.length == 0 THEN
        // This is valid (no runs = no results) but log for info
        LogInfo("SARIF report has empty runs array")
    END IF

    // Step 5: Validate first run structure (if exists)
    IF sarifReport.runs.length > 0 THEN
        firstRun = sarifReport.runs[0]

        IF firstRun IS NULL THEN
            errors.append("runs[0] is null")
        ELSE
            // Check results array exists (can be empty)
            IF firstRun.results IS NULL THEN
                errors.append("runs[0].results is null")
            END IF

            // Check tool info exists
            IF firstRun.tool IS NULL THEN
                LogWarning("runs[0].tool is null")
            END IF
        END IF
    END IF

    // Step 6: Return validation result
    IF errors.length > 0 THEN
        RETURN {
            isValid: FALSE,
            errorMessage: JoinStrings(errors, "; ")
        }
    ELSE
        RETURN {
            isValid: TRUE,
            errorMessage: NULL
        }
    END IF
END FUNCTION
```

---

### 8. IsValidCommitSha

**Purpose:** Validate a string is a valid Git commit SHA.

**Inputs:**
- `sha`: String - Commit SHA to validate

**Outputs:**
- `Boolean` - TRUE if valid, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION IsValidCommitSha(sha: String) -> Boolean
    // Step 1: Check null or empty
    IF sha IS NULL OR sha == "" THEN
        RETURN FALSE
    END IF

    // Step 2: Check length (full SHA is 40 chars, short is 7-40)
    IF sha.length < 7 OR sha.length > 40 THEN
        RETURN FALSE
    END IF

    // Step 3: Check all characters are hexadecimal
    FOR EACH char IN sha DO
        IF NOT IsHexDigit(char) THEN
            RETURN FALSE
        END IF
    END FOR

    RETURN TRUE
END FUNCTION
```

---

### 9. IsHexDigit

**Purpose:** Check if character is hexadecimal digit.

**Inputs:**
- `c`: Character - Character to check

**Outputs:**
- `Boolean` - TRUE if hex digit, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION IsHexDigit(c: Character) -> Boolean
    RETURN (c >= '0' AND c <= '9') OR
           (c >= 'a' AND c <= 'f') OR
           (c >= 'A' AND c <= 'F')
END FUNCTION
```

---

## Error Handling

### Error Categories

#### 1. File System Errors

```pseudo
ERROR HANDLING FileSystemErrors
    WHEN FileNotFound THEN
        LOG ERROR "SARIF file not found: " + filePath
        RETURN ParseResult with error = "SARIF file not found"

    WHEN PermissionDenied THEN
        LOG ERROR "Permission denied reading SARIF file: " + filePath
        RETURN ParseResult with error = "Permission denied"

    WHEN IOError THEN
        LOG ERROR "I/O error reading SARIF file: " + error.message
        RETURN ParseResult with error = "Failed to read SARIF file"
END ERROR HANDLING
```

#### 2. JSON Parsing Errors

```pseudo
ERROR HANDLING JSONParsingErrors
    WHEN InvalidJSON THEN
        LOG ERROR "Invalid JSON in SARIF file"
        LOG DEBUG "Parse error: " + error.message + " at position " + error.position
        RETURN ParseResult with error = "Invalid JSON in SARIF file"

    WHEN UnexpectedEndOfInput THEN
        LOG ERROR "Incomplete JSON in SARIF file"
        RETURN ParseResult with error = "Incomplete JSON"

    WHEN TypeMismatch THEN
        LOG ERROR "JSON type mismatch in SARIF structure"
        LOG DEBUG "Expected " + expected + " but got " + actual
        RETURN ParseResult with error = "Invalid SARIF structure"
END ERROR HANDLING
```

#### 3. Structure Validation Errors

```pseudo
ERROR HANDLING StructureValidationErrors
    WHEN MissingRequiredField THEN
        LOG ERROR "SARIF missing required field: " + fieldName
        RETURN ParseResult with error = "Invalid SARIF structure"

    WHEN InvalidFieldType THEN
        LOG ERROR "SARIF field has invalid type: " + fieldName
        LOG DEBUG "Expected " + expectedType + " but got " + actualType
        RETURN ParseResult with error = "Invalid SARIF structure"

    WHEN UnsupportedVersion THEN
        LOG ERROR "Unsupported SARIF version: " + version
        LOG INFO "Supported versions: 2.x"
        RETURN ParseResult with error = "Unsupported SARIF version"
END ERROR HANDLING
```

#### 4. Data Extraction Errors

```pseudo
ERROR HANDLING DataExtractionErrors
    WHEN MissingLocation THEN
        LOG WARNING "SARIF result missing location, skipping"
        CONTINUE to next result

    WHEN MissingRuleId THEN
        LOG WARNING "SARIF result missing ruleId, skipping"
        CONTINUE to next result

    WHEN InvalidLineNumber THEN
        LOG WARNING "Invalid line number: " + lineNumber + ", using default"
        USE default value (1)

    WHEN MissingFingerprints THEN
        LOG WARNING "SARIF result missing partialFingerprints"
        USE default values ("unknown")
END ERROR HANDLING
```

---

### Null-Safe Navigation Pattern

**Pattern for accessing nested optional fields:**

```pseudo
FUNCTION SafeGet(object: Object, path: String, default: Any) -> Any
    // Split path into parts (e.g., "runs[0].results" -> ["runs", "0", "results"])
    parts = SplitPath(path)
    current = object

    FOR EACH part IN parts DO
        IF current IS NULL THEN
            RETURN default
        END IF

        IF IsArrayIndex(part) THEN
            index = ParseInteger(part)
            IF NOT IsArray(current) OR index >= current.length THEN
                RETURN default
            END IF
            current = current[index]
        ELSE
            IF NOT HasProperty(current, part) THEN
                RETURN default
            END IF
            current = current[part]
        END IF
    END FOR

    RETURN IF current IS NOT NULL THEN current ELSE default
END FUNCTION
```

**Usage Example:**

```pseudo
// Safe navigation through nested structure
commitSha = SafeGet(result, "partialFingerprints.commitSha", "unknown")
filePath = SafeGet(result, "locations[0].physicalLocation.artifactLocation.uri", "unknown")
startLine = SafeGet(result, "locations[0].physicalLocation.region.startLine", 1)
```

---

## Validation Functions

### 1. ValidateFilePath

**Purpose:** Validate extracted file path is valid.

```pseudo
FUNCTION ValidateFilePath(filePath: String) -> Boolean
    // Step 1: Not null or empty
    IF filePath IS NULL OR filePath == "" THEN
        RETURN FALSE
    END IF

    // Step 2: Not absolute path (should be relative)
    IF filePath.startsWith("/") OR filePath.contains(":\\") THEN
        LogWarning("File path appears to be absolute: " + filePath)
        // Allow but warn
    END IF

    // Step 3: No path traversal attempts
    IF filePath.contains("..") THEN
        LogWarning("File path contains '..': " + filePath)
        RETURN FALSE
    END IF

    RETURN TRUE
END FUNCTION
```

---

### 2. ValidateLineNumber

**Purpose:** Validate line number is in valid range.

```pseudo
FUNCTION ValidateLineNumber(lineNumber: Integer) -> Integer
    // Line numbers are 1-indexed
    IF lineNumber < 1 THEN
        LogWarning("Invalid line number: " + lineNumber + ", using 1")
        RETURN 1
    END IF

    // Reasonable upper limit (1 million lines)
    IF lineNumber > 1000000 THEN
        LogWarning("Suspiciously large line number: " + lineNumber)
        // Allow but warn
    END IF

    RETURN lineNumber
END FUNCTION
```

---

### 3. ValidateRuleId

**Purpose:** Validate rule ID is valid.

```pseudo
FUNCTION ValidateRuleId(ruleId: String) -> Boolean
    // Step 1: Not null or empty
    IF ruleId IS NULL OR ruleId == "" THEN
        RETURN FALSE
    END IF

    // Step 2: Reasonable length
    IF ruleId.length > 200 THEN
        LogWarning("Unusually long rule ID: " + ruleId)
        RETURN FALSE
    END IF

    // Step 3: Only contains safe characters (alphanumeric, dash, underscore)
    FOR EACH char IN ruleId DO
        IF NOT (IsAlphanumeric(char) OR char == '-' OR char == '_') THEN
            LogWarning("Rule ID contains invalid character: " + char)
            RETURN FALSE
        END IF
    END FOR

    RETURN TRUE
END FUNCTION
```

---

## Usage Examples

### Example 1: Basic SARIF Parsing

```pseudo
FUNCTION Main()
    // Parse SARIF file
    result = ParseSARIFFile("/workspace/results.sarif")

    // Check if parsing succeeded
    IF NOT result.success THEN
        LogError("Failed to parse SARIF: " + result.error)
        LogDebug("Details: " + result.errorDetails)
        Exit(1)
    END IF

    // Process secrets
    IF result.secrets.length == 0 THEN
        LogInfo("No secrets detected")
        Exit(0)
    ELSE
        LogInfo("Found " + result.secrets.length + " secret(s)")

        FOR EACH secret IN result.secrets DO
            LogInfo("Secret detected:")
            LogInfo("  Rule: " + secret.ruleId)
            LogInfo("  File: " + secret.filePath + ":" + secret.startLine)
            LogInfo("  Commit: " + secret.commitSha)
            LogInfo("  Fingerprint: " + secret.fingerprint)
        END FOR

        Exit(2)  // Secrets found
    END IF
END FUNCTION
```

---

### Example 2: SARIF Parsing with Error Recovery

```pseudo
FUNCTION ParseWithRecovery(filePath: String) -> Array<DetectedSecret>
    // Try to parse
    result = ParseSARIFFile(filePath)

    IF result.success THEN
        RETURN result.secrets
    END IF

    // Check if it's a structure issue vs file issue
    IF result.error.contains("not found") THEN
        LogError("SARIF file not found - gitleaks may not have run")
        RETURN []
    END IF

    IF result.error.contains("Invalid JSON") THEN
        LogError("SARIF file is corrupted - attempting to regenerate")
        // Could trigger re-run of gitleaks here
        RETURN []
    END IF

    IF result.error.contains("Invalid SARIF structure") THEN
        LogWarning("SARIF structure unexpected, trying lenient parse")
        // Could implement lenient parsing here
        RETURN []
    END IF

    // Unknown error
    LogError("Unexpected error: " + result.error)
    RETURN []
END FUNCTION
```

---

### Example 3: Filtering Results

```pseudo
FUNCTION FilterSecretsByRule(
    secrets: Array<DetectedSecret>,
    allowedRules: Array<String>
) -> Array<DetectedSecret>
    filtered = []

    FOR EACH secret IN secrets DO
        IF allowedRules.contains(secret.ruleId) THEN
            filtered.append(secret)
        ELSE
            LogDebug("Filtering out secret with rule: " + secret.ruleId)
        END IF
    END FOR

    RETURN filtered
END FUNCTION
```

---

### Example 4: Generating Summary Statistics

```pseudo
FUNCTION GenerateSummary(secrets: Array<DetectedSecret>) -> Object
    summary = {
        totalSecrets: secrets.length,
        secretsByRule: {},
        secretsByFile: {},
        secretsByAuthor: {},
        uniqueCommits: new Set()
    }

    FOR EACH secret IN secrets DO
        // Count by rule
        IF NOT summary.secretsByRule.hasKey(secret.ruleId) THEN
            summary.secretsByRule[secret.ruleId] = 0
        END IF
        summary.secretsByRule[secret.ruleId] += 1

        // Count by file
        IF NOT summary.secretsByFile.hasKey(secret.filePath) THEN
            summary.secretsByFile[secret.filePath] = 0
        END IF
        summary.secretsByFile[secret.filePath] += 1

        // Count by author
        IF NOT summary.secretsByAuthor.hasKey(secret.author) THEN
            summary.secretsByAuthor[secret.author] = 0
        END IF
        summary.secretsByAuthor[secret.author] += 1

        // Track unique commits
        summary.uniqueCommits.add(secret.commitSha)
    END FOR

    RETURN summary
END FUNCTION
```

---

### Example 5: Exporting Fingerprints for .gitleaksignore

```pseudo
FUNCTION ExportFingerprints(
    secrets: Array<DetectedSecret>,
    outputPath: String
) -> Boolean
    TRY
        // Open output file
        file = OpenFile(outputPath, WriteMode)

        // Write header
        file.writeLine("# Gitleaks Ignore File")
        file.writeLine("# Generated: " + CurrentTimestamp())
        file.writeLine("# Format: {commit}:{file}:{rule}:{line}")
        file.writeLine("")

        // Write fingerprints
        FOR EACH secret IN secrets DO
            file.writeLine(secret.fingerprint)
        END FOR

        // Close file
        file.close()

        LogInfo("Exported " + secrets.length + " fingerprints to " + outputPath)
        RETURN TRUE

    CATCH IOException e
        LogError("Failed to write fingerprints: " + e.message)
        RETURN FALSE
    END TRY
END FUNCTION
```

---

## Implementation Notes

### 1. Performance Considerations

**For Large SARIF Files:**
- Stream parsing for files > 10MB
- Lazy evaluation of results array
- Early termination if max results reached
- Memory-efficient string handling

```pseudo
// Streaming approach for large files
FUNCTION ParseSARIFFileStreaming(filePath: String, maxResults: Integer) -> ParseResult
    result = new ParseResult()
    result.secrets = []

    // Use streaming JSON parser
    parser = CreateStreamingJSONParser(filePath)

    // Navigate to results array
    parser.navigateToPath("runs[0].results")

    // Stream each result
    WHILE parser.hasNext() AND result.secrets.length < maxResults DO
        resultObject = parser.readNext()
        secret = ExtractSecret(resultObject)

        IF secret IS NOT NULL THEN
            result.secrets.append(secret)
        END IF
    END WHILE

    result.success = TRUE
    RETURN result
END FUNCTION
```

---

### 2. Rust-Specific Implementation Hints

**Using serde_json for parsing:**
```rust
// Rust implementation notes (not strict pseudocode)
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Use Option<T> for all optional fields
#[derive(Deserialize)]
struct SARIFReport {
    version: String,
    runs: Option<Vec<SARIFRun>>,
}

// Use Result<T, E> for error handling
fn parse_sarif_file(path: &str) -> Result<ParseResult, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let report: SARIFReport = serde_json::from_str(&content)?;
    Ok(extract_results(report))
}

// Use pattern matching for null-safe navigation
fn extract_location(result: &SARIFResult) -> Option<Location> {
    result.locations
        .as_ref()?
        .first()?
        .physical_location
        .as_ref()?
        .artifact_location
        .as_ref()
        .map(|a| Location {
            file_path: a.uri.clone(),
            start_line: result.locations.as_ref()
                .and_then(|l| l.first())
                .and_then(|l| l.physical_location.as_ref())
                .and_then(|p| p.region.as_ref())
                .and_then(|r| r.start_line)
                .unwrap_or(1),
        })
}
```

---

### 3. Testing Considerations

**Test Cases to Implement:**

1. **Valid SARIF with secrets**
   - Input: Complete SARIF with all fields
   - Expected: All secrets extracted

2. **Valid SARIF with no secrets**
   - Input: SARIF with empty results array
   - Expected: Empty secrets array, success = true

3. **Missing optional fields**
   - Input: SARIF with missing partialFingerprints
   - Expected: Secrets extracted with default values

4. **Malformed JSON**
   - Input: Invalid JSON syntax
   - Expected: Parse error with position

5. **Invalid SARIF structure**
   - Input: JSON without required fields
   - Expected: Validation error

6. **File not found**
   - Input: Non-existent file path
   - Expected: File not found error

7. **Large SARIF file**
   - Input: SARIF with 10,000+ results
   - Expected: All secrets extracted, reasonable performance

8. **Edge cases**
   - Empty file
   - File with only whitespace
   - SARIF version 2.1.0 vs 2.0.0
   - Line number = 0 (invalid)
   - Negative line number
   - Extremely long file paths

---

## Appendix: SARIF Example

### Complete SARIF Example from Gitleaks

```json
{
  "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
  "version": "2.1.0",
  "runs": [
    {
      "tool": {
        "driver": {
          "name": "Gitleaks",
          "semanticVersion": "8.24.3",
          "informationUri": "https://github.com/gitleaks/gitleaks"
        }
      },
      "results": [
        {
          "ruleId": "aws-access-token",
          "ruleIndex": 0,
          "message": {
            "text": "Detected AWS Access Token"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "src/config/aws.js"
                },
                "region": {
                  "startLine": 42,
                  "startColumn": 15,
                  "endLine": 42,
                  "endColumn": 55,
                  "snippet": {
                    "text": "const key = 'AKIAIOSFODNN7EXAMPLE';"
                  }
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "abc123def456789",
            "author": "John Doe",
            "email": "john@example.com",
            "date": "2025-10-15T14:30:00Z"
          }
        },
        {
          "ruleId": "generic-api-key",
          "ruleIndex": 1,
          "message": {
            "text": "Detected Generic API Key"
          },
          "locations": [
            {
              "physicalLocation": {
                "artifactLocation": {
                  "uri": "config.yml"
                },
                "region": {
                  "startLine": 23,
                  "startColumn": 8,
                  "endLine": 23,
                  "endColumn": 48
                }
              }
            }
          ],
          "partialFingerprints": {
            "commitSha": "def789abc123456",
            "author": "Jane Smith",
            "email": "jane@example.com",
            "date": "2025-10-14T09:15:00Z"
          }
        }
      ]
    }
  ]
}
```

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-16 | SARIF Processing Specialist | Initial pseudocode document |

---

**END OF SARIF PROCESSING PSEUDOCODE**

**Status:** ✅ COMPLETE

**Next Steps:**
1. Review pseudocode algorithms
2. Implement in Rust with serde_json
3. Create unit tests for each function
4. Test with real gitleaks SARIF output
