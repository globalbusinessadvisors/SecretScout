# PR Comment System Pseudocode

**Project:** SecretScout - Rust Port of gitleaks-action
**Component:** Pull Request Review Comment System (FR-4)
**Phase:** Pseudocode
**Date:** October 16, 2025
**Version:** 1.0

---

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Core Algorithms](#core-algorithms)
4. [Comment Generation](#comment-generation)
5. [Comment Placement](#comment-placement)
6. [Deduplication Logic](#deduplication-logic)
7. [Error Handling](#error-handling)
8. [User Mentions](#user-mentions)
9. [Usage Examples](#usage-examples)

---

## Overview

This document provides detailed pseudocode algorithms for the PR review comment system. The system handles:

- Comment content generation with emoji, rule ID, commit SHA, fingerprint
- Comment placement determination (file path, line number, commit, side)
- Deduplication logic to prevent spam on re-runs
- Error handling for large diff errors (non-fatal)
- User mention parsing and appending

**Reference:** SPARC_SPECIFICATION.md Section 3.1, FR-4

**Key Principles:**
- Comments are **non-fatal** - failures are logged as warnings, execution continues
- Secrets appear in summary/artifacts even if commenting fails
- Deduplication prevents spam on workflow re-runs
- Large diff errors are expected and handled gracefully

---

## Data Structures

### 1. PRComment

```pseudo
STRUCTURE PRComment
    body: String                       // Markdown comment content
    path: String                       // File path (relative to repo root)
    line: Integer                      // Line number in diff
    commit_id: String                  // Commit SHA
    side: String                       // "RIGHT" (new code) or "LEFT" (old code)
END STRUCTURE
```

### 2. CommentMetadata

```pseudo
STRUCTURE CommentMetadata
    ruleId: String                     // Detection rule ID
    commitSha: String                  // Commit containing secret (first 7 chars)
    fingerprint: String                // For .gitleaksignore
    filePath: String                   // File path
    startLine: Integer                 // Line number
    mentions: Array<String>            // User mentions (e.g., ["@user1", "@user2"])
END STRUCTURE
```

### 3. ExistingComment

```pseudo
STRUCTURE ExistingComment
    id: Integer                        // GitHub comment ID
    body: String                       // Comment text
    path: String                       // File path
    original_line: Integer             // Line number when comment was created
    line: Integer                      // Current line number (may differ due to edits)
    commit_id: String                  // Commit SHA
    created_at: String                 // ISO 8601 timestamp
END STRUCTURE
```

### 4. CommentResult

```pseudo
STRUCTURE CommentResult
    posted: Integer                    // Number of comments successfully posted
    skipped: Integer                   // Number skipped (duplicates)
    failed: Integer                    // Number failed (errors)
    errors: Array<String>              // Error messages
END STRUCTURE
```

### 5. CommentRequest

```pseudo
STRUCTURE CommentRequest
    owner: String                      // Repository owner
    repo: String                       // Repository name
    pr_number: Integer                 // Pull request number
    secrets: Array<DetectedSecret>     // Secrets from SARIF parsing
    github_token: String               // GitHub API authentication token
    notify_users: Array<String>        // Optional user mentions
END STRUCTURE
```

---

## Core Algorithms

### 1. PostReviewComments

**Purpose:** Main entry point for posting PR review comments on detected secrets.

**Inputs:**
- `request`: CommentRequest - Comment request with PR details and secrets

**Outputs:**
- `CommentResult` - Summary of posted, skipped, and failed comments

**Algorithm:**

```pseudo
FUNCTION PostReviewComments(request: CommentRequest) -> CommentResult
    // Initialize result
    result = new CommentResult()
    result.posted = 0
    result.skipped = 0
    result.failed = 0
    result.errors = []

    // Step 1: Validate inputs
    IF request.secrets IS NULL OR request.secrets.length == 0 THEN
        LogInfo("No secrets to comment on")
        RETURN result
    END IF

    IF request.github_token IS NULL OR request.github_token == "" THEN
        LogWarning("GITHUB_TOKEN not provided, skipping PR comments")
        RETURN result
    END IF

    // Step 2: Fetch existing comments for deduplication
    existingComments = FetchExistingComments(
        request.owner,
        request.repo,
        request.pr_number,
        request.github_token
    )

    IF existingComments IS NULL THEN
        LogWarning("Failed to fetch existing comments, proceeding without deduplication")
        existingComments = []
    END IF

    // Step 3: Build deduplication map
    commentMap = BuildDeduplicationMap(existingComments)

    // Step 4: Process each secret
    FOR EACH secret IN request.secrets DO
        // Generate comment metadata
        metadata = new CommentMetadata()
        metadata.ruleId = secret.ruleId
        metadata.commitSha = TruncateCommitSha(secret.commitSha)
        metadata.fingerprint = secret.fingerprint
        metadata.filePath = secret.filePath
        metadata.startLine = secret.startLine
        metadata.mentions = request.notify_users

        // Generate comment body
        commentBody = GenerateCommentBody(metadata)

        // Determine placement
        placement = DetermineCommentPlacement(secret)

        // Check for duplicate
        IF IsDuplicateComment(commentMap, placement.path, placement.line, commentBody) THEN
            LogInfo("Skipping duplicate comment for " + placement.path + ":" + placement.line)
            result.skipped += 1
            CONTINUE to next secret
        END IF

        // Create comment object
        comment = new PRComment()
        comment.body = commentBody
        comment.path = placement.path
        comment.line = placement.line
        comment.commit_id = placement.commit_id
        comment.side = placement.side

        // Post comment (non-fatal operation)
        TRY
            success = PostComment(
                request.owner,
                request.repo,
                request.pr_number,
                comment,
                request.github_token
            )

            IF success THEN
                result.posted += 1
                LogInfo("Posted comment on " + comment.path + ":" + comment.line)
            ELSE
                result.failed += 1
                result.errors.append("Failed to post comment on " + comment.path)
            END IF

        CATCH Exception e
            // Non-fatal: Log and continue
            result.failed += 1
            errorMsg = "Error posting comment on " + comment.path + ": " + e.message
            result.errors.append(errorMsg)
            LogWarning(errorMsg)
        END TRY
    END FOR

    // Step 5: Log summary
    LogInfo("Comment posting complete:")
    LogInfo("  Posted: " + result.posted)
    LogInfo("  Skipped (duplicates): " + result.skipped)
    LogInfo("  Failed: " + result.failed)

    IF result.failed > 0 THEN
        LogWarning("Some comments failed to post. Secrets still reported in summary/artifacts.")
    END IF

    RETURN result
END FUNCTION
```

---

### 2. GenerateCommentBody

**Purpose:** Generate the markdown comment body with all required information.

**Inputs:**
- `metadata`: CommentMetadata - Comment metadata (rule ID, commit, fingerprint, etc.)

**Outputs:**
- `String` - Formatted markdown comment body

**Algorithm:**

```pseudo
FUNCTION GenerateCommentBody(metadata: CommentMetadata) -> String
    // Step 1: Initialize comment with emoji and header
    body = "🛑 **Gitleaks** has detected a secret with rule-id `" + metadata.ruleId + "`"
    body += " in commit " + metadata.commitSha + ".\n\n"

    // Step 2: Add guidance for true positives
    body += "**If this secret is a _true_ positive:**\n"
    body += "- Please rotate the secret ASAP\n"
    body += "- Update all systems using this secret\n"
    body += "- Review access logs for unauthorized usage\n\n"

    // Step 3: Add guidance for false positives
    body += "**If this secret is a _false_ positive:**\n"
    body += "You can add the fingerprint below to your `.gitleaksignore` file "
    body += "and commit the change to this branch:\n\n"

    // Step 4: Add fingerprint code block
    body += "```bash\n"
    body += "echo '" + metadata.fingerprint + "' >> .gitleaksignore\n"
    body += "```\n"

    // Step 5: Add user mentions if provided
    IF metadata.mentions IS NOT NULL AND metadata.mentions.length > 0 THEN
        body += "\n\ncc " + JoinStrings(metadata.mentions, ", ")
    END IF

    RETURN body
END FUNCTION
```

**Example Output:**

```markdown
🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123d.

**If this secret is a _true_ positive:**
- Please rotate the secret ASAP
- Update all systems using this secret
- Review access logs for unauthorized usage

**If this secret is a _false_ positive:**
You can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch:

```bash
echo 'abc123d:src/config.js:aws-access-token:42' >> .gitleaksignore
```

cc @user1, @user2
```

---

### 3. DetermineCommentPlacement

**Purpose:** Determine where to place the comment in the PR diff.

**Inputs:**
- `secret`: DetectedSecret - Secret from SARIF parsing

**Outputs:**
- `Object{path: String, line: Integer, commit_id: String, side: String}` - Placement info

**Algorithm:**

```pseudo
FUNCTION DetermineCommentPlacement(secret: DetectedSecret) -> Object
    // Step 1: Extract file path (relative to repo root)
    path = secret.filePath

    // Step 2: Normalize file path
    // Remove leading "./" if present
    IF path.startsWith("./") THEN
        path = path.substring(2)
    END IF

    // Ensure forward slashes (not backslashes)
    path = path.replace("\\", "/")

    // Step 3: Extract line number
    line = secret.startLine

    // Validate line number
    IF line < 1 THEN
        LogWarning("Invalid line number " + line + ", using 1")
        line = 1
    END IF

    // Step 4: Extract commit SHA (full SHA, not truncated)
    commit_id = secret.commitSha

    // Validate commit SHA
    IF NOT IsValidCommitSha(commit_id) THEN
        LogWarning("Invalid commit SHA: " + commit_id)
        // Use it anyway - GitHub API will reject if truly invalid
    END IF

    // Step 5: Determine side (always "RIGHT" for new code)
    // "RIGHT" = new version of file (PR branch)
    // "LEFT" = old version of file (base branch)
    // Gitleaks detects secrets in new commits, so always RIGHT
    side = "RIGHT"

    // Step 6: Return placement object
    RETURN {
        path: path,
        line: line,
        commit_id: commit_id,
        side: side
    }
END FUNCTION
```

---

### 4. FetchExistingComments

**Purpose:** Fetch all existing PR review comments for deduplication.

**Inputs:**
- `owner`: String - Repository owner
- `repo`: String - Repository name
- `pr_number`: Integer - Pull request number
- `github_token`: String - GitHub API token

**Outputs:**
- `Array<ExistingComment>` - Existing comments, or NULL on error

**Algorithm:**

```pseudo
FUNCTION FetchExistingComments(
    owner: String,
    repo: String,
    pr_number: Integer,
    github_token: String
) -> Array<ExistingComment>
    // Step 1: Build API URL
    // Endpoint: GET /repos/{owner}/{repo}/pulls/{pull_number}/comments
    api_url = "https://api.github.com/repos/" + owner + "/" + repo
    api_url += "/pulls/" + ToString(pr_number) + "/comments"

    // Step 2: Prepare request headers
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": "Bearer " + github_token,
        "X-GitHub-Api-Version": "2022-11-28"
    }

    // Step 3: Fetch comments with pagination
    allComments = []
    page = 1
    per_page = 100  // GitHub API max per page

    WHILE TRUE DO
        // Build URL with pagination
        page_url = api_url + "?page=" + ToString(page) + "&per_page=" + ToString(per_page)

        // Make API request
        TRY
            response = HttpGet(page_url, headers)

            // Check response status
            IF response.status_code != 200 THEN
                LogWarning("Failed to fetch comments (page " + page + "): HTTP " + response.status_code)

                // Return what we have so far (may be empty)
                IF page == 1 THEN
                    RETURN NULL  // First page failed, return NULL
                ELSE
                    RETURN allComments  // Later page failed, return partial results
                END IF
            END IF

            // Parse response body
            comments = ParseJSON(response.body) AS Array

            // No more comments
            IF comments.length == 0 THEN
                BREAK
            END IF

            // Add comments to collection
            FOR EACH comment IN comments DO
                existingComment = new ExistingComment()
                existingComment.id = comment.id
                existingComment.body = comment.body
                existingComment.path = comment.path
                existingComment.original_line = comment.original_line
                existingComment.line = comment.line
                existingComment.commit_id = comment.commit_id
                existingComment.created_at = comment.created_at

                allComments.append(existingComment)
            END FOR

            // Check if there are more pages
            IF comments.length < per_page THEN
                BREAK  // Last page
            END IF

            // Move to next page
            page += 1

            // Safety limit to prevent infinite loop
            IF page > 100 THEN
                LogWarning("Reached pagination limit (100 pages)")
                BREAK
            END IF

        CATCH Exception e
            LogWarning("Error fetching comments (page " + page + "): " + e.message)

            IF page == 1 THEN
                RETURN NULL  // First page failed
            ELSE
                RETURN allComments  // Return partial results
            END IF
        END TRY
    END WHILE

    LogInfo("Fetched " + allComments.length + " existing PR comments")
    RETURN allComments
END FUNCTION
```

---

### 5. BuildDeduplicationMap

**Purpose:** Build a map for fast duplicate comment detection.

**Inputs:**
- `existingComments`: Array<ExistingComment> - Existing PR comments

**Outputs:**
- `Map<String, Boolean>` - Map of comment keys to existence flag

**Algorithm:**

```pseudo
FUNCTION BuildDeduplicationMap(
    existingComments: Array<ExistingComment>
) -> Map<String, Boolean>
    // Initialize map
    commentMap = new Map<String, Boolean>()

    // Handle null input
    IF existingComments IS NULL THEN
        RETURN commentMap  // Empty map
    END IF

    // Step 1: Build deduplication keys for each comment
    FOR EACH comment IN existingComments DO
        // Create deduplication key
        // Format: "path:line:body"
        // Use original_line (line when comment was created)
        // because line may change if file is edited after comment
        key = GenerateDeduplicationKey(
            comment.path,
            comment.original_line,
            comment.body
        )

        // Add to map
        commentMap[key] = TRUE
    END FOR

    LogDebug("Built deduplication map with " + commentMap.size + " entries")
    RETURN commentMap
END FUNCTION
```

---

### 6. GenerateDeduplicationKey

**Purpose:** Generate a unique key for comment deduplication.

**Inputs:**
- `path`: String - File path
- `line`: Integer - Line number
- `body`: String - Comment body

**Outputs:**
- `String` - Deduplication key

**Algorithm:**

```pseudo
FUNCTION GenerateDeduplicationKey(
    path: String,
    line: Integer,
    body: String
) -> String
    // Step 1: Normalize path
    normalizedPath = path.replace("\\", "/")

    // Remove leading "./" if present
    IF normalizedPath.startsWith("./") THEN
        normalizedPath = normalizedPath.substring(2)
    END IF

    // Step 2: Normalize body (trim whitespace)
    normalizedBody = body.trim()

    // Step 3: Create key
    // Format: "path:line:body"
    key = normalizedPath + ":" + ToString(line) + ":" + normalizedBody

    RETURN key
END FUNCTION
```

---

### 7. IsDuplicateComment

**Purpose:** Check if a comment is a duplicate of an existing comment.

**Inputs:**
- `commentMap`: Map<String, Boolean> - Deduplication map
- `path`: String - File path
- `line`: Integer - Line number
- `body`: String - Comment body

**Outputs:**
- `Boolean` - TRUE if duplicate, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION IsDuplicateComment(
    commentMap: Map<String, Boolean>,
    path: String,
    line: Integer,
    body: String
) -> Boolean
    // Step 1: Generate deduplication key
    key = GenerateDeduplicationKey(path, line, body)

    // Step 2: Check if key exists in map
    IF commentMap.hasKey(key) THEN
        RETURN TRUE  // Duplicate found
    END IF

    RETURN FALSE  // Not a duplicate
END FUNCTION
```

---

### 8. PostComment

**Purpose:** Post a single review comment to GitHub PR.

**Inputs:**
- `owner`: String - Repository owner
- `repo`: String - Repository name
- `pr_number`: Integer - Pull request number
- `comment`: PRComment - Comment to post
- `github_token`: String - GitHub API token

**Outputs:**
- `Boolean` - TRUE if posted successfully, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION PostComment(
    owner: String,
    repo: String,
    pr_number: Integer,
    comment: PRComment,
    github_token: String
) -> Boolean
    // Step 1: Build API URL
    // Endpoint: POST /repos/{owner}/{repo}/pulls/{pull_number}/comments
    api_url = "https://api.github.com/repos/" + owner + "/" + repo
    api_url += "/pulls/" + ToString(pr_number) + "/comments"

    // Step 2: Prepare request headers
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": "Bearer " + github_token,
        "X-GitHub-Api-Version": "2022-11-28",
        "Content-Type": "application/json"
    }

    // Step 3: Prepare request body
    body = {
        "body": comment.body,
        "commit_id": comment.commit_id,
        "path": comment.path,
        "line": comment.line,
        "side": comment.side
    }

    // Step 4: Make API request
    TRY
        response = HttpPost(api_url, headers, body)

        // Step 5: Check response status
        IF response.status_code == 201 THEN
            // Success - comment created
            LogDebug("Comment posted successfully")
            RETURN TRUE

        ELSE IF response.status_code == 422 THEN
            // Unprocessable Entity - common for large diffs
            errorMsg = "Failed to post comment: diff too large or line not in diff"

            // Try to extract error details from response
            TRY
                errorData = ParseJSON(response.body)
                IF errorData.message IS NOT NULL THEN
                    errorMsg += " - " + errorData.message
                END IF
            CATCH
                // Ignore parse error
            END TRY

            LogWarning(errorMsg)
            RETURN FALSE

        ELSE IF response.status_code == 403 THEN
            // Forbidden - permission issue or rate limit
            LogWarning("Failed to post comment: permission denied or rate limited (HTTP 403)")
            RETURN FALSE

        ELSE IF response.status_code == 404 THEN
            // Not found - PR may have been deleted or merged
            LogWarning("Failed to post comment: PR not found (HTTP 404)")
            RETURN FALSE

        ELSE
            // Other error
            LogWarning("Failed to post comment: HTTP " + response.status_code)
            RETURN FALSE
        END IF

    CATCH Exception e
        // Network or other error
        LogWarning("Error posting comment: " + e.message)
        RETURN FALSE
    END TRY
END FUNCTION
```

---

### 9. HandleCommentError

**Purpose:** Handle comment posting errors gracefully (non-fatal).

**Inputs:**
- `error`: Exception - The error that occurred
- `context`: String - Context information (file path, line, etc.)

**Outputs:**
- None (logs warning and continues)

**Algorithm:**

```pseudo
FUNCTION HandleCommentError(error: Exception, context: String)
    // Step 1: Log warning (not error - this is non-fatal)
    LogWarning("Failed to post PR comment for " + context)

    // Step 2: Log error details for debugging
    LogDebug("Error details: " + error.message)

    // Step 3: Check if it's a known error type
    IF error.message.contains("diff") OR error.message.contains("422") THEN
        LogInfo("This is likely due to a large diff. Secret will still appear in summary/artifacts.")

    ELSE IF error.message.contains("rate limit") OR error.message.contains("403") THEN
        LogInfo("This may be due to GitHub API rate limiting. Try again later.")

    ELSE IF error.message.contains("404") THEN
        LogInfo("PR may have been deleted or merged. Continuing with other comments.")

    ELSE
        LogInfo("Unexpected error. Secret will still appear in summary/artifacts.")
    END IF

    // Step 4: Do NOT throw or exit - this is non-fatal
    // Execution continues to process other secrets
END FUNCTION
```

---

## Comment Generation

### 1. TruncateCommitSha

**Purpose:** Truncate commit SHA to first 7 characters for display.

**Inputs:**
- `commitSha`: String - Full commit SHA (40 characters)

**Outputs:**
- `String` - Truncated SHA (7 characters) or original if invalid

**Algorithm:**

```pseudo
FUNCTION TruncateCommitSha(commitSha: String) -> String
    // Step 1: Validate input
    IF commitSha IS NULL OR commitSha == "" THEN
        RETURN "unknown"
    END IF

    // Step 2: Check if SHA is valid format
    IF commitSha.length >= 7 AND IsValidCommitSha(commitSha) THEN
        RETURN commitSha.substring(0, 7)
    END IF

    // Step 3: Return original if too short or invalid
    RETURN commitSha
END FUNCTION
```

---

### 2. FormatFingerprint

**Purpose:** Format fingerprint for display in comment.

**Inputs:**
- `fingerprint`: String - Fingerprint from SARIF

**Outputs:**
- `String` - Formatted fingerprint

**Algorithm:**

```pseudo
FUNCTION FormatFingerprint(fingerprint: String) -> String
    // Step 1: Validate input
    IF fingerprint IS NULL OR fingerprint == "" THEN
        RETURN "unknown:unknown:unknown:0"
    END IF

    // Step 2: Escape any special characters
    // In case fingerprint contains characters that need escaping
    escaped = EscapeMarkdown(fingerprint)

    RETURN escaped
END FUNCTION
```

---

### 3. EscapeMarkdown

**Purpose:** Escape special markdown characters in text.

**Inputs:**
- `text`: String - Text to escape

**Outputs:**
- `String` - Escaped text

**Algorithm:**

```pseudo
FUNCTION EscapeMarkdown(text: String) -> String
    // Characters that need escaping in markdown:
    // * _ [ ] ( ) # + - . ! | \

    escaped = text

    // Escape backslash first (otherwise we'd escape our escape chars)
    escaped = escaped.replace("\\", "\\\\")

    // Escape other special characters
    escaped = escaped.replace("*", "\\*")
    escaped = escaped.replace("_", "\\_")
    escaped = escaped.replace("[", "\\[")
    escaped = escaped.replace("]", "\\]")
    escaped = escaped.replace("(", "\\(")
    escaped = escaped.replace(")", "\\)")
    escaped = escaped.replace("#", "\\#")
    escaped = escaped.replace("+", "\\+")
    escaped = escaped.replace("-", "\\-")
    escaped = escaped.replace(".", "\\.")
    escaped = escaped.replace("!", "\\!")
    escaped = escaped.replace("|", "\\|")

    RETURN escaped
END FUNCTION
```

---

## Comment Placement

### 1. ValidateCommentPlacement

**Purpose:** Validate that comment placement is valid before posting.

**Inputs:**
- `path`: String - File path
- `line`: Integer - Line number
- `commit_id`: String - Commit SHA

**Outputs:**
- `Object{isValid: Boolean, errorMessage: String}` - Validation result

**Algorithm:**

```pseudo
FUNCTION ValidateCommentPlacement(
    path: String,
    line: Integer,
    commit_id: String
) -> Object
    errors = []

    // Step 1: Validate file path
    IF path IS NULL OR path == "" THEN
        errors.append("File path is empty")
    ELSE
        // Check for path traversal
        IF path.contains("..") THEN
            errors.append("File path contains path traversal: " + path)
        END IF

        // Check for absolute path (should be relative)
        IF path.startsWith("/") OR path.contains(":\\") THEN
            errors.append("File path should be relative: " + path)
        END IF
    END IF

    // Step 2: Validate line number
    IF line < 1 THEN
        errors.append("Line number must be >= 1: " + line)
    END IF

    IF line > 1000000 THEN
        errors.append("Line number suspiciously large: " + line)
    END IF

    // Step 3: Validate commit SHA
    IF commit_id IS NULL OR commit_id == "" THEN
        errors.append("Commit SHA is empty")
    ELSE IF NOT IsValidCommitSha(commit_id) THEN
        errors.append("Invalid commit SHA format: " + commit_id)
    END IF

    // Step 4: Return result
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

### 2. NormalizePath

**Purpose:** Normalize file path for consistent comparison.

**Inputs:**
- `path`: String - File path to normalize

**Outputs:**
- `String` - Normalized path

**Algorithm:**

```pseudo
FUNCTION NormalizePath(path: String) -> String
    // Step 1: Handle null
    IF path IS NULL THEN
        RETURN ""
    END IF

    normalized = path

    // Step 2: Convert backslashes to forward slashes
    normalized = normalized.replace("\\", "/")

    // Step 3: Remove leading "./"
    WHILE normalized.startsWith("./") DO
        normalized = normalized.substring(2)
    END WHILE

    // Step 4: Remove trailing slashes
    WHILE normalized.endsWith("/") AND normalized.length > 1 DO
        normalized = normalized.substring(0, normalized.length - 1)
    END WHILE

    // Step 5: Collapse multiple slashes
    WHILE normalized.contains("//") DO
        normalized = normalized.replace("//", "/")
    END WHILE

    RETURN normalized
END FUNCTION
```

---

## Deduplication Logic

### 1. CompareComments

**Purpose:** Compare two comments for equality (used in deduplication).

**Inputs:**
- `comment1`: Object - First comment (path, line, body)
- `comment2`: Object - Second comment (path, line, body)

**Outputs:**
- `Boolean` - TRUE if comments are equal, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION CompareComments(comment1: Object, comment2: Object) -> Boolean
    // Step 1: Compare paths (normalized)
    path1 = NormalizePath(comment1.path)
    path2 = NormalizePath(comment2.path)

    IF path1 != path2 THEN
        RETURN FALSE
    END IF

    // Step 2: Compare line numbers
    IF comment1.line != comment2.line THEN
        RETURN FALSE
    END IF

    // Step 3: Compare bodies (trimmed)
    body1 = comment1.body.trim()
    body2 = comment2.body.trim()

    IF body1 != body2 THEN
        RETURN FALSE
    END IF

    // All checks passed
    RETURN TRUE
END FUNCTION
```

---

### 2. FindDuplicateComment

**Purpose:** Find if a comment already exists in list of existing comments.

**Inputs:**
- `existingComments`: Array<ExistingComment> - Existing comments
- `newComment`: PRComment - New comment to check

**Outputs:**
- `ExistingComment` - Matching comment if found, NULL otherwise

**Algorithm:**

```pseudo
FUNCTION FindDuplicateComment(
    existingComments: Array<ExistingComment>,
    newComment: PRComment
) -> ExistingComment
    // Handle null inputs
    IF existingComments IS NULL OR newComment IS NULL THEN
        RETURN NULL
    END IF

    // Search for matching comment
    FOR EACH existing IN existingComments DO
        // Build comparison objects
        obj1 = {
            path: existing.path,
            line: existing.original_line,  // Use original line
            body: existing.body
        }

        obj2 = {
            path: newComment.path,
            line: newComment.line,
            body: newComment.body
        }

        // Compare
        IF CompareComments(obj1, obj2) THEN
            RETURN existing  // Duplicate found
        END IF
    END FOR

    RETURN NULL  // No duplicate found
END FUNCTION
```

---

## Error Handling

### Error Categories

#### 1. Non-Fatal Errors (Log Warning, Continue)

```pseudo
ERROR HANDLING NonFatalCommentErrors
    WHEN LargeDiffError THEN
        LOG WARNING "Comment not posted: diff too large"
        LOG INFO "Secret will still appear in summary and artifacts"
        CONTINUE processing other secrets

    WHEN LineNotInDiffError THEN
        LOG WARNING "Comment not posted: line not in PR diff"
        LOG INFO "This can happen if the file was modified after the secret was added"
        CONTINUE processing other secrets

    WHEN RateLimitError THEN
        LOG WARNING "Comment not posted: GitHub API rate limit exceeded"
        LOG INFO "Secret will still appear in summary and artifacts"
        CONTINUE processing other secrets

    WHEN NetworkError THEN
        LOG WARNING "Comment not posted: network error"
        LOG DEBUG "Error details: " + error.message
        CONTINUE processing other secrets

    WHEN InvalidCommentError THEN
        LOG WARNING "Comment not posted: invalid comment format"
        LOG DEBUG "Path: " + path + ", Line: " + line
        CONTINUE processing other secrets
END ERROR HANDLING
```

#### 2. HTTP Status Code Handling

```pseudo
ERROR HANDLING HTTPStatusCodes
    WHEN 201 Created THEN
        // Success
        LOG DEBUG "Comment posted successfully"
        RETURN TRUE

    WHEN 422 Unprocessable Entity THEN
        // Diff too large or line not in diff
        LOG WARNING "Failed to post comment: diff too large or line not in diff"
        RETURN FALSE

    WHEN 403 Forbidden THEN
        // Permission denied or rate limit
        LOG WARNING "Failed to post comment: permission denied or rate limited"
        RETURN FALSE

    WHEN 404 Not Found THEN
        // PR not found
        LOG WARNING "Failed to post comment: PR not found"
        RETURN FALSE

    WHEN 401 Unauthorized THEN
        // Invalid token
        LOG ERROR "Failed to post comment: invalid GitHub token"
        RETURN FALSE

    WHEN 5xx Server Error THEN
        // GitHub server error
        LOG WARNING "Failed to post comment: GitHub server error"
        RETURN FALSE
END ERROR HANDLING
```

---

### Error Recovery Strategies

#### 1. Retry with Exponential Backoff

```pseudo
FUNCTION PostCommentWithRetry(
    owner: String,
    repo: String,
    pr_number: Integer,
    comment: PRComment,
    github_token: String,
    max_retries: Integer = 3
) -> Boolean
    retries = 0
    backoff = 1  // seconds

    WHILE retries < max_retries DO
        // Try to post comment
        success = PostComment(owner, repo, pr_number, comment, github_token)

        IF success THEN
            RETURN TRUE
        END IF

        // Increment retry count
        retries += 1

        // Don't retry on certain errors
        IF last_error_code == 422 OR last_error_code == 404 THEN
            // These errors won't be fixed by retrying
            RETURN FALSE
        END IF

        // Sleep before retry
        IF retries < max_retries THEN
            LogDebug("Retrying comment post in " + backoff + " seconds (attempt " + retries + ")")
            Sleep(backoff)
            backoff = backoff * 2  // Exponential backoff
        END IF
    END WHILE

    // Max retries exceeded
    LogWarning("Failed to post comment after " + max_retries + " retries")
    RETURN FALSE
END FUNCTION
```

---

## User Mentions

### 1. ParseNotifyUserList

**Purpose:** Parse comma-separated user mention list from environment variable.

**Inputs:**
- `user_list`: String - Comma-separated list (e.g., "@user1,@user2,@user3")

**Outputs:**
- `Array<String>` - Array of user mentions (with @ prefix)

**Algorithm:**

```pseudo
FUNCTION ParseNotifyUserList(user_list: String) -> Array<String>
    // Step 1: Handle null or empty input
    IF user_list IS NULL OR user_list.trim() == "" THEN
        RETURN []  // Empty array
    END IF

    // Step 2: Split by comma
    parts = user_list.split(",")

    // Step 3: Process each part
    users = []
    FOR EACH part IN parts DO
        // Trim whitespace
        user = part.trim()

        // Skip empty strings
        IF user == "" THEN
            CONTINUE
        END IF

        // Ensure @ prefix
        IF NOT user.startsWith("@") THEN
            user = "@" + user
        END IF

        // Validate username format
        IF IsValidGitHubUsername(user.substring(1)) THEN
            users.append(user)
        ELSE
            LogWarning("Invalid GitHub username: " + user)
        END IF
    END FOR

    RETURN users
END FUNCTION
```

---

### 2. IsValidGitHubUsername

**Purpose:** Validate GitHub username format.

**Inputs:**
- `username`: String - Username to validate (without @ prefix)

**Outputs:**
- `Boolean` - TRUE if valid, FALSE otherwise

**Algorithm:**

```pseudo
FUNCTION IsValidGitHubUsername(username: String) -> Boolean
    // Step 1: Check null or empty
    IF username IS NULL OR username == "" THEN
        RETURN FALSE
    END IF

    // Step 2: Check length (1-39 characters)
    IF username.length < 1 OR username.length > 39 THEN
        RETURN FALSE
    END IF

    // Step 3: Check for invalid characters
    // Valid: alphanumeric and hyphens
    // Cannot start or end with hyphen
    // Cannot have consecutive hyphens
    IF username.startsWith("-") OR username.endsWith("-") THEN
        RETURN FALSE
    END IF

    IF username.contains("--") THEN
        RETURN FALSE
    END IF

    // Step 4: Check each character
    FOR EACH char IN username DO
        IF NOT (IsAlphanumeric(char) OR char == '-') THEN
            RETURN FALSE
        END IF
    END FOR

    RETURN TRUE
END FUNCTION
```

---

### 3. FormatUserMentions

**Purpose:** Format user mentions for comment body.

**Inputs:**
- `users`: Array<String> - User mentions (with @ prefix)

**Outputs:**
- `String` - Formatted mentions or empty string

**Algorithm:**

```pseudo
FUNCTION FormatUserMentions(users: Array<String>) -> String
    // Step 1: Handle null or empty
    IF users IS NULL OR users.length == 0 THEN
        RETURN ""
    END IF

    // Step 2: Join with commas and space
    mentions = JoinStrings(users, ", ")

    // Step 3: Add "cc" prefix
    RETURN "cc " + mentions
END FUNCTION
```

**Example Output:**
```
cc @user1, @user2, @user3
```

---

## Usage Examples

### Example 1: Basic Comment Posting

```pseudo
FUNCTION Main()
    // Create request
    request = new CommentRequest()
    request.owner = "myorg"
    request.repo = "myrepo"
    request.pr_number = 123
    request.github_token = GetEnvVar("GITHUB_TOKEN")
    request.notify_users = ["@security-team"]
    request.secrets = ParsedSecrets  // From SARIF parsing

    // Post comments
    result = PostReviewComments(request)

    // Check results
    IF result.posted > 0 THEN
        LogInfo("Successfully posted " + result.posted + " comment(s)")
    END IF

    IF result.skipped > 0 THEN
        LogInfo("Skipped " + result.skipped + " duplicate comment(s)")
    END IF

    IF result.failed > 0 THEN
        LogWarning("Failed to post " + result.failed + " comment(s)")
        FOR EACH error IN result.errors DO
            LogWarning("  - " + error)
        END FOR
    END IF
END FUNCTION
```

---

### Example 2: Comment with Deduplication

```pseudo
FUNCTION PostCommentsWithDedup()
    owner = "myorg"
    repo = "myrepo"
    pr_number = 123
    token = GetEnvVar("GITHUB_TOKEN")

    // Fetch existing comments
    existing = FetchExistingComments(owner, repo, pr_number, token)

    IF existing IS NULL THEN
        LogWarning("Could not fetch existing comments, proceeding without dedup")
        existing = []
    END IF

    // Build dedup map
    commentMap = BuildDeduplicationMap(existing)

    // Process each secret
    FOR EACH secret IN secrets DO
        metadata = new CommentMetadata()
        metadata.ruleId = secret.ruleId
        metadata.commitSha = TruncateCommitSha(secret.commitSha)
        metadata.fingerprint = secret.fingerprint
        metadata.filePath = secret.filePath
        metadata.startLine = secret.startLine
        metadata.mentions = []

        body = GenerateCommentBody(metadata)
        placement = DetermineCommentPlacement(secret)

        // Check for duplicate
        IF IsDuplicateComment(commentMap, placement.path, placement.line, body) THEN
            LogInfo("Skipping duplicate: " + placement.path + ":" + placement.line)
            CONTINUE
        END IF

        // Create and post comment
        comment = new PRComment()
        comment.body = body
        comment.path = placement.path
        comment.line = placement.line
        comment.commit_id = placement.commit_id
        comment.side = placement.side

        PostComment(owner, repo, pr_number, comment, token)
    END FOR
END FUNCTION
```

---

### Example 3: Handling Large Diffs

```pseudo
FUNCTION PostCommentsWithErrorHandling()
    posted = 0
    failed = 0

    FOR EACH secret IN secrets DO
        // Generate comment
        metadata = CreateMetadata(secret)
        body = GenerateCommentBody(metadata)
        placement = DetermineCommentPlacement(secret)

        // Create comment
        comment = new PRComment()
        comment.body = body
        comment.path = placement.path
        comment.line = placement.line
        comment.commit_id = placement.commit_id
        comment.side = "RIGHT"

        // Try to post (non-fatal)
        TRY
            success = PostComment(owner, repo, pr_number, comment, token)

            IF success THEN
                posted += 1
                LogInfo("Posted: " + comment.path + ":" + comment.line)
            ELSE
                failed += 1
                LogWarning("Failed: " + comment.path + ":" + comment.line + " (likely large diff)")
            END IF

        CATCH Exception e
            failed += 1
            HandleCommentError(e, comment.path + ":" + comment.line)
        END TRY
    END FOR

    // Summary
    LogInfo("Posted: " + posted + ", Failed: " + failed)

    IF failed > 0 THEN
        LogInfo("Failed comments are non-fatal. All secrets are in summary/artifacts.")
    END IF
END FUNCTION
```

---

### Example 4: User Mention Parsing

```pseudo
FUNCTION ParseAndFormatMentions()
    // Get from environment
    notifyList = GetEnvVar("GITLEAKS_NOTIFY_USER_LIST")

    // Parse
    users = ParseNotifyUserList(notifyList)

    // Log results
    IF users.length > 0 THEN
        LogInfo("Will notify " + users.length + " user(s):")
        FOR EACH user IN users DO
            LogInfo("  - " + user)
        END FOR
    ELSE
        LogInfo("No users to notify")
    END IF

    RETURN users
END FUNCTION
```

**Example Input:**
```
GITLEAKS_NOTIFY_USER_LIST="@alice, bob, @charlie"
```

**Example Output:**
```
Will notify 3 user(s):
  - @alice
  - @bob
  - @charlie
```

---

### Example 5: Complete Integration

```pseudo
FUNCTION HandlePullRequestComments(
    pr_number: Integer,
    secrets: Array<DetectedSecret>,
    enable_comments: Boolean
)
    // Step 1: Check if comments are enabled
    IF NOT enable_comments THEN
        LogInfo("PR comments disabled via configuration")
        RETURN
    END IF

    // Step 2: Validate prerequisites
    github_token = GetEnvVar("GITHUB_TOKEN")
    IF github_token IS NULL OR github_token == "" THEN
        LogWarning("GITHUB_TOKEN not provided, skipping PR comments")
        RETURN
    END IF

    // Step 3: Parse user mentions
    notify_list = GetEnvVar("GITLEAKS_NOTIFY_USER_LIST")
    notify_users = ParseNotifyUserList(notify_list)

    // Step 4: Extract repo info from environment
    repo_full_name = GetEnvVar("GITHUB_REPOSITORY")  // "owner/repo"
    parts = repo_full_name.split("/")
    owner = parts[0]
    repo = parts[1]

    // Step 5: Create request
    request = new CommentRequest()
    request.owner = owner
    request.repo = repo
    request.pr_number = pr_number
    request.secrets = secrets
    request.github_token = github_token
    request.notify_users = notify_users

    // Step 6: Post comments
    LogInfo("Posting PR review comments...")
    result = PostReviewComments(request)

    // Step 7: Report results
    LogInfo("Comment posting complete:")
    LogInfo("  Posted: " + result.posted)
    LogInfo("  Skipped: " + result.skipped)
    LogInfo("  Failed: " + result.failed)

    IF result.failed > 0 THEN
        LogWarning("Some comments failed (non-fatal)")
        LogInfo("All secrets are still reported in summary and artifacts")
    END IF
END FUNCTION
```

---

## Implementation Notes

### 1. GitHub API Considerations

**Rate Limits:**
- 1,000 requests/hour with GITHUB_TOKEN
- 5,000 requests/hour with personal access token
- Use conditional requests (ETags) when possible
- Implement exponential backoff for rate limit errors

**Pagination:**
- API returns max 100 items per page
- Use `page` and `per_page` query parameters
- Check `Link` header for pagination info
- Set reasonable limits to prevent infinite loops

**Comment Limitations:**
- Comments can only be posted on lines in the PR diff
- Cannot comment on unchanged lines
- Large diffs may prevent commenting on some lines
- Deleted files cannot be commented on

---

### 2. Rust-Specific Implementation Hints

```rust
// Rust implementation notes (not strict pseudocode)
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct ReviewComment {
    body: String,
    commit_id: String,
    path: String,
    line: u32,
    side: String,
}

async fn post_review_comments(
    octocrab: &Octocrab,
    owner: &str,
    repo: &str,
    pr_number: u64,
    secrets: &[DetectedSecret],
) -> Result<CommentResult, Box<dyn std::error::Error>> {
    let mut result = CommentResult {
        posted: 0,
        skipped: 0,
        failed: 0,
        errors: Vec::new(),
    };

    // Fetch existing comments
    let existing_comments = fetch_existing_comments(
        octocrab,
        owner,
        repo,
        pr_number,
    )
    .await
    .unwrap_or_default();

    // Build dedup map
    let comment_map = build_deduplication_map(&existing_comments);

    // Post comments
    for secret in secrets {
        // Generate comment
        let metadata = CommentMetadata::from(secret);
        let body = generate_comment_body(&metadata);
        let placement = determine_comment_placement(secret);

        // Check duplicate
        if is_duplicate_comment(&comment_map, &placement.path, placement.line, &body) {
            result.skipped += 1;
            continue;
        }

        // Post comment (non-fatal)
        match post_comment(octocrab, owner, repo, pr_number, &comment).await {
            Ok(_) => {
                result.posted += 1;
            }
            Err(e) => {
                result.failed += 1;
                result.errors.push(format!("Failed to post comment: {}", e));
                // Continue processing other secrets
            }
        }
    }

    Ok(result)
}
```

---

### 3. Testing Considerations

**Test Cases:**

1. **Successful Comment Posting**
   - Input: Valid secret, valid PR
   - Expected: Comment posted, HTTP 201

2. **Duplicate Comment Detection**
   - Input: Same secret on re-run
   - Expected: Comment skipped

3. **Large Diff Error**
   - Input: Secret in large file
   - Expected: HTTP 422, logged as warning, non-fatal

4. **Line Not in Diff**
   - Input: Secret on line not in PR diff
   - Expected: HTTP 422, logged as warning, non-fatal

5. **Invalid Token**
   - Input: Invalid GITHUB_TOKEN
   - Expected: HTTP 401, logged as error

6. **Rate Limit**
   - Input: Exceed API rate limit
   - Expected: HTTP 403, logged as warning, retry with backoff

7. **User Mention Parsing**
   - Input: "@user1,user2, @user3"
   - Expected: ["@user1", "@user2", "@user3"]

8. **Path Normalization**
   - Input: "./src/config.js", "src\\config.js"
   - Expected: Both normalized to "src/config.js"

---

## Appendix: Comment Format Example

### Complete Comment Example

```markdown
🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123d.

**If this secret is a _true_ positive:**
- Please rotate the secret ASAP
- Update all systems using this secret
- Review access logs for unauthorized usage

**If this secret is a _false_ positive:**
You can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch:

```bash
echo 'abc123d:src/config/aws.js:aws-access-token:42' >> .gitleaksignore
```

cc @security-team, @john-doe
```

---

### API Request Example

```json
POST /repos/myorg/myrepo/pulls/123/comments
Headers:
  Accept: application/vnd.github+json
  Authorization: Bearer ghs_xxxxxxxxxxxxx
  X-GitHub-Api-Version: 2022-11-28
  Content-Type: application/json

Body:
{
  "body": "🛑 **Gitleaks** has detected a secret with rule-id `aws-access-token` in commit abc123d...",
  "commit_id": "abc123def456789abc123def456789abc123def4",
  "path": "src/config/aws.js",
  "line": 42,
  "side": "RIGHT"
}
```

---

### API Response Example (Success)

```json
HTTP/1.1 201 Created

{
  "id": 987654321,
  "body": "🛑 **Gitleaks** has detected a secret...",
  "path": "src/config/aws.js",
  "line": 42,
  "original_line": 42,
  "commit_id": "abc123def456789abc123def456789abc123def4",
  "created_at": "2025-10-16T12:34:56Z",
  "updated_at": "2025-10-16T12:34:56Z"
}
```

---

### API Response Example (Large Diff Error)

```json
HTTP/1.1 422 Unprocessable Entity

{
  "message": "Validation Failed",
  "errors": [
    {
      "resource": "PullRequestReviewComment",
      "code": "custom",
      "message": "diff too large"
    }
  ],
  "documentation_url": "https://docs.github.com/rest/reference/pulls#create-a-review-comment-for-a-pull-request"
}
```

---

## Document Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-16 | PR Comment System Specialist | Initial pseudocode document |

---

**END OF PR COMMENT SYSTEM PSEUDOCODE**

**Status:** ✅ COMPLETE

**Next Steps:**
1. Review pseudocode algorithms
2. Implement in Rust with octocrab
3. Create unit tests for each function
4. Test with real GitHub PRs
5. Test error handling for large diffs
