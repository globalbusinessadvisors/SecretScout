# PSEUDOCODE: GitHub API Integration

**Module:** `github.rs`
**Purpose:** GitHub REST API client for metadata retrieval and PR comment posting
**Specification Reference:** SPARC_SPECIFICATION.md Section 8.2
**Dependencies:** `octocrab`, `reqwest`, `serde`, `thiserror`

---

## TABLE OF CONTENTS

1. [Data Structures](#1-data-structures)
2. [Configuration](#2-configuration)
3. [Core API Functions](#3-core-api-functions)
4. [Retry Logic](#4-retry-logic)
5. [Error Handling](#5-error-handling)
6. [Rate Limit Management](#6-rate-limit-management)
7. [Complete Workflows](#7-complete-workflows)

---

## 1. DATA STRUCTURES

### 1.1 API Client Configuration

```
STRUCT GitHubApiClient:
    FIELD base_url: String = "https://api.github.com"
    FIELD auth_token: String
    FIELD user_agent: String = "SecretScout/3.0.0 (Rust/WASM)"
    FIELD rate_limit_state: RateLimitState
    FIELD http_client: HttpClient
END STRUCT

STRUCT RateLimitState:
    FIELD remaining: AtomicUsize
    FIELD reset_time: AtomicU64       // Unix timestamp
    FIELD limit: AtomicUsize
    FIELD used: AtomicUsize
END STRUCT

STRUCT RetryConfig:
    FIELD max_retries: usize = 3
    FIELD initial_backoff_ms: u64 = 1000
    FIELD max_backoff_ms: u64 = 60000
    FIELD backoff_multiplier: f64 = 2.0
    FIELD jitter_factor: f64 = 0.1
    FIELD retry_on_codes: Vec<u16> = [429, 500, 502, 503, 504]
END STRUCT
```

### 1.2 Request/Response Structures

```
STRUCT AccountTypeResponse:
    FIELD account_type: String        // "Organization" | "User"
    FIELD login: String
    FIELD id: u64
END STRUCT

STRUCT LatestReleaseResponse:
    FIELD tag_name: String            // e.g., "v8.24.3"
    FIELD name: String
    FIELD published_at: String
    FIELD assets: Vec<ReleaseAsset>
END STRUCT

STRUCT ReleaseAsset:
    FIELD name: String
    FIELD browser_download_url: String
    FIELD size: u64
END STRUCT

STRUCT PullRequestCommit:
    FIELD sha: String                 // 40-character hex SHA
    FIELD commit: CommitDetails
    FIELD author: Option<User>
END STRUCT

STRUCT CommitDetails:
    FIELD message: String
    FIELD author: GitAuthor
    FIELD committer: GitAuthor
END STRUCT

STRUCT GitAuthor:
    FIELD name: String
    FIELD email: String
    FIELD date: String
END STRUCT

STRUCT ReviewComment:
    FIELD id: u64
    FIELD body: String
    FIELD path: String
    FIELD line: Option<u32>
    FIELD commit_id: String
    FIELD created_at: String
    FIELD user: User
END STRUCT

STRUCT User:
    FIELD login: String
    FIELD id: u64
END STRUCT

STRUCT CreateCommentRequest:
    FIELD body: String
    FIELD commit_id: String
    FIELD path: String
    FIELD side: String = "RIGHT"
    FIELD line: u32
END STRUCT
```

### 1.3 Error Types

```
ENUM GitHubApiError:
    // Network errors
    NetworkError(String)

    // HTTP errors
    HttpError(u16, String)          // status code, message

    // Rate limiting
    RateLimitExceeded(RateLimitInfo)

    // Authentication
    Unauthorized(String)
    Forbidden(String)

    // Resource errors
    NotFound(String)

    // Transient errors
    ServiceUnavailable(String)
    Timeout(String)

    // Parse errors
    ParseError(String)
    InvalidResponse(String)

    // Validation errors
    InvalidInput(String)
END ENUM

STRUCT RateLimitInfo:
    FIELD remaining: usize
    FIELD reset_time: u64
    FIELD limit: usize
    FIELD retry_after: Option<u64>   // Seconds until retry allowed
END STRUCT
```

---

## 2. CONFIGURATION

### 2.1 Client Initialization

```
FUNCTION NewGitHubApiClient(token: String) -> Result<GitHubApiClient>:
    INPUT:
        - token: GitHub API authentication token (GITHUB_TOKEN)

    OUTPUT:
        - Result containing configured GitHubApiClient

    VALIDATION:
        IF token is empty:
            RETURN Error(InvalidInput("GitHub token is required"))
        END IF

    ALGORITHM:
        // Create HTTP client with timeout and TLS
        SET http_client = HttpClient::new()
            .timeout(Duration::from_secs(30))
            .user_agent("SecretScout/3.0.0 (Rust/WASM)")
            .default_headers({
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28"
            })

        // Initialize rate limit state
        SET rate_limit_state = RateLimitState {
            remaining: AtomicUsize::new(5000),
            reset_time: AtomicU64::new(0),
            limit: AtomicUsize::new(5000),
            used: AtomicUsize::new(0)
        }

        // Create client
        SET client = GitHubApiClient {
            base_url: "https://api.github.com",
            auth_token: token,
            user_agent: "SecretScout/3.0.0 (Rust/WASM)",
            rate_limit_state: rate_limit_state,
            http_client: http_client
        }

        RETURN Ok(client)
    END ALGORITHM
END FUNCTION
```

### 2.2 Request Builder

```
FUNCTION BuildRequest(
    client: GitHubApiClient,
    method: HttpMethod,
    endpoint: String,
    body: Option<String>
) -> HttpRequest:
    INPUT:
        - client: GitHubApiClient instance
        - method: GET, POST, PUT, DELETE, etc.
        - endpoint: API endpoint path (e.g., "/users/octocat")
        - body: Optional JSON request body

    OUTPUT:
        - Configured HttpRequest

    ALGORITHM:
        SET url = client.base_url + endpoint

        SET request = HttpRequest::new(method, url)
            .header("Authorization", "Bearer " + client.auth_token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", client.user_agent)

        IF body is Some(json_body):
            request = request
                .header("Content-Type", "application/json")
                .body(json_body)
        END IF

        RETURN request
    END ALGORITHM
END FUNCTION
```

---

## 3. CORE API FUNCTIONS

### 3.1 Get Account Type

```
FUNCTION GetAccountType(
    client: GitHubApiClient,
    username: String
) -> Result<String>:
    INPUT:
        - client: GitHubApiClient instance
        - username: GitHub username or organization name

    OUTPUT:
        - Result containing "Organization" or "User"

    PURPOSE:
        Determine if account is an organization (requires license) or personal user

    API ENDPOINT:
        GET /users/{username}

    ALGORITHM:
        // Validate input
        IF username is empty:
            RETURN Error(InvalidInput("Username cannot be empty"))
        END IF

        // Build request
        SET endpoint = "/users/" + username
        SET request = BuildRequest(client, GET, endpoint, None)

        // Execute with retry logic
        SET response = RetryWithBackoff(
            || ExecuteRequest(client, request),
            RetryConfig::default()
        )?

        // Check status code
        MATCH response.status_code:
            CASE 200:
                // Success - parse response
                SET json = ParseJson(response.body)?

                // Extract type field
                IF json.contains_key("type"):
                    SET account_type = json["type"].as_str()

                    // Validate type
                    IF account_type IN ["Organization", "User"]:
                        RETURN Ok(account_type)
                    ELSE:
                        RETURN Error(InvalidResponse(
                            "Unexpected account type: " + account_type
                        ))
                    END IF
                ELSE:
                    RETURN Error(InvalidResponse("Missing 'type' field"))
                END IF

            CASE 404:
                RETURN Error(NotFound("User not found: " + username))

            CASE 401:
                RETURN Error(Unauthorized("Invalid GitHub token"))

            CASE 403:
                RETURN Error(Forbidden("Access denied - check token permissions"))

            CASE other:
                RETURN Error(HttpError(
                    response.status_code,
                    "Unexpected status code: " + response.status_code
                ))
        END MATCH
    END ALGORITHM

    ERROR HANDLING:
        - Network errors: Retry with exponential backoff
        - 404 Not Found: Return error (invalid username)
        - 401/403: Return error (authentication issue)
        - Parse errors: Return error (malformed response)
        - On failure: Log warning, assume "Organization" (require license)

    EXAMPLE RESPONSE:
        {
            "login": "octocat",
            "id": 1,
            "type": "User",
            "name": "The Octocat",
            "company": "@github"
        }
END FUNCTION
```

### 3.2 Get Latest Gitleaks Release

```
FUNCTION GetLatestGitleaksRelease(
    client: GitHubApiClient
) -> Result<String>:
    INPUT:
        - client: GitHubApiClient instance

    OUTPUT:
        - Result containing version string (e.g., "8.24.3" without 'v' prefix)

    PURPOSE:
        Fetch the latest gitleaks release version from GitHub

    API ENDPOINT:
        GET /repos/zricethezav/gitleaks/releases/latest

    ALGORITHM:
        SET endpoint = "/repos/zricethezav/gitleaks/releases/latest"
        SET request = BuildRequest(client, GET, endpoint, None)

        // Execute with retry
        SET response = RetryWithBackoff(
            || ExecuteRequest(client, request),
            RetryConfig::default()
        )?

        // Check status
        MATCH response.status_code:
            CASE 200:
                SET json = ParseJson(response.body)?

                // Extract tag_name
                IF json.contains_key("tag_name"):
                    SET tag_name = json["tag_name"].as_str()

                    // Remove 'v' prefix if present
                    SET version = IF tag_name.starts_with("v"):
                        tag_name[1..]
                    ELSE:
                        tag_name
                    END IF

                    // Validate version format (semantic versioning)
                    IF IsValidVersion(version):
                        RETURN Ok(version)
                    ELSE:
                        RETURN Error(InvalidResponse(
                            "Invalid version format: " + version
                        ))
                    END IF
                ELSE:
                    RETURN Error(InvalidResponse("Missing 'tag_name' field"))
                END IF

            CASE 404:
                RETURN Error(NotFound("Gitleaks repository not found"))

            CASE other:
                RETURN Error(HttpError(
                    response.status_code,
                    "Failed to fetch latest release"
                ))
        END MATCH
    END ALGORITHM

    ERROR HANDLING:
        - On any error: Fall back to default version "8.24.3"
        - Log warning with error details
        - Continue execution with fallback

    EXAMPLE RESPONSE:
        {
            "tag_name": "v8.24.3",
            "name": "8.24.3",
            "published_at": "2024-01-15T10:30:00Z",
            "assets": [...]
        }
END FUNCTION

FUNCTION IsValidVersion(version: String) -> Boolean:
    // Validate semantic version format: X.Y.Z or X.Y.Z-suffix
    SET pattern = r"^\d+\.\d+\.\d+(-[a-zA-Z0-9.]+)?$"
    RETURN version.matches(pattern)
END FUNCTION
```

### 3.3 Get Pull Request Commits

```
FUNCTION GetPRCommits(
    client: GitHubApiClient,
    owner: String,
    repo: String,
    pr_number: u64
) -> Result<Vec<PullRequestCommit>>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner (username or org name)
        - repo: Repository name
        - pr_number: Pull request number

    OUTPUT:
        - Result containing list of commits in PR (ordered chronologically)

    PURPOSE:
        Fetch all commits in a pull request to determine scan range

    API ENDPOINT:
        GET /repos/{owner}/{repo}/pulls/{pr_number}/commits

    ALGORITHM:
        // Validate inputs
        IF owner is empty OR repo is empty:
            RETURN Error(InvalidInput("Owner and repo are required"))
        END IF

        IF pr_number == 0:
            RETURN Error(InvalidInput("Invalid PR number"))
        END IF

        SET endpoint = Format(
            "/repos/{}/{}/pulls/{}/commits",
            owner, repo, pr_number
        )

        SET all_commits = []
        SET page = 1
        SET per_page = 100

        // Paginate through all commits
        LOOP:
            SET query_params = "?page=" + page + "&per_page=" + per_page
            SET request = BuildRequest(
                client,
                GET,
                endpoint + query_params,
                None
            )

            // Execute with retry
            SET response = RetryWithBackoff(
                || ExecuteRequest(client, request),
                RetryConfig::default()
            )?

            MATCH response.status_code:
                CASE 200:
                    SET commits = ParseJson<Vec<PullRequestCommit>>(response.body)?

                    // Add to result
                    all_commits.extend(commits)

                    // Check if more pages exist
                    IF commits.length < per_page:
                        BREAK LOOP  // Last page reached
                    END IF

                    page = page + 1

                CASE 404:
                    RETURN Error(NotFound(
                        "Pull request not found: " + pr_number
                    ))

                CASE 401:
                    RETURN Error(Unauthorized("Invalid GitHub token"))

                CASE 403:
                    RETURN Error(Forbidden("Access denied to repository"))

                CASE other:
                    RETURN Error(HttpError(
                        response.status_code,
                        "Failed to fetch PR commits"
                    ))
            END MATCH
        END LOOP

        // Validate we got commits
        IF all_commits is empty:
            RETURN Error(InvalidResponse("PR has no commits"))
        END IF

        // Validate commit SHAs
        FOR EACH commit IN all_commits:
            IF NOT IsValidCommitSHA(commit.sha):
                RETURN Error(InvalidResponse(
                    "Invalid commit SHA: " + commit.sha
                ))
            END IF
        END FOR

        RETURN Ok(all_commits)
    END ALGORITHM

    ERROR HANDLING:
        - Critical error: Exit with code 1
        - Must have commits to determine scan range
        - Log detailed error message

    EXAMPLE RESPONSE:
        [
            {
                "sha": "6dcb09b5b57875f334f61aebed695e2e4193db5e",
                "commit": {
                    "message": "Fix typo in README",
                    "author": {
                        "name": "Octocat",
                        "email": "octocat@github.com",
                        "date": "2024-01-15T10:30:00Z"
                    }
                }
            }
        ]
END FUNCTION

FUNCTION IsValidCommitSHA(sha: String) -> Boolean:
    // Validate 40-character hex string
    RETURN sha.length == 40 AND sha.chars().all(|c| c.is_ascii_hexdigit())
END FUNCTION
```

### 3.4 Get Pull Request Comments

```
FUNCTION GetPRComments(
    client: GitHubApiClient,
    owner: String,
    repo: String,
    pr_number: u64
) -> Result<Vec<ReviewComment>>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner
        - repo: Repository name
        - pr_number: Pull request number

    OUTPUT:
        - Result containing list of existing review comments

    PURPOSE:
        Fetch existing PR review comments for deduplication

    API ENDPOINT:
        GET /repos/{owner}/{repo}/pulls/{pr_number}/comments

    ALGORITHM:
        // Validate inputs
        IF owner is empty OR repo is empty:
            RETURN Error(InvalidInput("Owner and repo are required"))
        END IF

        IF pr_number == 0:
            RETURN Error(InvalidInput("Invalid PR number"))
        END IF

        SET endpoint = Format(
            "/repos/{}/{}/pulls/{}/comments",
            owner, repo, pr_number
        )

        SET all_comments = []
        SET page = 1
        SET per_page = 100

        // Paginate through all comments
        LOOP:
            SET query_params = "?page=" + page + "&per_page=" + per_page
            SET request = BuildRequest(
                client,
                GET,
                endpoint + query_params,
                None
            )

            // Execute with retry
            SET response = RetryWithBackoff(
                || ExecuteRequest(client, request),
                RetryConfig::default()
            )?

            MATCH response.status_code:
                CASE 200:
                    SET comments = ParseJson<Vec<ReviewComment>>(response.body)?

                    all_comments.extend(comments)

                    // Check pagination
                    IF comments.length < per_page:
                        BREAK LOOP
                    END IF

                    page = page + 1

                CASE 404:
                    RETURN Error(NotFound("Pull request not found"))

                CASE 401:
                    RETURN Error(Unauthorized("Invalid GitHub token"))

                CASE 403:
                    RETURN Error(Forbidden("Access denied"))

                CASE other:
                    RETURN Error(HttpError(
                        response.status_code,
                        "Failed to fetch PR comments"
                    ))
            END MATCH
        END LOOP

        RETURN Ok(all_comments)
    END ALGORITHM

    ERROR HANDLING:
        - Non-critical: Log warning if fetch fails
        - Continue without deduplication
        - May result in duplicate comments (acceptable)

    EXAMPLE RESPONSE:
        [
            {
                "id": 123456,
                "body": "🛑 **Gitleaks** has detected secret...",
                "path": "config/secrets.yml",
                "line": 42,
                "commit_id": "abc123...",
                "created_at": "2024-01-15T10:30:00Z",
                "user": {
                    "login": "github-actions[bot]"
                }
            }
        ]
END FUNCTION
```

### 3.5 Post Pull Request Comment

```
FUNCTION PostPRComment(
    client: GitHubApiClient,
    owner: String,
    repo: String,
    pr_number: u64,
    comment: CreateCommentRequest
) -> Result<ReviewComment>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner
        - repo: Repository name
        - pr_number: Pull request number
        - comment: Comment details (body, file path, line, commit SHA)

    OUTPUT:
        - Result containing created comment

    PURPOSE:
        Post inline review comment on PR at specific file/line

    API ENDPOINT:
        POST /repos/{owner}/{repo}/pulls/{pr_number}/comments

    ALGORITHM:
        // Validate inputs
        IF owner is empty OR repo is empty:
            RETURN Error(InvalidInput("Owner and repo are required"))
        END IF

        IF pr_number == 0:
            RETURN Error(InvalidInput("Invalid PR number"))
        END IF

        IF comment.body is empty:
            RETURN Error(InvalidInput("Comment body is required"))
        END IF

        IF comment.path is empty:
            RETURN Error(InvalidInput("File path is required"))
        END IF

        IF comment.line == 0:
            RETURN Error(InvalidInput("Line number is required"))
        END IF

        IF NOT IsValidCommitSHA(comment.commit_id):
            RETURN Error(InvalidInput("Invalid commit SHA"))
        END IF

        // Build request body
        SET body_json = {
            "body": comment.body,
            "commit_id": comment.commit_id,
            "path": comment.path,
            "side": "RIGHT",
            "line": comment.line
        }

        SET endpoint = Format(
            "/repos/{}/{}/pulls/{}/comments",
            owner, repo, pr_number
        )

        SET request = BuildRequest(
            client,
            POST,
            endpoint,
            Some(SerializeToJson(body_json))
        )

        // Execute with retry
        SET response = RetryWithBackoff(
            || ExecuteRequest(client, request),
            RetryConfig::default()
        )?

        MATCH response.status_code:
            CASE 201:
                // Success - comment created
                SET created_comment = ParseJson<ReviewComment>(response.body)?
                RETURN Ok(created_comment)

            CASE 422:
                // Unprocessable entity - likely diff too large or line invalid
                RETURN Error(InvalidInput(
                    "Cannot post comment - line may not be in diff or diff too large"
                ))

            CASE 404:
                RETURN Error(NotFound("Pull request not found"))

            CASE 401:
                RETURN Error(Unauthorized("Invalid GitHub token"))

            CASE 403:
                RETURN Error(Forbidden(
                    "Insufficient permissions - token needs 'repo' scope"
                ))

            CASE other:
                RETURN Error(HttpError(
                    response.status_code,
                    "Failed to post comment"
                ))
        END MATCH
    END ALGORITHM

    ERROR HANDLING:
        - Non-critical: Log warning if post fails
        - Continue execution (secret still in summary/artifacts)
        - Common failures:
            - Line not in PR diff (large files)
            - Diff too large (GitHub limitation)
            - Line was removed (not on RIGHT side)

    EXAMPLE REQUEST:
        {
            "body": "🛑 **Gitleaks** detected secret...",
            "commit_id": "6dcb09b5b57875f334f61aebed695e2e4193db5e",
            "path": "config/database.yml",
            "side": "RIGHT",
            "line": 12
        }

    EXAMPLE RESPONSE:
        {
            "id": 123456,
            "body": "🛑 **Gitleaks** detected secret...",
            "path": "config/database.yml",
            "line": 12,
            "commit_id": "6dcb09b...",
            "created_at": "2024-01-15T10:30:00Z"
        }
END FUNCTION
```

### 3.6 Check for Duplicate Comment

```
FUNCTION IsDuplicateComment(
    existing_comments: Vec<ReviewComment>,
    new_comment: CreateCommentRequest
) -> Boolean:
    INPUT:
        - existing_comments: List of existing PR comments
        - new_comment: Comment to check for duplication

    OUTPUT:
        - Boolean indicating if duplicate exists

    PURPOSE:
        Prevent posting duplicate comments on PR re-runs

    ALGORITHM:
        FOR EACH existing IN existing_comments:
            // Check if comment matches on all key fields
            IF existing.path == new_comment.path
                AND existing.line == Some(new_comment.line)
                AND existing.commit_id == new_comment.commit_id
                AND existing.body.contains(new_comment.body):

                RETURN true  // Duplicate found
            END IF
        END FOR

        RETURN false  // No duplicate found
    END ALGORITHM

    NOTES:
        - Checks path, line, commit, and body content
        - Uses substring match for body (may have user mentions added)
        - Prevents spam on action re-runs
        - If fetch of existing comments fails, returns false (allow posting)
END FUNCTION
```

---

## 4. RETRY LOGIC

### 4.1 Exponential Backoff with Jitter

```
FUNCTION RetryWithBackoff<T, F>(
    operation: F,
    config: RetryConfig
) -> Result<T>
WHERE
    F: Fn() -> Result<T>
{
    INPUT:
        - operation: Function to retry
        - config: Retry configuration

    OUTPUT:
        - Result from successful operation or final error

    PURPOSE:
        Execute operation with exponential backoff on transient failures

    ALGORITHM:
        SET attempt = 0
        SET backoff_ms = config.initial_backoff_ms

        LOOP:
            // Execute operation
            SET result = operation()

            MATCH result:
                CASE Ok(value):
                    // Success - return immediately
                    RETURN Ok(value)

                CASE Err(error):
                    attempt = attempt + 1

                    // Check if error is retryable
                    IF NOT IsRetryableError(error, config):
                        // Permanent error - fail immediately
                        RETURN Err(error)
                    END IF

                    // Check retry limit
                    IF attempt >= config.max_retries:
                        // Max retries exceeded
                        RETURN Err(error)
                    END IF

                    // Calculate backoff with jitter
                    SET jitter = RandomFloat(0.0, config.jitter_factor)
                    SET jitter_multiplier = 1.0 + jitter
                    SET delay_ms = backoff_ms * jitter_multiplier

                    // Cap at max backoff
                    delay_ms = Min(delay_ms, config.max_backoff_ms)

                    // Check if error has Retry-After header
                    IF error HAS retry_after_seconds:
                        delay_ms = retry_after_seconds * 1000
                    END IF

                    // Log retry attempt
                    LOG(INFO, Format(
                        "Retry attempt {}/{} after {}ms (error: {})",
                        attempt, config.max_retries, delay_ms, error
                    ))

                    // Sleep before retry
                    Sleep(Duration::from_millis(delay_ms))

                    // Increase backoff for next attempt
                    backoff_ms = backoff_ms * config.backoff_multiplier
            END MATCH
        END LOOP
    END ALGORITHM

    EXAMPLE USAGE:
        SET response = RetryWithBackoff(
            || GetAccountType(client, "octocat"),
            RetryConfig {
                max_retries: 3,
                initial_backoff_ms: 1000,
                max_backoff_ms: 60000,
                backoff_multiplier: 2.0,
                jitter_factor: 0.1,
                retry_on_codes: [429, 500, 502, 503, 504]
            }
        )?
END FUNCTION
```

### 4.2 Retry Decision Logic

```
FUNCTION IsRetryableError(
    error: GitHubApiError,
    config: RetryConfig
) -> Boolean:
    INPUT:
        - error: Error that occurred
        - config: Retry configuration

    OUTPUT:
        - Boolean indicating if error is retryable

    ALGORITHM:
        MATCH error:
            // Rate limit - always retry with backoff
            CASE RateLimitExceeded(_):
                RETURN true

            // HTTP errors - check status code
            CASE HttpError(status_code, _):
                RETURN config.retry_on_codes.contains(status_code)

            // Network errors - retry
            CASE NetworkError(_):
                RETURN true

            // Timeout - retry
            CASE Timeout(_):
                RETURN true

            // Service unavailable - retry
            CASE ServiceUnavailable(_):
                RETURN true

            // Permanent errors - do not retry
            CASE Unauthorized(_):
                RETURN false

            CASE Forbidden(_):
                RETURN false

            CASE NotFound(_):
                RETURN false

            CASE InvalidInput(_):
                RETURN false

            CASE ParseError(_):
                RETURN false

            CASE InvalidResponse(_):
                RETURN false

            CASE _:
                RETURN false
        END MATCH
    END ALGORITHM
END FUNCTION
```

---

## 5. ERROR HANDLING

### 5.1 Error Classification

```
FUNCTION HandleAPIError(
    error: GitHubApiError,
    context: String,
    is_critical: Boolean
) -> Result<()>:
    INPUT:
        - error: Error that occurred
        - context: Description of operation (for logging)
        - is_critical: Whether error should terminate execution

    OUTPUT:
        - Result indicating continue or abort

    PURPOSE:
        Centralized error handling with logging and exit decisions

    ALGORITHM:
        // Log error with context
        MATCH error:
            CASE RateLimitExceeded(info):
                LOG(WARNING, Format(
                    "{}: Rate limit exceeded. {} requests remaining. " +
                    "Resets at {}. Retry after {} seconds.",
                    context, info.remaining, info.reset_time, info.retry_after
                ))

                IF is_critical:
                    RETURN Err(error)
                ELSE:
                    RETURN Ok(())  // Continue with degraded functionality
                END IF

            CASE HttpError(status, message):
                LOG(ERROR, Format(
                    "{}: HTTP {} - {}",
                    context, status, message
                ))

                IF is_critical:
                    RETURN Err(error)
                ELSE:
                    RETURN Ok(())
                END IF

            CASE NetworkError(message):
                LOG(ERROR, Format(
                    "{}: Network error - {}",
                    context, message
                ))

                IF is_critical:
                    RETURN Err(error)
                ELSE:
                    RETURN Ok(())
                END IF

            CASE Unauthorized(message):
                LOG(ERROR, Format(
                    "{}: Authentication failed - {}. " +
                    "Check GITHUB_TOKEN permissions.",
                    context, message
                ))

                RETURN Err(error)  // Always critical

            CASE Forbidden(message):
                LOG(ERROR, Format(
                    "{}: Access forbidden - {}. " +
                    "Token needs 'repo' scope.",
                    context, message
                ))

                RETURN Err(error)  // Always critical

            CASE NotFound(message):
                LOG(ERROR, Format(
                    "{}: Resource not found - {}",
                    context, message
                ))

                IF is_critical:
                    RETURN Err(error)
                ELSE:
                    RETURN Ok(())
                END IF

            CASE InvalidInput(message):
                LOG(ERROR, Format(
                    "{}: Invalid input - {}",
                    context, message
                ))

                RETURN Err(error)  // Always critical

            CASE _:
                LOG(ERROR, Format(
                    "{}: Unexpected error - {:?}",
                    context, error
                ))

                IF is_critical:
                    RETURN Err(error)
                ELSE:
                    RETURN Ok(())
                END IF
        END MATCH
    END ALGORITHM
END FUNCTION
```

### 5.2 Critical vs Non-Critical Operations

```
CONST CRITICAL_OPERATIONS = [
    "GetAccountType",      // Need to know if license required
    "GetPRCommits",        // Need commits for scan range
    "Authentication"       // Need valid token
]

CONST NON_CRITICAL_OPERATIONS = [
    "GetPRComments",       // Can continue without deduplication
    "PostPRComment",       // Secret still in summary/artifacts
    "GetLatestRelease"     // Can fall back to default version
]

FUNCTION IsCriticalOperation(operation_name: String) -> Boolean:
    RETURN CRITICAL_OPERATIONS.contains(operation_name)
END FUNCTION
```

---

## 6. RATE LIMIT MANAGEMENT

### 6.1 Parse Rate Limit Headers

```
FUNCTION ParseRateLimitHeaders(
    response: HttpResponse
) -> Option<RateLimitInfo>:
    INPUT:
        - response: HTTP response with GitHub headers

    OUTPUT:
        - Optional rate limit information

    PURPOSE:
        Extract rate limit info from response headers

    GITHUB HEADERS:
        - X-RateLimit-Limit: Total request quota
        - X-RateLimit-Remaining: Requests remaining
        - X-RateLimit-Reset: Unix timestamp when limit resets
        - X-RateLimit-Used: Requests used
        - Retry-After: Seconds to wait (on 429 responses)

    ALGORITHM:
        SET headers = response.headers

        // Extract limit
        SET limit = IF headers.contains("X-RateLimit-Limit"):
            headers.get("X-RateLimit-Limit").parse::<usize>().ok()
        ELSE:
            None
        END IF

        // Extract remaining
        SET remaining = IF headers.contains("X-RateLimit-Remaining"):
            headers.get("X-RateLimit-Remaining").parse::<usize>().ok()
        ELSE:
            None
        END IF

        // Extract reset time
        SET reset_time = IF headers.contains("X-RateLimit-Reset"):
            headers.get("X-RateLimit-Reset").parse::<u64>().ok()
        ELSE:
            None
        END IF

        // Extract used
        SET used = IF headers.contains("X-RateLimit-Used"):
            headers.get("X-RateLimit-Used").parse::<usize>().ok()
        ELSE:
            None
        END IF

        // Extract retry-after (on 429 errors)
        SET retry_after = IF headers.contains("Retry-After"):
            headers.get("Retry-After").parse::<u64>().ok()
        ELSE:
            None
        END IF

        // Build info if we have essential fields
        IF limit is Some AND remaining is Some AND reset_time is Some:
            RETURN Some(RateLimitInfo {
                limit: limit.unwrap(),
                remaining: remaining.unwrap(),
                reset_time: reset_time.unwrap(),
                retry_after: retry_after
            })
        ELSE:
            RETURN None
        END IF
    END ALGORITHM
END FUNCTION
```

### 6.2 Update Rate Limit State

```
FUNCTION UpdateRateLimitState(
    client: GitHubApiClient,
    info: RateLimitInfo
) -> Void:
    INPUT:
        - client: GitHubApiClient instance
        - info: Rate limit information from response

    PURPOSE:
        Update client's internal rate limit tracking

    ALGORITHM:
        // Update atomic values
        client.rate_limit_state.limit.store(info.limit, Ordering::Relaxed)
        client.rate_limit_state.remaining.store(info.remaining, Ordering::Relaxed)
        client.rate_limit_state.reset_time.store(info.reset_time, Ordering::Relaxed)

        SET used = info.limit - info.remaining
        client.rate_limit_state.used.store(used, Ordering::Relaxed)

        // Log if approaching limit
        IF info.remaining < 100:
            LOG(WARNING, Format(
                "API rate limit low: {} requests remaining. " +
                "Resets at {}",
                info.remaining, FormatTimestamp(info.reset_time)
            ))
        END IF

        // Log if limit exceeded
        IF info.remaining == 0:
            SET wait_seconds = info.reset_time - CurrentUnixTime()
            LOG(ERROR, Format(
                "API rate limit exceeded. Wait {} seconds before retry.",
                wait_seconds
            ))
        END IF
    END ALGORITHM
END FUNCTION
```

### 6.3 Check Rate Limit Before Request

```
FUNCTION CheckRateLimit(
    client: GitHubApiClient
) -> Result<()>:
    INPUT:
        - client: GitHubApiClient instance

    OUTPUT:
        - Result indicating if request should proceed

    PURPOSE:
        Pre-emptively check rate limit to avoid wasted requests

    ALGORITHM:
        SET remaining = client.rate_limit_state.remaining.load(Ordering::Relaxed)
        SET reset_time = client.rate_limit_state.reset_time.load(Ordering::Relaxed)

        // Check if depleted
        IF remaining == 0:
            SET current_time = CurrentUnixTime()

            // Check if reset time has passed
            IF current_time < reset_time:
                SET wait_seconds = reset_time - current_time

                RETURN Err(RateLimitExceeded(RateLimitInfo {
                    remaining: 0,
                    reset_time: reset_time,
                    limit: client.rate_limit_state.limit.load(Ordering::Relaxed),
                    retry_after: Some(wait_seconds)
                }))
            ELSE:
                // Reset time passed - rate limit should be refreshed
                // Let request proceed (API will return fresh limits)
                RETURN Ok(())
            END IF
        ELSE:
            // Sufficient quota remaining
            RETURN Ok(())
        END IF
    END ALGORITHM
END FUNCTION
```

---

## 7. COMPLETE WORKFLOWS

### 7.1 Validate License for Organization

```
FUNCTION ValidateLicenseForOrganization(
    client: GitHubApiClient,
    owner: String,
    license_key: Option<String>
) -> Result<()>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner username
        - license_key: Optional license key from GITLEAKS_LICENSE

    OUTPUT:
        - Result indicating if execution can proceed

    PURPOSE:
        Determine if organization account has valid license

    WORKFLOW:
        Step 1: Determine account type
        Step 2: If organization, validate license key
        Step 3: Return success or error

    ALGORITHM:
        // Step 1: Get account type
        LOG(INFO, "Checking account type for: " + owner)

        SET account_type_result = GetAccountType(client, owner)

        SET account_type = MATCH account_type_result:
            CASE Ok(type_str):
                type_str

            CASE Err(error):
                // Non-fatal: Assume organization (require license)
                LOG(WARNING, Format(
                    "Failed to determine account type: {}. " +
                    "Assuming organization (license required).",
                    error
                ))
                "Organization"
        END MATCH

        // Step 2: Check if license required
        IF account_type == "User":
            LOG(INFO, "Personal account detected - license not required")
            RETURN Ok(())
        END IF

        // Organization - license required
        LOG(INFO, "Organization account detected - license required")

        IF license_key is None:
            RETURN Err(InvalidInput(
                "Organization accounts require GITLEAKS_LICENSE environment variable. " +
                "Get your license at https://gitleaks.io/"
            ))
        END IF

        // Step 3: Validate license (if feature enabled)
        // NOTE: Currently disabled in implementation
        // When re-enabled, call Keygen API here

        LOG(INFO, "License validation currently disabled")
        RETURN Ok(())
    END ALGORITHM

    ERROR HANDLING:
        - Account type fetch failure: Assume organization
        - Missing license: Exit with code 1
        - Invalid license: Exit with code 1
END FUNCTION
```

### 7.2 Fetch PR Scan Range

```
FUNCTION FetchPRScanRange(
    client: GitHubApiClient,
    owner: String,
    repo: String,
    pr_number: u64
) -> Result<(String, String)>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner
        - repo: Repository name
        - pr_number: Pull request number

    OUTPUT:
        - Result containing (base_sha, head_sha) tuple

    PURPOSE:
        Determine git commit range to scan for PR

    WORKFLOW:
        Step 1: Fetch PR commits
        Step 2: Extract first and last commit SHAs
        Step 3: Return scan range

    ALGORITHM:
        // Step 1: Fetch commits
        LOG(INFO, Format(
            "Fetching commits for PR #{} in {}/{}",
            pr_number, owner, repo
        ))

        SET commits = GetPRCommits(client, owner, repo, pr_number)?

        LOG(INFO, Format("Found {} commits in PR", commits.length))

        // Step 2: Validate commits exist
        IF commits is empty:
            RETURN Err(InvalidResponse("Pull request has no commits"))
        END IF

        // Step 3: Extract range
        SET base_sha = commits[0].sha
        SET head_sha = commits[commits.length - 1].sha

        LOG(INFO, Format(
            "PR scan range: {}^..{}",
            base_sha, head_sha
        ))

        RETURN Ok((base_sha, head_sha))
    END ALGORITHM

    ERROR HANDLING:
        - Critical: Must have commits for scan
        - On error: Exit with code 1
END FUNCTION
```

### 7.3 Post Comments on PR Secrets

```
FUNCTION PostCommentsOnPRSecrets(
    client: GitHubApiClient,
    owner: String,
    repo: String,
    pr_number: u64,
    secrets: Vec<SarifResult>,
    notify_users: Option<String>
) -> Result<usize>:
    INPUT:
        - client: GitHubApiClient instance
        - owner: Repository owner
        - repo: Repository name
        - pr_number: Pull request number
        - secrets: List of detected secrets from SARIF
        - notify_users: Optional comma-separated user mentions

    OUTPUT:
        - Result containing count of comments posted

    PURPOSE:
        Post inline review comments for each detected secret

    WORKFLOW:
        Step 1: Fetch existing comments (for deduplication)
        Step 2: For each secret, build comment
        Step 3: Check if duplicate
        Step 4: Post if not duplicate
        Step 5: Return count posted

    ALGORITHM:
        // Step 1: Fetch existing comments
        LOG(INFO, "Fetching existing PR comments for deduplication")

        SET existing_comments = MATCH GetPRComments(client, owner, repo, pr_number):
            CASE Ok(comments):
                LOG(INFO, Format("Found {} existing comments", comments.length))
                comments

            CASE Err(error):
                // Non-fatal: Continue without deduplication
                LOG(WARNING, Format(
                    "Failed to fetch existing comments: {}. " +
                    "Proceeding without deduplication.",
                    error
                ))
                Vec::new()
        END MATCH

        // Step 2: Process each secret
        SET posted_count = 0
        SET skipped_count = 0
        SET error_count = 0

        FOR EACH secret IN secrets:
            // Extract location info
            SET file_path = secret.locations[0].physical_location.artifact_location.uri
            SET line_number = secret.locations[0].physical_location.region.start_line
            SET commit_sha = secret.partial_fingerprints.commit_sha
            SET rule_id = secret.rule_id

            // Build fingerprint
            SET fingerprint = Format(
                "{}:{}:{}:{}",
                commit_sha, file_path, rule_id, line_number
            )

            // Build comment body
            SET comment_body = BuildCommentBody(
                rule_id,
                commit_sha,
                fingerprint,
                notify_users
            )

            // Create comment request
            SET comment_request = CreateCommentRequest {
                body: comment_body,
                commit_id: commit_sha,
                path: file_path,
                side: "RIGHT",
                line: line_number
            }

            // Step 3: Check for duplicate
            IF IsDuplicateComment(existing_comments, comment_request):
                LOG(INFO, Format(
                    "Skipping duplicate comment on {}:{}",
                    file_path, line_number
                ))
                skipped_count = skipped_count + 1
                CONTINUE
            END IF

            // Step 4: Post comment
            LOG(INFO, Format(
                "Posting comment on {}:{}",
                file_path, line_number
            ))

            SET post_result = PostPRComment(
                client,
                owner,
                repo,
                pr_number,
                comment_request
            )

            MATCH post_result:
                CASE Ok(created_comment):
                    LOG(INFO, Format(
                        "Successfully posted comment (ID: {})",
                        created_comment.id
                    ))
                    posted_count = posted_count + 1

                CASE Err(error):
                    // Non-fatal: Log and continue
                    LOG(WARNING, Format(
                        "Failed to post comment on {}:{} - {}. " +
                        "Secret still reported in summary/artifacts.",
                        file_path, line_number, error
                    ))
                    error_count = error_count + 1
            END MATCH
        END FOR

        // Step 5: Log summary
        LOG(INFO, Format(
            "Comment posting complete: {} posted, {} skipped, {} errors",
            posted_count, skipped_count, error_count
        ))

        RETURN Ok(posted_count)
    END ALGORITHM

    ERROR HANDLING:
        - Non-critical: Continue on individual comment failures
        - Secrets still reported in summary and artifacts
        - Common failures logged but don't terminate execution
END FUNCTION

FUNCTION BuildCommentBody(
    rule_id: String,
    commit_sha: String,
    fingerprint: String,
    notify_users: Option<String>
) -> String:
    SET body = Format(
        "🛑 **Gitleaks** has detected secret in code.\n\n" +
        "**Rule:** {}\n" +
        "**Commit:** {}\n\n" +
        "**Fingerprint:** `{}`\n\n" +
        "To ignore this secret, add the fingerprint to `.gitleaksignore`",
        rule_id,
        commit_sha[0..7],  // First 7 chars of SHA
        fingerprint
    )

    IF notify_users is Some:
        body = body + "\n\n**CC:** " + notify_users
    END IF

    RETURN body
END FUNCTION
```

### 7.4 Complete GitHub API Integration Example

```
FUNCTION ExecutePullRequestWorkflow(
    github_token: String,
    owner: String,
    repo: String,
    pr_number: u64,
    license_key: Option<String>,
    enable_comments: Boolean,
    notify_users: Option<String>
) -> Result<(String, String)>:
    INPUT:
        - github_token: GitHub API authentication token
        - owner: Repository owner
        - repo: Repository name
        - pr_number: Pull request number
        - license_key: Optional license key
        - enable_comments: Whether to post PR comments
        - notify_users: Optional user mentions

    OUTPUT:
        - Result containing (base_sha, head_sha) for scan range

    PURPOSE:
        Complete workflow for PR event GitHub API interactions

    ALGORITHM:
        // Initialize API client
        LOG(INFO, "Initializing GitHub API client")
        SET client = NewGitHubApiClient(github_token)?

        // Validate license (if organization)
        LOG(INFO, "Validating license requirements")
        ValidateLicenseForOrganization(client, owner, license_key)?

        // Fetch PR scan range
        LOG(INFO, "Determining PR scan range")
        SET (base_sha, head_sha) = FetchPRScanRange(
            client,
            owner,
            repo,
            pr_number
        )?

        // Note: Comment posting happens after gitleaks scan
        // This is just the API setup and scan range determination

        RETURN Ok((base_sha, head_sha))
    END ALGORITHM

    SUBSEQUENT WORKFLOW:
        After gitleaks scan completes and SARIF is parsed:

        IF enable_comments AND secrets detected:
            PostCommentsOnPRSecrets(
                client,
                owner,
                repo,
                pr_number,
                secrets,
                notify_users
            )
        END IF
END FUNCTION
```

---

## 8. IMPLEMENTATION NOTES

### 8.1 Thread Safety

All rate limit state uses atomic operations for thread-safe access:
- `AtomicUsize` for counts
- `AtomicU64` for timestamps
- `Ordering::Relaxed` sufficient (no cross-field dependencies)

### 8.2 Memory Efficiency

- Use streaming JSON parsing for large responses
- Paginate API results (max 100 per page)
- Release response bodies after parsing
- Reuse HTTP client connection pool

### 8.3 Security Considerations

- Never log `github_token` value
- Use TLS for all API requests (rustls)
- Validate all inputs before API calls
- Sanitize error messages (no token leakage)

### 8.4 Testing Strategy

**Unit Tests:**
- Response parsing with valid/invalid JSON
- Fingerprint generation
- Duplicate comment detection
- Retry logic with mock failures
- Rate limit header parsing

**Integration Tests:**
- End-to-end API calls (requires test token)
- Pagination handling
- Error handling (404, 401, 403, 429)
- Retry with real rate limits

**Mock Tests:**
- Use `mockito` crate for HTTP mocking
- Test all error paths
- Test retry backoff timing
- Test rate limit threshold detection

### 8.5 Observability

**Logging:**
- INFO: API calls, pagination, results
- WARNING: Retries, rate limit warnings, non-critical failures
- ERROR: Authentication failures, critical errors

**Metrics:**
- API request count per endpoint
- Retry count per operation
- Rate limit usage percentage
- Comment posting success rate

### 8.6 Performance Optimizations

- Connection pooling (reuse TCP connections)
- HTTP/2 multiplexing if available
- Concurrent API calls where possible (user lookup + commits)
- Conditional requests with ETags (future enhancement)
- Response compression (Accept-Encoding: gzip)

---

## 9. ERROR SCENARIOS AND HANDLING

### 9.1 Critical Errors (Exit Code 1)

| Scenario | Error | Action |
|----------|-------|--------|
| Invalid token | Unauthorized | Exit with clear error message |
| Missing token | InvalidInput | Exit - token required for PR events |
| Insufficient permissions | Forbidden | Exit - token needs 'repo' scope |
| PR not found | NotFound | Exit - invalid PR number |
| No commits in PR | InvalidResponse | Exit - cannot determine scan range |
| Organization missing license | InvalidInput | Exit - license required |

### 9.2 Non-Critical Errors (Log Warning, Continue)

| Scenario | Error | Action |
|----------|-------|--------|
| Cannot fetch comments | Any | Continue without deduplication |
| Cannot post comment | 422, others | Log warning, secret in summary |
| Cannot determine account type | Any | Assume organization, require license |
| Cannot fetch latest release | Any | Fall back to default version |

### 9.3 Retry Scenarios

| Scenario | Status Code | Retry Strategy |
|----------|-------------|----------------|
| Rate limit exceeded | 429 | Retry with Retry-After header |
| Server error | 500, 502, 503, 504 | Exponential backoff |
| Timeout | N/A | Exponential backoff |
| Network error | N/A | Exponential backoff |

---

## 10. API REFERENCE SUMMARY

### 10.1 Endpoints Used

| Endpoint | Method | Purpose | Critical |
|----------|--------|---------|----------|
| `/users/{username}` | GET | Account type | No |
| `/repos/zricethezav/gitleaks/releases/latest` | GET | Latest version | No |
| `/repos/{owner}/{repo}/pulls/{pr}/commits` | GET | PR commits | Yes |
| `/repos/{owner}/{repo}/pulls/{pr}/comments` | GET | Existing comments | No |
| `/repos/{owner}/{repo}/pulls/{pr}/comments` | POST | Create comment | No |

### 10.2 Authentication

All requests require `Authorization: Bearer {GITHUB_TOKEN}` header.

Token scopes required:
- `repo` (full repository access) for private repositories
- `public_repo` sufficient for public repositories

### 10.3 Rate Limits

| Token Type | Limit | Notes |
|------------|-------|-------|
| Workflow token (GITHUB_TOKEN) | 1,000/hour | Per repository |
| Personal access token | 5,000/hour | Per user |
| Unauthenticated | 60/hour | Not applicable |

### 10.4 Response Headers

| Header | Description |
|--------|-------------|
| `X-RateLimit-Limit` | Total quota |
| `X-RateLimit-Remaining` | Requests remaining |
| `X-RateLimit-Reset` | Unix timestamp of reset |
| `X-RateLimit-Used` | Requests used |
| `Retry-After` | Seconds to wait (on 429) |

---

## COMPLETION CHECKLIST

- [x] Data structures defined
- [x] Configuration functions specified
- [x] All 5 required API endpoints implemented
- [x] Retry logic with exponential backoff
- [x] Jitter added to prevent thundering herd
- [x] Rate limit parsing and tracking
- [x] Error classification (critical vs non-critical)
- [x] Duplicate comment detection
- [x] Complete workflows for PR event
- [x] Security considerations documented
- [x] Thread safety addressed
- [x] Testing strategy defined
- [x] Error scenarios catalogued

**Status:** ✅ COMPLETE

**Next Steps:**
1. Implement in `src/github.rs`
2. Write unit tests
3. Write integration tests with mock server
4. Test with real GitHub API (requires token)
5. Integrate with event routing module

---

**END OF PSEUDOCODE**
