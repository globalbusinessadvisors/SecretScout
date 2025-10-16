# PSEUDOCODE: SecretScout - Gitleaks-Action Rust Port

**Project:** SecretScout - Rust Port of gitleaks-action
**Methodology:** SPARC (Specification, Pseudocode, Architecture, Refinement, Completion)
**Phase:** PSEUDOCODE ONLY
**Date:** October 16, 2025
**Version:** 1.0

---

## TABLE OF CONTENTS

1. [Introduction](#1-introduction)
2. [Module Overview](#2-module-overview)
3. [Data Structures](#3-data-structures)
4. [Module 1: Main Orchestrator](#4-module-1-main-orchestrator)
5. [Module 2: Event Routing](#5-module-2-event-routing)
6. [Module 3: Binary Management](#6-module-3-binary-management)
7. [Module 4: SARIF Processing](#7-module-4-sarif-processing)
8. [Module 5: PR Comment Management](#8-module-5-pr-comment-management)
9. [Module 6: Job Summary Generation](#9-module-6-job-summary-generation)
10. [Module 7: Configuration Management](#10-module-7-configuration-management)
11. [Module 8: License Validation](#11-module-8-license-validation)
12. [Module 9: GitHub API Integration](#12-module-9-github-api-integration)
13. [Error Handling Strategy](#13-error-handling-strategy)
14. [Control Flow Diagrams](#14-control-flow-diagrams)

---

## 1. INTRODUCTION

### 1.1 Purpose

This document provides comprehensive pseudocode for the SecretScout project, a Rust port of gitleaks-action. The pseudocode is language-agnostic and describes WHAT and HOW at an algorithmic level, without implementing actual code.

### 1.2 Pseudocode Conventions

```
NOTATION GUIDE:
- UPPERCASE: Keywords and control structures
- lowercase: Variables and function names
- CamelCase: Data types and structures
- snake_case: Function and variable names
- // Single line comment
- /* Multi-line comment */
- -> Return type indicator
- ?: Error handling operator
```

### 1.3 Cross-References

All pseudocode sections reference the specification document:
- **FR-X**: Functional Requirement X
- **TR-X**: Technical Requirement X
- **SR-X**: Security Requirement X

---

## 2. MODULE OVERVIEW

### 2.1 Module Dependency Graph

```
┌─────────────────────────────────────────────────┐
│         Main Orchestrator (Entry Point)         │
│         [Section 4]                             │
└───────────┬─────────────────────────────────────┘
            │
            ├──> Event Routing [Section 5] (FR-1)
            │    └──> Configuration [Section 10] (FR-8, FR-9)
            │
            ├──> License Validation [Section 11] (FR-7)
            │    └──> GitHub API [Section 12]
            │
            ├──> Binary Management [Section 6] (FR-2)
            │    └──> Configuration [Section 10]
            │
            ├──> SARIF Processing [Section 7] (FR-3)
            │
            ├──> PR Comment Management [Section 8] (FR-4)
            │    └──> GitHub API [Section 12]
            │
            ├──> Job Summary Generation [Section 9] (FR-5)
            │
            └──> Artifact Upload (Handled by JS wrapper)
```

### 2.2 Module Responsibilities

| Module | Primary Responsibility | Specification Reference |
|--------|----------------------|------------------------|
| Main Orchestrator | Coordinate execution flow | All FR sections |
| Event Routing | Parse events, determine scan strategy | FR-1 |
| Binary Management | Download, cache, execute gitleaks | FR-2 |
| SARIF Processing | Parse SARIF, extract findings | FR-3 |
| PR Comment Management | Create inline PR comments | FR-4 |
| Job Summary Generation | Generate GitHub Actions summaries | FR-5 |
| Configuration Management | Parse environment variables | FR-8, FR-9 |
| License Validation | Validate organization licenses | FR-7 |
| GitHub API Integration | All GitHub API interactions | Section 8.2 |

---

## 3. DATA STRUCTURES

### 3.1 Core Data Types

```pseudocode
STRUCTURE Config {
    github_token: String,              // GITHUB_TOKEN
    gitleaks_license: Optional<String>, // GITLEAKS_LICENSE
    gitleaks_version: String,           // Default: "8.24.3"
    gitleaks_config_path: Optional<String>, // Path to config file
    enable_summary: Boolean,            // Default: true
    enable_upload_artifact: Boolean,    // Default: true
    enable_comments: Boolean,           // Default: true
    notify_user_list: List<String>,     // @mentions
    base_ref: Optional<String>,         // Override base ref
    workspace_path: String,             // GITHUB_WORKSPACE
    event_path: String,                 // GITHUB_EVENT_PATH
    event_name: String,                 // GITHUB_EVENT_NAME
    repository: String,                 // GITHUB_REPOSITORY (owner/repo)
    repository_owner: String            // GITHUB_REPOSITORY_OWNER
}

STRUCTURE EventContext {
    event_type: EventType,
    repository: Repository,
    base_ref: String,
    head_ref: String,
    commits: List<Commit>,
    pull_request: Optional<PullRequest>
}

ENUMERATION EventType {
    Push,
    PullRequest,
    WorkflowDispatch,
    Schedule
}

STRUCTURE Repository {
    owner: String,
    name: String,
    full_name: String,
    html_url: String
}

STRUCTURE Commit {
    sha: String,
    author: Author,
    message: String
}

STRUCTURE Author {
    name: String,
    email: String
}

STRUCTURE PullRequest {
    number: Integer,
    base: GitReference,
    head: GitReference
}

STRUCTURE GitReference {
    sha: String,
    ref_name: String
}

STRUCTURE SarifReport {
    runs: List<SarifRun>
}

STRUCTURE SarifRun {
    results: List<SarifResult>
}

STRUCTURE SarifResult {
    rule_id: String,
    message: String,
    locations: List<SarifLocation>,
    partial_fingerprints: PartialFingerprints
}

STRUCTURE SarifLocation {
    physical_location: PhysicalLocation
}

STRUCTURE PhysicalLocation {
    artifact_location: ArtifactLocation,
    region: Region
}

STRUCTURE ArtifactLocation {
    uri: String  // File path
}

STRUCTURE Region {
    start_line: Integer
}

STRUCTURE PartialFingerprints {
    commit_sha: String,
    author: String,
    email: String,
    date: String
}

STRUCTURE Finding {
    rule_id: String,
    file_path: String,
    line_number: Integer,
    commit_sha: String,
    author: String,
    email: String,
    date: String,
    fingerprint: String
}

STRUCTURE PRComment {
    body: String,
    commit_id: String,
    path: String,
    line: Integer,
    side: String  // "RIGHT"
}

STRUCTURE AccountInfo {
    account_type: AccountType,
    login: String
}

ENUMERATION AccountType {
    User,
    Organization
}

STRUCTURE ValidationResult {
    is_valid: Boolean,
    validation_code: String,
    error_message: Optional<String>
}

STRUCTURE GitleaksExecutionResult {
    exit_code: Integer,
    stdout: String,
    stderr: String
}
```

---

## 4. MODULE 1: MAIN ORCHESTRATOR

### 4.1 Entry Point

**References:** All FR sections

```pseudocode
FUNCTION main() -> ExitCode {
    /*
     * Main entry point for SecretScout action
     * Coordinates all modules and handles top-level error handling
     */

    TRY {
        // Step 1: Load configuration from environment variables
        config = load_configuration()?

        // Step 2: Parse event context
        event_context = parse_event_context(config)?

        // Step 3: Validate license (if required)
        IF should_validate_license(config, event_context.repository) THEN
            validate_license(config, event_context.repository)?
        END IF

        // Step 4: Obtain gitleaks binary
        gitleaks_path = obtain_gitleaks_binary(config)?

        // Step 5: Build gitleaks arguments based on event type
        gitleaks_args = build_gitleaks_arguments(config, event_context)?

        // Step 6: Execute gitleaks scan
        execution_result = execute_gitleaks(gitleaks_path, gitleaks_args)?

        // Step 7: Process results based on exit code
        MATCH execution_result.exit_code {
            CASE 0:  // No secrets found
                IF config.enable_summary THEN
                    generate_success_summary()
                END IF
                RETURN ExitCode(0)

            CASE 2:  // Secrets detected
                // Parse SARIF report
                sarif_report = parse_sarif_report(config.workspace_path + "/results.sarif")?
                findings = extract_findings_from_sarif(sarif_report)?

                // Generate outputs (must complete before exiting)
                IF event_context.event_type == EventType.PullRequest AND config.enable_comments THEN
                    post_pr_comments(config, event_context, findings)
                END IF

                IF config.enable_summary THEN
                    generate_findings_summary(event_context.repository, findings)
                END IF

                IF config.enable_upload_artifact THEN
                    // Artifact upload handled by JavaScript wrapper
                    log_info("SARIF report ready for artifact upload")
                END IF

                RETURN ExitCode(1)  // Fail workflow when secrets found

            CASE 1:  // Gitleaks error
                IF config.enable_summary THEN
                    generate_error_summary(1)
                END IF
                RETURN ExitCode(1)

            DEFAULT:  // Unexpected exit code
                log_error("Unexpected gitleaks exit code: " + execution_result.exit_code)
                RETURN ExitCode(execution_result.exit_code)
        }

    } CATCH error {
        log_error("Fatal error: " + error.message)
        log_debug("Stack trace: " + error.stack_trace)
        RETURN ExitCode(1)
    }
}
```

### 4.2 Configuration Loading Helper

```pseudocode
FUNCTION load_configuration() -> Result<Config> {
    /*
     * Load and validate all configuration from environment variables
     * References: FR-8, FR-9
     */

    // Required environment variables (GitHub Actions provides these)
    workspace = get_env("GITHUB_WORKSPACE")?
    event_path = get_env("GITHUB_EVENT_PATH")?
    event_name = get_env("GITHUB_EVENT_NAME")?
    repository = get_env("GITHUB_REPOSITORY")?
    repository_owner = get_env("GITHUB_REPOSITORY_OWNER")?

    // Optional but critical for PR events
    github_token = get_env("GITHUB_TOKEN")
    IF github_token IS EMPTY THEN
        IF event_name == "pull_request" THEN
            RETURN Error("GITHUB_TOKEN is required for pull_request events")
        END IF
    END IF

    // Gitleaks-specific configuration
    gitleaks_license = get_env("GITLEAKS_LICENSE")
    gitleaks_version = get_env_or_default("GITLEAKS_VERSION", "8.24.3")
    gitleaks_config = get_env("GITLEAKS_CONFIG")

    // Feature toggles (FR-8 boolean parsing)
    enable_summary = parse_boolean_env("GITLEAKS_ENABLE_SUMMARY", true)
    enable_upload_artifact = parse_boolean_env("GITLEAKS_ENABLE_UPLOAD_ARTIFACT", true)
    enable_comments = parse_boolean_env("GITLEAKS_ENABLE_COMMENTS", true)

    // User notification list
    notify_list = parse_user_list(get_env("GITLEAKS_NOTIFY_USER_LIST"))

    // Base ref override
    base_ref_override = get_env("BASE_REF")

    // Auto-detect config file if not explicitly set
    IF gitleaks_config IS EMPTY THEN
        default_config_path = workspace + "/gitleaks.toml"
        IF file_exists(default_config_path) THEN
            gitleaks_config = default_config_path
        END IF
    END IF

    RETURN Config {
        github_token: github_token,
        gitleaks_license: gitleaks_license,
        gitleaks_version: gitleaks_version,
        gitleaks_config_path: gitleaks_config,
        enable_summary: enable_summary,
        enable_upload_artifact: enable_upload_artifact,
        enable_comments: enable_comments,
        notify_user_list: notify_list,
        base_ref: base_ref_override,
        workspace_path: workspace,
        event_path: event_path,
        event_name: event_name,
        repository: repository,
        repository_owner: repository_owner
    }
}

FUNCTION parse_boolean_env(key: String, default: Boolean) -> Boolean {
    /*
     * Parse boolean environment variables per FR-8 specification
     * False values: "false", "0"
     * True values: All other values (including empty, "true", "1")
     */

    value = get_env(key)

    IF value IS EMPTY THEN
        RETURN default
    END IF

    IF value == "false" OR value == "0" THEN
        RETURN false
    ELSE
        RETURN true
    END IF
}

FUNCTION parse_user_list(user_list_str: String) -> List<String> {
    /*
     * Parse comma-separated user list with @ prefix
     * Example: "@user1,@user2,@user3" -> ["@user1", "@user2", "@user3"]
     */

    IF user_list_str IS EMPTY THEN
        RETURN empty_list
    END IF

    users = split(user_list_str, ",")
    result = empty_list

    FOR EACH user IN users DO
        trimmed = trim_whitespace(user)
        IF trimmed IS NOT EMPTY THEN
            append(result, trimmed)
        END IF
    END FOR

    RETURN result
}
```

---

## 5. MODULE 2: EVENT ROUTING

### 5.1 Event Context Parser

**References:** FR-1, Section 7 (Behavioral Specifications)

```pseudocode
FUNCTION parse_event_context(config: Config) -> Result<EventContext> {
    /*
     * Parse GitHub event JSON and create EventContext
     * Handles all four event types: push, pull_request, workflow_dispatch, schedule
     */

    // Read event JSON file
    event_json = read_json_file(config.event_path)?

    // Determine event type
    event_type = parse_event_type(config.event_name)?

    // Parse repository information
    repository = parse_repository(event_json, config)?

    // Route to event-specific parser
    MATCH event_type {
        CASE EventType.Push:
            RETURN parse_push_event(event_json, repository, config.base_ref)

        CASE EventType.PullRequest:
            RETURN parse_pull_request_event(event_json, repository, config)

        CASE EventType.WorkflowDispatch:
            RETURN parse_workflow_dispatch_event(event_json, repository)

        CASE EventType.Schedule:
            RETURN parse_schedule_event(event_json, repository, config)

        DEFAULT:
            RETURN Error("Unsupported event type: " + config.event_name)
    }
}

FUNCTION parse_event_type(event_name: String) -> Result<EventType> {
    /*
     * Map GITHUB_EVENT_NAME to EventType enum
     */

    MATCH event_name {
        CASE "push":
            RETURN EventType.Push

        CASE "pull_request":
            RETURN EventType.PullRequest

        CASE "workflow_dispatch":
            RETURN EventType.WorkflowDispatch

        CASE "schedule":
            RETURN EventType.Schedule

        DEFAULT:
            RETURN Error("Unsupported event: " + event_name)
    }
}

FUNCTION parse_repository(event_json: JsonObject, config: Config) -> Result<Repository> {
    /*
     * Extract repository information from event JSON
     * Handle special case for schedule events where repository may be undefined
     */

    IF event_json.has_field("repository") THEN
        repo_obj = event_json.get("repository")

        RETURN Repository {
            owner: repo_obj.get("owner").get("login"),
            name: repo_obj.get("name"),
            full_name: repo_obj.get("full_name"),
            html_url: repo_obj.get("html_url")
        }
    ELSE
        // Schedule event fallback (FR-1, Section 7.4)
        owner = config.repository_owner
        full_name = config.repository
        name = extract_repo_name(full_name, owner)

        RETURN Repository {
            owner: owner,
            name: name,
            full_name: full_name,
            html_url: "https://github.com/" + full_name
        }
    END IF
}

FUNCTION extract_repo_name(full_name: String, owner: String) -> String {
    /*
     * Extract repo name from "owner/repo" format
     */

    prefix = owner + "/"
    IF starts_with(full_name, prefix) THEN
        RETURN substring(full_name, length(prefix))
    ELSE
        RETURN full_name
    END IF
}
```

### 5.2 Push Event Parser

**References:** FR-1, Section 7.1

```pseudocode
FUNCTION parse_push_event(event_json: JsonObject, repository: Repository,
                          base_ref_override: Optional<String>) -> Result<EventContext> {
    /*
     * Parse push event and determine scan range
     * Handles single commit vs multi-commit scenarios
     */

    commits = event_json.get("commits")

    IF length(commits) == 0 THEN
        log_info("No commits in push event, skipping scan")
        RETURN Error("NoCommitsError")  // Will result in clean exit
    END IF

    // Determine base and head refs
    first_commit = commits[0]
    last_commit = commits[length(commits) - 1]

    base_ref = first_commit.get("id")
    head_ref = last_commit.get("id")

    // Apply base ref override if specified
    IF base_ref_override IS NOT EMPTY THEN
        base_ref = base_ref_override
    END IF

    RETURN EventContext {
        event_type: EventType.Push,
        repository: repository,
        base_ref: base_ref,
        head_ref: head_ref,
        commits: parse_commit_list(commits),
        pull_request: None
    }
}

FUNCTION parse_commit_list(commits_json: JsonArray) -> List<Commit> {
    /*
     * Parse commit array from event JSON
     */

    result = empty_list

    FOR EACH commit_obj IN commits_json DO
        commit = Commit {
            sha: commit_obj.get("id"),
            author: Author {
                name: commit_obj.get("author").get("name"),
                email: commit_obj.get("author").get("email")
            },
            message: commit_obj.get("message")
        }
        append(result, commit)
    END FOR

    RETURN result
}
```

### 5.3 Pull Request Event Parser

**References:** FR-1, Section 7.2

```pseudocode
FUNCTION parse_pull_request_event(event_json: JsonObject, repository: Repository,
                                  config: Config) -> Result<EventContext> {
    /*
     * Parse pull request event
     * Fetches PR commits via GitHub API to determine scan range
     */

    pr_obj = event_json.get("pull_request")
    pr_number = pr_obj.get("number")

    // Create PullRequest structure
    pull_request = PullRequest {
        number: pr_number,
        base: GitReference {
            sha: pr_obj.get("base").get("sha"),
            ref_name: pr_obj.get("base").get("ref")
        },
        head: GitReference {
            sha: pr_obj.get("head").get("sha"),
            ref_name: pr_obj.get("head").get("ref")
        }
    }

    // Fetch PR commits to determine exact scan range
    pr_commits = fetch_pr_commits(config.github_token, repository, pr_number)?

    IF length(pr_commits) == 0 THEN
        RETURN Error("No commits found in PR")
    END IF

    // First commit to last commit in PR
    base_ref = pr_commits[0].sha
    head_ref = pr_commits[length(pr_commits) - 1].sha

    // Apply base ref override if specified
    IF config.base_ref IS NOT EMPTY THEN
        base_ref = config.base_ref
    END IF

    RETURN EventContext {
        event_type: EventType.PullRequest,
        repository: repository,
        base_ref: base_ref,
        head_ref: head_ref,
        commits: pr_commits,
        pull_request: Some(pull_request)
    }
}
```

### 5.4 Workflow Dispatch Event Parser

**References:** FR-1, Section 7.3

```pseudocode
FUNCTION parse_workflow_dispatch_event(event_json: JsonObject,
                                       repository: Repository) -> Result<EventContext> {
    /*
     * Parse workflow_dispatch event
     * No git range - scans entire repository history
     */

    RETURN EventContext {
        event_type: EventType.WorkflowDispatch,
        repository: repository,
        base_ref: "",  // Empty means full scan
        head_ref: "",
        commits: empty_list,
        pull_request: None
    }
}
```

### 5.5 Schedule Event Parser

**References:** FR-1, Section 7.4

```pseudocode
FUNCTION parse_schedule_event(event_json: JsonObject, repository: Repository,
                              config: Config) -> Result<EventContext> {
    /*
     * Parse schedule event
     * Similar to workflow_dispatch but handles undefined repository field
     */

    RETURN EventContext {
        event_type: EventType.Schedule,
        repository: repository,
        base_ref: "",  // Empty means full scan
        head_ref: "",
        commits: empty_list,
        pull_request: None
    }
}
```

### 5.6 Gitleaks Arguments Builder

**References:** FR-1, FR-2

```pseudocode
FUNCTION build_gitleaks_arguments(config: Config, event_context: EventContext) -> List<String> {
    /*
     * Build command-line arguments for gitleaks execution
     * Arguments vary by event type
     */

    args = [
        "detect",
        "--redact",
        "-v",
        "--exit-code=2",
        "--report-format=sarif",
        "--report-path=" + config.workspace_path + "/results.sarif",
        "--log-level=debug"
    ]

    // Add config file argument if present
    IF config.gitleaks_config_path IS NOT EMPTY THEN
        append(args, "--config=" + config.gitleaks_config_path)
    END IF

    // Add log-opts based on event type
    log_opts = build_log_opts(event_context)
    IF log_opts IS NOT EMPTY THEN
        append(args, "--log-opts=" + log_opts)
    END IF

    RETURN args
}

FUNCTION build_log_opts(event_context: EventContext) -> String {
    /*
     * Build --log-opts argument based on event type and git refs
     */

    MATCH event_context.event_type {
        CASE EventType.Push:
            IF event_context.base_ref == event_context.head_ref THEN
                // Single commit scan
                RETURN "-1"
            ELSE
                // Range scan
                RETURN "--no-merges --first-parent " +
                       event_context.base_ref + "^.." + event_context.head_ref
            END IF

        CASE EventType.PullRequest:
            // Always range scan for PRs
            RETURN "--no-merges --first-parent " +
                   event_context.base_ref + "^.." + event_context.head_ref

        CASE EventType.WorkflowDispatch:
            // Full repository scan - no log-opts
            RETURN ""

        CASE EventType.Schedule:
            // Full repository scan - no log-opts
            RETURN ""
    }
}
```

---

## 6. MODULE 3: BINARY MANAGEMENT

### 6.1 Binary Obtainer

**References:** FR-2

```pseudocode
FUNCTION obtain_gitleaks_binary(config: Config) -> Result<String> {
    /*
     * Download, cache, and prepare gitleaks binary
     * Returns path to executable binary
     */

    // Resolve version (may need API call for "latest")
    version = resolve_gitleaks_version(config.gitleaks_version)?

    // Detect platform and architecture
    platform = detect_platform()?
    architecture = detect_architecture()?

    // Check cache first
    cache_key = "gitleaks-cache-" + version + "-" + platform + "-" + architecture
    cached_path = check_cache(cache_key)

    IF cached_path IS NOT EMPTY AND file_exists(cached_path) THEN
        log_info("Using cached gitleaks binary: " + cached_path)
        RETURN cached_path
    END IF

    // Cache miss - download binary
    log_info("Downloading gitleaks version " + version + " for " + platform + "/" + architecture)

    download_url = build_download_url(version, platform, architecture)
    archive_path = download_file(download_url)?

    // Extract archive
    extract_dir = extract_archive(archive_path, platform)?
    binary_path = find_binary_in_directory(extract_dir, "gitleaks")?

    // Make executable (Unix-like systems)
    IF platform != "windows" THEN
        make_executable(binary_path)?
    END IF

    // Save to cache
    save_to_cache(cache_key, binary_path)

    // Add to PATH
    add_to_path(extract_dir)

    RETURN binary_path
}

FUNCTION resolve_gitleaks_version(version_input: String) -> Result<String> {
    /*
     * Resolve version string to actual version number
     * Handles special "latest" keyword
     */

    IF version_input == "latest" THEN
        // Fetch latest release from GitHub API
        latest_release = fetch_latest_gitleaks_release()?
        version = extract_version_from_tag(latest_release.tag_name)
        log_info("Resolved 'latest' to version: " + version)
        RETURN version
    ELSE
        // Use specified version directly
        RETURN version_input
    END IF
}

FUNCTION fetch_latest_gitleaks_release() -> Result<JsonObject> {
    /*
     * Call GitHub API to get latest gitleaks release
     */

    url = "https://api.github.com/repos/zricethezav/gitleaks/releases/latest"
    response = http_get(url)?

    IF response.status_code != 200 THEN
        RETURN Error("Failed to fetch latest release: HTTP " + response.status_code)
    END IF

    RETURN parse_json(response.body)
}

FUNCTION extract_version_from_tag(tag_name: String) -> String {
    /*
     * Extract version number from tag (e.g., "v8.24.3" -> "8.24.3")
     */

    IF starts_with(tag_name, "v") THEN
        RETURN substring(tag_name, 1)
    ELSE
        RETURN tag_name
    END IF
}

FUNCTION detect_platform() -> Result<String> {
    /*
     * Detect operating system platform
     * Maps to gitleaks release naming convention
     */

    os = get_operating_system()

    MATCH os {
        CASE "Linux":
            RETURN "linux"

        CASE "Darwin":
            RETURN "darwin"

        CASE "Windows":
            RETURN "windows"

        DEFAULT:
            RETURN Error("Unsupported platform: " + os)
    }
}

FUNCTION detect_architecture() -> Result<String> {
    /*
     * Detect CPU architecture
     * Maps to gitleaks release naming convention
     */

    arch = get_cpu_architecture()

    MATCH arch {
        CASE "x86_64", "amd64":
            RETURN "x64"

        CASE "aarch64", "arm64":
            RETURN "arm64"

        CASE "arm":
            RETURN "arm"

        DEFAULT:
            RETURN Error("Unsupported architecture: " + arch)
    }
}

FUNCTION build_download_url(version: String, platform: String, architecture: String) -> String {
    /*
     * Construct GitHub release download URL
     * Format: https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{platform}_{arch}.{ext}
     */

    base_url = "https://github.com/zricethezav/gitleaks/releases/download/v" + version
    filename = "gitleaks_" + version + "_" + platform + "_" + architecture

    IF platform == "windows" THEN
        extension = ".zip"
    ELSE
        extension = ".tar.gz"
    END IF

    RETURN base_url + "/" + filename + extension
}

FUNCTION extract_archive(archive_path: String, platform: String) -> Result<String> {
    /*
     * Extract downloaded archive
     * Returns directory containing extracted files
     */

    extract_dir = create_temp_directory("gitleaks-extract")?

    IF platform == "windows" THEN
        // Extract ZIP archive
        extract_zip(archive_path, extract_dir)?
    ELSE
        // Extract TAR.GZ archive
        extract_tar_gz(archive_path, extract_dir)?
    END IF

    log_debug("Extracted archive to: " + extract_dir)
    RETURN extract_dir
}

FUNCTION find_binary_in_directory(directory: String, binary_name: String) -> Result<String> {
    /*
     * Locate binary executable in extracted directory
     */

    files = list_files(directory)

    FOR EACH file IN files DO
        IF basename(file) == binary_name OR basename(file) == binary_name + ".exe" THEN
            RETURN file
        END IF
    END FOR

    RETURN Error("Binary not found in extracted archive: " + binary_name)
}
```

### 6.2 Gitleaks Executor

**References:** FR-2

```pseudocode
FUNCTION execute_gitleaks(binary_path: String, args: List<String>) -> Result<GitleaksExecutionResult> {
    /*
     * Execute gitleaks binary and capture output
     */

    // Build full command
    command = [binary_path] + args
    log_info("Executing: " + join(command, " "))

    // Execute process
    process = spawn_process(command)

    // Capture output
    stdout = read_stream(process.stdout)
    stderr = read_stream(process.stderr)

    // Wait for completion
    exit_code = wait_for_process(process)

    // Log output
    log_debug("Gitleaks stdout:\n" + stdout)
    log_debug("Gitleaks stderr:\n" + stderr)
    log_info("Gitleaks exit code: " + exit_code)

    RETURN GitleaksExecutionResult {
        exit_code: exit_code,
        stdout: stdout,
        stderr: stderr
    }
}
```

---

## 7. MODULE 4: SARIF PROCESSING

### 7.1 SARIF Parser

**References:** FR-3

```pseudocode
FUNCTION parse_sarif_report(sarif_path: String) -> Result<SarifReport> {
    /*
     * Parse SARIF v2 JSON file
     */

    IF NOT file_exists(sarif_path) THEN
        RETURN Error("SARIF report not found: " + sarif_path)
    END IF

    json_content = read_file(sarif_path)?
    sarif_data = parse_json(json_content)?

    // Validate SARIF structure
    IF NOT sarif_data.has_field("runs") THEN
        RETURN Error("Invalid SARIF: missing 'runs' field")
    END IF

    runs = sarif_data.get("runs")

    IF length(runs) == 0 THEN
        RETURN Error("Invalid SARIF: empty 'runs' array")
    END IF

    RETURN SarifReport {
        runs: parse_sarif_runs(runs)
    }
}

FUNCTION parse_sarif_runs(runs_json: JsonArray) -> List<SarifRun> {
    /*
     * Parse SARIF runs array
     */

    result = empty_list

    FOR EACH run_obj IN runs_json DO
        results = []

        IF run_obj.has_field("results") THEN
            results = parse_sarif_results(run_obj.get("results"))
        END IF

        run = SarifRun {
            results: results
        }
        append(result, run)
    END FOR

    RETURN result
}

FUNCTION parse_sarif_results(results_json: JsonArray) -> List<SarifResult> {
    /*
     * Parse SARIF results array
     */

    result = empty_list

    FOR EACH result_obj IN results_json DO
        // Extract required fields
        rule_id = result_obj.get("ruleId")
        message = result_obj.get("message").get("text")

        // Extract locations
        locations = []
        IF result_obj.has_field("locations") THEN
            locations = parse_sarif_locations(result_obj.get("locations"))
        END IF

        // Extract partial fingerprints (gitleaks extension)
        partial_fingerprints = PartialFingerprints {
            commit_sha: "",
            author: "",
            email: "",
            date: ""
        }

        IF result_obj.has_field("partialFingerprints") THEN
            fp_obj = result_obj.get("partialFingerprints")
            partial_fingerprints = PartialFingerprints {
                commit_sha: fp_obj.get_or_default("commitSha", ""),
                author: fp_obj.get_or_default("author", ""),
                email: fp_obj.get_or_default("email", ""),
                date: fp_obj.get_or_default("date", "")
            }
        END IF

        sarif_result = SarifResult {
            rule_id: rule_id,
            message: message,
            locations: locations,
            partial_fingerprints: partial_fingerprints
        }

        append(result, sarif_result)
    END FOR

    RETURN result
}

FUNCTION parse_sarif_locations(locations_json: JsonArray) -> List<SarifLocation> {
    /*
     * Parse SARIF locations array
     */

    result = empty_list

    FOR EACH location_obj IN locations_json DO
        IF location_obj.has_field("physicalLocation") THEN
            phys_loc = location_obj.get("physicalLocation")

            artifact_location = ArtifactLocation {
                uri: phys_loc.get("artifactLocation").get("uri")
            }

            region = Region {
                start_line: phys_loc.get("region").get("startLine")
            }

            physical_location = PhysicalLocation {
                artifact_location: artifact_location,
                region: region
            }

            location = SarifLocation {
                physical_location: physical_location
            }

            append(result, location)
        END IF
    END FOR

    RETURN result
}
```

### 7.2 Findings Extractor

**References:** FR-3

```pseudocode
FUNCTION extract_findings_from_sarif(sarif_report: SarifReport) -> Result<List<Finding>> {
    /*
     * Extract findings from SARIF report and generate fingerprints
     */

    findings = empty_list

    FOR EACH run IN sarif_report.runs DO
        FOR EACH result IN run.results DO
            // Extract location (use first location if multiple)
            IF length(result.locations) == 0 THEN
                log_warning("Skipping result with no locations: " + result.rule_id)
                CONTINUE
            END IF

            location = result.locations[0]
            file_path = location.physical_location.artifact_location.uri
            line_number = location.physical_location.region.start_line

            // Extract fingerprint metadata
            fp = result.partial_fingerprints

            // Generate fingerprint string
            fingerprint = generate_fingerprint(fp.commit_sha, file_path, result.rule_id, line_number)

            finding = Finding {
                rule_id: result.rule_id,
                file_path: file_path,
                line_number: line_number,
                commit_sha: fp.commit_sha,
                author: fp.author,
                email: fp.email,
                date: fp.date,
                fingerprint: fingerprint
            }

            append(findings, finding)
        END FOR
    END FOR

    log_info("Extracted " + length(findings) + " findings from SARIF report")
    RETURN findings
}

FUNCTION generate_fingerprint(commit_sha: String, file_path: String,
                              rule_id: String, line_number: Integer) -> String {
    /*
     * Generate fingerprint for .gitleaksignore file
     * Format: {commitSha}:{filePath}:{ruleId}:{startLine}
     */

    RETURN commit_sha + ":" + file_path + ":" + rule_id + ":" + to_string(line_number)
}
```

---

## 8. MODULE 5: PR COMMENT MANAGEMENT

### 8.1 Comment Poster

**References:** FR-4

```pseudocode
FUNCTION post_pr_comments(config: Config, event_context: EventContext,
                         findings: List<Finding>) -> Void {
    /*
     * Post inline review comments on pull request
     * Includes deduplication to prevent spam
     */

    IF event_context.pull_request IS None THEN
        log_error("Cannot post PR comments: not a pull request event")
        RETURN
    END IF

    pr = event_context.pull_request.unwrap()
    repository = event_context.repository

    // Fetch existing PR comments for deduplication
    existing_comments = TRY fetch_pr_review_comments(
        config.github_token,
        repository,
        pr.number
    ) CATCH error {
        log_warning("Failed to fetch existing comments: " + error.message)
        empty_list  // Continue with empty list
    }

    comments_posted = 0
    comments_skipped = 0

    FOR EACH finding IN findings DO
        // Build comment content
        comment_body = build_comment_body(finding, config.notify_user_list)

        // Check if duplicate comment exists
        IF is_duplicate_comment(existing_comments, finding, comment_body) THEN
            log_debug("Skipping duplicate comment for " + finding.file_path + ":" + finding.line_number)
            comments_skipped = comments_skipped + 1
            CONTINUE
        END IF

        // Create PR comment object
        pr_comment = PRComment {
            body: comment_body,
            commit_id: finding.commit_sha,
            path: finding.file_path,
            line: finding.line_number,
            side: "RIGHT"
        }

        // Post comment (non-fatal error handling)
        TRY {
            post_pr_review_comment(config.github_token, repository, pr.number, pr_comment)
            comments_posted = comments_posted + 1
            log_debug("Posted comment on " + finding.file_path + ":" + finding.line_number)
        } CATCH error {
            log_warning("Failed to post comment on " + finding.file_path + ":" +
                       finding.line_number + ": " + error.message)
            // Continue processing other comments
        }
    END FOR

    log_info("Posted " + comments_posted + " PR comments, skipped " +
             comments_skipped + " duplicates")
}

FUNCTION build_comment_body(finding: Finding, notify_users: List<String>) -> String {
    /*
     * Build comment body with emoji, rule ID, commit SHA, and fingerprint
     * Format from FR-4
     */

    body = "🛑 **Gitleaks Secret Detected**\n\n"
    body = body + "**Rule:** `" + finding.rule_id + "`\n"
    body = body + "**Commit:** `" + finding.commit_sha + "`\n"
    body = body + "**Fingerprint:** `" + finding.fingerprint + "`\n\n"
    body = body + "To ignore this finding, add the fingerprint to `.gitleaksignore`.\n"

    // Add user mentions if configured
    IF length(notify_users) > 0 THEN
        body = body + "\n**CC:** " + join(notify_users, " ") + "\n"
    END IF

    RETURN body
}

FUNCTION is_duplicate_comment(existing_comments: List<JsonObject>, finding: Finding,
                              new_body: String) -> Boolean {
    /*
     * Check if an identical comment already exists
     * Compares: body, path, line number
     */

    FOR EACH comment IN existing_comments DO
        // Extract comment fields
        existing_body = comment.get_or_default("body", "")
        existing_path = comment.get_or_default("path", "")
        existing_line = comment.get_or_default("line", 0)

        // Check for exact match
        IF existing_body == new_body AND
           existing_path == finding.file_path AND
           existing_line == finding.line_number THEN
            RETURN true
        END IF
    END FOR

    RETURN false
}
```

---

## 9. MODULE 6: JOB SUMMARY GENERATION

### 9.1 Summary Generator

**References:** FR-5

```pseudocode
FUNCTION generate_success_summary() -> Void {
    /*
     * Generate success summary when no secrets detected
     */

    summary = "## No leaks detected ✅\n"
    write_to_summary_file(summary)
    log_info("Generated success summary")
}

FUNCTION generate_findings_summary(repository: Repository, findings: List<Finding>) -> Void {
    /*
     * Generate detailed summary table when secrets detected
     */

    summary = "## 🛑 Gitleaks detected secrets 🛑\n\n"

    // Build HTML table
    summary = summary + "<table>\n"
    summary = summary + "<tr>\n"
    summary = summary + "  <th>Rule ID</th>\n"
    summary = summary + "  <th>Commit</th>\n"
    summary = summary + "  <th>Secret URL</th>\n"
    summary = summary + "  <th>Start Line</th>\n"
    summary = summary + "  <th>Author</th>\n"
    summary = summary + "  <th>Date</th>\n"
    summary = summary + "  <th>Email</th>\n"
    summary = summary + "  <th>File</th>\n"
    summary = summary + "</tr>\n"

    FOR EACH finding IN findings DO
        // Build URLs
        commit_url = repository.html_url + "/commit/" + finding.commit_sha
        secret_url = repository.html_url + "/blob/" + finding.commit_sha + "/" +
                    finding.file_path + "#L" + to_string(finding.line_number)
        file_url = repository.html_url + "/blob/" + finding.commit_sha + "/" + finding.file_path

        // Truncate commit SHA to 7 characters
        short_sha = substring(finding.commit_sha, 0, 7)

        summary = summary + "<tr>\n"
        summary = summary + "  <td>" + escape_html(finding.rule_id) + "</td>\n"
        summary = summary + "  <td><a href=\"" + commit_url + "\">" + short_sha + "</a></td>\n"
        summary = summary + "  <td><a href=\"" + secret_url + "\">View Secret</a></td>\n"
        summary = summary + "  <td>" + to_string(finding.line_number) + "</td>\n"
        summary = summary + "  <td>" + escape_html(finding.author) + "</td>\n"
        summary = summary + "  <td>" + escape_html(finding.date) + "</td>\n"
        summary = summary + "  <td>" + escape_html(finding.email) + "</td>\n"
        summary = summary + "  <td><a href=\"" + file_url + "\">" + escape_html(finding.file_path) + "</a></td>\n"
        summary = summary + "</tr>\n"
    END FOR

    summary = summary + "</table>\n"

    write_to_summary_file(summary)
    log_info("Generated findings summary with " + length(findings) + " entries")
}

FUNCTION generate_error_summary(exit_code: Integer) -> Void {
    /*
     * Generate error summary when gitleaks fails
     */

    summary = "## ❌ Gitleaks exited with error. Exit code [" + to_string(exit_code) + "]\n"
    write_to_summary_file(summary)
    log_info("Generated error summary")
}

FUNCTION write_to_summary_file(content: String) -> Void {
    /*
     * Write summary content to GITHUB_STEP_SUMMARY file
     * This is typically handled by JavaScript wrapper, but included for completeness
     */

    summary_path = get_env("GITHUB_STEP_SUMMARY")

    IF summary_path IS NOT EMPTY THEN
        append_to_file(summary_path, content)
        log_debug("Wrote summary to: " + summary_path)
    ELSE
        log_warning("GITHUB_STEP_SUMMARY not set, cannot write summary")
    END IF
}

FUNCTION escape_html(text: String) -> String {
    /*
     * Escape HTML special characters for safe rendering
     */

    result = text
    result = replace(result, "&", "&amp;")
    result = replace(result, "<", "&lt;")
    result = replace(result, ">", "&gt;")
    result = replace(result, "\"", "&quot;")
    result = replace(result, "'", "&#39;")
    RETURN result
}
```

---

## 10. MODULE 7: CONFIGURATION MANAGEMENT

### 10.1 Environment Variable Parser

**References:** FR-8, FR-9

This module is largely covered in Section 4.2 (Configuration Loading Helper). Additional helper functions:

```pseudocode
FUNCTION get_env(key: String) -> String {
    /*
     * Get environment variable value
     * Returns empty string if not set
     */

    value = read_environment_variable(key)

    IF value IS NULL THEN
        RETURN ""
    ELSE
        RETURN value
    END IF
}

FUNCTION get_env_or_default(key: String, default: String) -> String {
    /*
     * Get environment variable with fallback default
     */

    value = get_env(key)

    IF value IS EMPTY THEN
        RETURN default
    ELSE
        RETURN value
    END IF
}

FUNCTION validate_git_reference(git_ref: String) -> Result<String> {
    /*
     * Validate git reference (commit SHA, branch, tag)
     * Prevents injection attacks
     */

    IF is_empty(git_ref) THEN
        RETURN Error("Git reference cannot be empty")
    END IF

    // Check for shell metacharacters
    dangerous_chars = [";", "&", "|", "$", "`", "\n", "\r"]

    FOR EACH char IN dangerous_chars DO
        IF contains(git_ref, char) THEN
            RETURN Error("Invalid git reference: contains dangerous character '" + char + "'")
        END IF
    END FOR

    // Check for path traversal
    IF contains(git_ref, "..") THEN
        RETURN Error("Invalid git reference: contains path traversal")
    END IF

    RETURN git_ref
}

FUNCTION validate_file_path(file_path: String, workspace: String) -> Result<String> {
    /*
     * Validate file path to prevent directory traversal
     * Ensures path is within workspace
     */

    // Resolve absolute path
    absolute_path = resolve_absolute_path(file_path)

    // Check if path is within workspace
    IF NOT starts_with(absolute_path, workspace) THEN
        RETURN Error("File path outside workspace: " + file_path)
    END IF

    // Check for path traversal patterns
    IF contains(file_path, "..") THEN
        RETURN Error("File path contains path traversal: " + file_path)
    END IF

    RETURN absolute_path
}
```

---

## 11. MODULE 8: LICENSE VALIDATION

### 11.1 License Validator

**References:** FR-7, Section 8.3

```pseudocode
FUNCTION should_validate_license(config: Config, repository: Repository) -> Boolean {
    /*
     * Determine if license validation is required
     * Currently this feature is disabled but pseudocode is retained
     */

    // Feature currently disabled per specification Section 3.1, FR-7
    RETURN false

    /*
     * When re-enabled, use this logic:
     *
     * account_info = fetch_account_info(config.github_token, repository.owner)
     * RETURN account_info.account_type == AccountType.Organization
     */
}

FUNCTION validate_license(config: Config, repository: Repository) -> Result<Void> {
    /*
     * Validate license via Keygen.sh API
     * Feature currently disabled but logic retained for future use
     */

    // Check for license key
    IF config.gitleaks_license IS EMPTY THEN
        RETURN Error("GITLEAKS_LICENSE is required for organization accounts")
    END IF

    // Validate license key
    validation_result = validate_license_key(config.gitleaks_license, repository)?

    MATCH validation_result.validation_code {
        CASE "VALID":
            log_info("License validated successfully")
            RETURN Ok

        CASE "TOO_MANY_MACHINES":
            RETURN Error("License limit exceeded. Please contact support to increase machine limit.")

        CASE "NO_MACHINE", "NO_MACHINES", "FINGERPRINT_SCOPE_MISMATCH":
            // Attempt activation
            log_info("Attempting license activation for repository: " + repository.full_name)
            activate_license(config.gitleaks_license, repository)?
            RETURN Ok

        DEFAULT:
            RETURN Error("License validation failed: " + validation_result.validation_code +
                        " - " + validation_result.error_message)
    }
}

FUNCTION validate_license_key(license_key: String, repository: Repository) -> Result<ValidationResult> {
    /*
     * Call Keygen API to validate license key
     */

    url = "https://api.keygen.sh/v1/accounts/{account}/licenses/actions/validate-key"

    request_body = {
        "meta": {
            "key": license_key,
            "scope": {
                "fingerprint": repository.full_name
            }
        }
    }

    headers = {
        "Content-Type": "application/vnd.api+json"
    }

    response = http_post(url, request_body, headers)?

    IF response.status_code == 200 THEN
        data = parse_json(response.body)
        validation_code = data.get("meta").get("constant")

        RETURN ValidationResult {
            is_valid: validation_code == "VALID",
            validation_code: validation_code,
            error_message: None
        }
    ELSE
        RETURN Error("License validation request failed: HTTP " + response.status_code)
    END IF
}

FUNCTION activate_license(license_key: String, repository: Repository) -> Result<Void> {
    /*
     * Activate repository with license (associate machine)
     */

    url = "https://api.keygen.sh/v1/accounts/{account}/machines"

    request_body = {
        "data": {
            "type": "machines",
            "attributes": {
                "fingerprint": repository.full_name,
                "platform": "github-actions",
                "name": repository.full_name
            },
            "relationships": {
                "license": {
                    "data": {
                        "type": "licenses",
                        "id": "{license_id}"  // Extracted from license key validation
                    }
                }
            }
        }
    }

    headers = {
        "Authorization": "License " + license_key,
        "Content-Type": "application/vnd.api+json"
    }

    response = http_post(url, request_body, headers)?

    IF response.status_code == 201 THEN
        log_info("License activated successfully for " + repository.full_name)
        RETURN Ok
    ELSE
        RETURN Error("License activation failed: HTTP " + response.status_code)
    END IF
}
```

---

## 12. MODULE 9: GITHUB API INTEGRATION

### 12.1 Account Information Fetcher

**References:** Section 8.2

```pseudocode
FUNCTION fetch_account_info(github_token: String, username: String) -> Result<AccountInfo> {
    /*
     * Fetch GitHub account information to determine type (User vs Organization)
     */

    url = "https://api.github.com/users/" + username

    headers = {
        "Authorization": "token " + github_token,
        "Accept": "application/vnd.github.v3+json"
    }

    response = http_get_with_retry(url, headers)?

    IF response.status_code == 200 THEN
        data = parse_json(response.body)

        account_type = parse_account_type(data.get("type"))

        RETURN AccountInfo {
            account_type: account_type,
            login: data.get("login")
        }
    ELSE IF response.status_code == 404 THEN
        log_warning("User not found, assuming organization: " + username)
        RETURN AccountInfo {
            account_type: AccountType.Organization,
            login: username
        }
    ELSE
        RETURN Error("Failed to fetch account info: HTTP " + response.status_code)
    END IF
}

FUNCTION parse_account_type(type_string: String) -> AccountType {
    /*
     * Parse account type from GitHub API response
     */

    IF type_string == "Organization" THEN
        RETURN AccountType.Organization
    ELSE
        RETURN AccountType.User
    END IF
}
```

### 12.2 PR Commits Fetcher

**References:** FR-1, Section 8.2

```pseudocode
FUNCTION fetch_pr_commits(github_token: String, repository: Repository,
                         pr_number: Integer) -> Result<List<Commit>> {
    /*
     * Fetch all commits in a pull request
     */

    url = "https://api.github.com/repos/" + repository.full_name +
          "/pulls/" + to_string(pr_number) + "/commits"

    headers = {
        "Authorization": "token " + github_token,
        "Accept": "application/vnd.github.v3+json"
    }

    response = http_get_with_retry(url, headers)?

    IF response.status_code == 200 THEN
        commits_data = parse_json(response.body)
        RETURN parse_commits_response(commits_data)
    ELSE
        RETURN Error("Failed to fetch PR commits: HTTP " + response.status_code)
    END IF
}

FUNCTION parse_commits_response(commits_json: JsonArray) -> List<Commit> {
    /*
     * Parse commits from GitHub API response
     */

    result = empty_list

    FOR EACH commit_obj IN commits_json DO
        commit = Commit {
            sha: commit_obj.get("sha"),
            author: Author {
                name: commit_obj.get("commit").get("author").get("name"),
                email: commit_obj.get("commit").get("author").get("email")
            },
            message: commit_obj.get("commit").get("message")
        }
        append(result, commit)
    END FOR

    RETURN result
}
```

### 12.3 PR Review Comments

**References:** FR-4, Section 8.2

```pseudocode
FUNCTION fetch_pr_review_comments(github_token: String, repository: Repository,
                                 pr_number: Integer) -> Result<List<JsonObject>> {
    /*
     * Fetch existing PR review comments for deduplication
     */

    url = "https://api.github.com/repos/" + repository.full_name +
          "/pulls/" + to_string(pr_number) + "/comments"

    headers = {
        "Authorization": "token " + github_token,
        "Accept": "application/vnd.github.v3+json"
    }

    response = http_get_with_retry(url, headers)?

    IF response.status_code == 200 THEN
        RETURN parse_json(response.body)
    ELSE
        RETURN Error("Failed to fetch PR comments: HTTP " + response.status_code)
    END IF
}

FUNCTION post_pr_review_comment(github_token: String, repository: Repository,
                                pr_number: Integer, comment: PRComment) -> Result<Void> {
    /*
     * Post a single review comment on a pull request
     */

    url = "https://api.github.com/repos/" + repository.full_name +
          "/pulls/" + to_string(pr_number) + "/comments"

    headers = {
        "Authorization": "token " + github_token,
        "Accept": "application/vnd.github.v3+json",
        "Content-Type": "application/json"
    }

    request_body = {
        "body": comment.body,
        "commit_id": comment.commit_id,
        "path": comment.path,
        "line": comment.line,
        "side": comment.side
    }

    response = http_post_with_retry(url, request_body, headers)?

    IF response.status_code == 201 THEN
        RETURN Ok
    ELSE IF response.status_code == 422 THEN
        // Unprocessable entity - often due to diff limitations
        log_warning("Cannot comment on line (diff too large or file not in PR)")
        RETURN Ok  // Non-fatal error
    ELSE
        RETURN Error("Failed to post PR comment: HTTP " + response.status_code)
    END IF
}
```

### 12.4 HTTP Retry Logic

**References:** Section 8.2

```pseudocode
FUNCTION http_get_with_retry(url: String, headers: Map<String, String>) -> Result<HttpResponse> {
    /*
     * HTTP GET with exponential backoff retry logic
     */

    max_retries = 3
    base_delay = 1000  // milliseconds

    FOR attempt = 0 TO max_retries DO
        TRY {
            response = http_get(url, headers)

            // Success cases
            IF response.status_code >= 200 AND response.status_code < 300 THEN
                RETURN response
            END IF

            // Client error (no retry)
            IF response.status_code >= 400 AND response.status_code < 500 THEN
                IF response.status_code != 429 THEN  // Rate limit is retryable
                    RETURN response
                END IF
            END IF

            // Server error or rate limit - retry with backoff
            IF attempt < max_retries THEN
                delay = base_delay * (2 ^ attempt)
                log_warning("HTTP " + response.status_code + " - retrying in " + delay + "ms")
                sleep(delay)
            ELSE
                RETURN response
            END IF

        } CATCH error {
            IF attempt < max_retries THEN
                delay = base_delay * (2 ^ attempt)
                log_warning("Request failed: " + error.message + " - retrying in " + delay + "ms")
                sleep(delay)
            ELSE
                RETURN Error(error)
            END IF
        }
    END FOR

    RETURN Error("Max retries exceeded")
}

FUNCTION http_post_with_retry(url: String, body: JsonObject,
                              headers: Map<String, String>) -> Result<HttpResponse> {
    /*
     * HTTP POST with exponential backoff retry logic
     * Similar to http_get_with_retry but for POST requests
     */

    max_retries = 3
    base_delay = 1000

    FOR attempt = 0 TO max_retries DO
        TRY {
            response = http_post(url, body, headers)

            IF response.status_code >= 200 AND response.status_code < 300 THEN
                RETURN response
            END IF

            IF response.status_code >= 400 AND response.status_code < 500 THEN
                IF response.status_code != 429 THEN
                    RETURN response
                END IF
            END IF

            IF attempt < max_retries THEN
                delay = base_delay * (2 ^ attempt)
                log_warning("HTTP " + response.status_code + " - retrying in " + delay + "ms")
                sleep(delay)
            ELSE
                RETURN response
            END IF

        } CATCH error {
            IF attempt < max_retries THEN
                delay = base_delay * (2 ^ attempt)
                log_warning("Request failed: " + error.message + " - retrying in " + delay + "ms")
                sleep(delay)
            ELSE
                RETURN Error(error)
            END IF
        }
    END FOR

    RETURN Error("Max retries exceeded")
}
```

---

## 13. ERROR HANDLING STRATEGY

### 13.1 Error Categories

**References:** Section 7.6

```pseudocode
ENUMERATION ErrorSeverity {
    Fatal,      // Exit immediately with code 1
    NonFatal,   // Log warning and continue
    Expected    // Normal flow control (e.g., no commits)
}

STRUCTURE ApplicationError {
    severity: ErrorSeverity,
    message: String,
    context: Map<String, String>,
    underlying_error: Optional<Error>
}
```

### 13.2 Error Handling Patterns

```pseudocode
FUNCTION handle_error(error: ApplicationError) -> ExitCode {
    /*
     * Central error handling dispatcher
     */

    MATCH error.severity {
        CASE ErrorSeverity.Fatal:
            log_error("Fatal error: " + error.message)

            // Log context
            FOR EACH (key, value) IN error.context DO
                log_debug("  " + key + ": " + value)
            END FOR

            // Log underlying error
            IF error.underlying_error IS NOT None THEN
                log_debug("  Caused by: " + error.underlying_error.message)
            END IF

            RETURN ExitCode(1)

        CASE ErrorSeverity.NonFatal:
            log_warning("Non-fatal error: " + error.message)

            FOR EACH (key, value) IN error.context DO
                log_debug("  " + key + ": " + value)
            END FOR

            // Continue execution
            RETURN ExitCode(-1)  // Special code meaning "continue"

        CASE ErrorSeverity.Expected:
            log_info(error.message)
            RETURN ExitCode(0)  // Clean exit
    }
}
```

### 13.3 Specific Error Handlers

```pseudocode
FUNCTION create_fatal_error(message: String) -> ApplicationError {
    RETURN ApplicationError {
        severity: ErrorSeverity.Fatal,
        message: message,
        context: empty_map,
        underlying_error: None
    }
}

FUNCTION create_non_fatal_error(message: String) -> ApplicationError {
    RETURN ApplicationError {
        severity: ErrorSeverity.NonFatal,
        message: message,
        context: empty_map,
        underlying_error: None
    }
}

FUNCTION create_expected_error(message: String) -> ApplicationError {
    RETURN ApplicationError {
        severity: ErrorSeverity.Expected,
        message: message,
        context: empty_map,
        underlying_error: None
    }
}
```

---

## 14. CONTROL FLOW DIAGRAMS

### 14.1 Main Execution Flow

```
START
  │
  ├─> Load Configuration (FR-8, FR-9)
  │   ├─ Success ──> Continue
  │   └─ Error ──> EXIT 1 (Fatal)
  │
  ├─> Parse Event Context (FR-1)
  │   ├─ Success ──> Continue
  │   ├─ No Commits (Push) ──> EXIT 0 (Success)
  │   └─ Error ──> EXIT 1 (Fatal)
  │
  ├─> Validate License (FR-7) [If Required]
  │   ├─ Valid/Not Required ──> Continue
  │   └─ Invalid ──> EXIT 1 (Fatal)
  │
  ├─> Obtain Gitleaks Binary (FR-2)
  │   ├─ Cached ──> Use Cache
  │   ├─ Download ──> Extract ──> Continue
  │   └─ Error ──> EXIT 1 (Fatal)
  │
  ├─> Build Gitleaks Arguments (FR-1, FR-2)
  │   └─> Continue
  │
  ├─> Execute Gitleaks (FR-2)
  │   ├─ Exit 0 (No Secrets) ──┐
  │   ├─ Exit 2 (Secrets) ────┼──> Continue
  │   └─ Exit 1 (Error) ──────┘
  │
  ├─> Process Results
  │   │
  │   ├─ Exit 0: Generate Success Summary ──> EXIT 0
  │   │
  │   ├─ Exit 2:
  │   │   ├─> Parse SARIF (FR-3)
  │   │   │   ├─ Success ──> Continue
  │   │   │   └─ Error ──> EXIT 1 (Fatal)
  │   │   │
  │   │   ├─> Extract Findings (FR-3)
  │   │   │
  │   │   ├─> Post PR Comments (FR-4) [If PR Event & Enabled]
  │   │   │   ├─ Success ──> Continue
  │   │   │   └─ Error ──> Log Warning, Continue
  │   │   │
  │   │   ├─> Generate Findings Summary (FR-5) [If Enabled]
  │   │   │
  │   │   ├─> Prepare Artifact Upload (FR-6) [If Enabled]
  │   │   │
  │   │   └──> EXIT 1 (Secrets Found = Failure)
  │   │
  │   └─ Exit 1: Generate Error Summary ──> EXIT 1
  │
END
```

### 14.2 Event Routing Flow

```
Parse Event Context
  │
  ├─> Determine Event Type
  │
  ├─ PUSH Event
  │   ├─> Extract commits array
  │   ├─> Determine base_ref (first commit)
  │   ├─> Determine head_ref (last commit)
  │   ├─> Apply BASE_REF override if set
  │   ├─> Calculate log-opts:
  │   │   ├─ Single commit: --log-opts=-1
  │   │   └─ Range: --log-opts=--no-merges --first-parent base^..head
  │   └─> Return EventContext
  │
  ├─ PULL_REQUEST Event
  │   ├─> Extract PR number
  │   ├─> Fetch PR commits via GitHub API
  │   ├─> Determine base_ref (first PR commit)
  │   ├─> Determine head_ref (last PR commit)
  │   ├─> Apply BASE_REF override if set
  │   ├─> Calculate log-opts: --no-merges --first-parent base^..head
  │   └─> Return EventContext with PR info
  │
  ├─ WORKFLOW_DISPATCH Event
  │   ├─> No git range (full scan)
  │   ├─> Empty base_ref and head_ref
  │   ├─> No log-opts
  │   └─> Return EventContext
  │
  └─ SCHEDULE Event
      ├─> Handle undefined repository field
      ├─> Construct repository from env vars
      ├─> No git range (full scan)
      ├─> Empty base_ref and head_ref
      ├─> No log-opts
      └─> Return EventContext
```

### 14.3 Binary Management Flow

```
Obtain Gitleaks Binary
  │
  ├─> Resolve Version
  │   ├─ "latest" ──> Fetch from GitHub API ──> Extract version
  │   └─ Specific ──> Use as-is
  │
  ├─> Detect Platform & Architecture
  │   ├─ Linux/Darwin/Windows
  │   └─ x64/arm64/arm
  │
  ├─> Check Cache
  │   ├─ Cache Hit ──> Verify File Exists ──> Return Path
  │   └─ Cache Miss ──> Continue
  │
  ├─> Build Download URL
  │   └─ Format: https://github.com/.../gitleaks_{version}_{platform}_{arch}.{ext}
  │
  ├─> Download Archive
  │   ├─ Success ──> Continue
  │   └─ Error ──> EXIT 1 (Fatal)
  │
  ├─> Extract Archive
  │   ├─ Windows: Unzip
  │   └─ Unix: Untar
  │
  ├─> Find Binary in Extract Directory
  │
  ├─> Make Executable (Unix)
  │
  ├─> Save to Cache
  │
  ├─> Add to PATH
  │
  └─> Return Binary Path
```

### 14.4 SARIF Processing Flow

```
Parse SARIF Report
  │
  ├─> Read results.sarif File
  │   ├─ Exists ──> Continue
  │   └─ Missing ──> ERROR
  │
  ├─> Parse JSON
  │   ├─ Valid ──> Continue
  │   └─ Invalid ──> ERROR
  │
  ├─> Extract runs[0].results[]
  │
  ├─> For Each Result:
  │   │
  │   ├─> Extract rule_id
  │   ├─> Extract locations[0].physicalLocation
  │   │   ├─> file_path (artifactLocation.uri)
  │   │   └─> line_number (region.startLine)
  │   │
  │   ├─> Extract partialFingerprints
  │   │   ├─> commit_sha
  │   │   ├─> author
  │   │   ├─> email
  │   │   └─> date
  │   │
  │   ├─> Generate Fingerprint
  │   │   └─> Format: {commit}:{file}:{rule}:{line}
  │   │
  │   └─> Create Finding Object
  │
  └─> Return List<Finding>
```

### 14.5 PR Comment Flow

```
Post PR Comments
  │
  ├─> Verify Event Type is PullRequest
  │   ├─ Yes ──> Continue
  │   └─ No ──> Log Error, Return
  │
  ├─> Fetch Existing PR Comments (for deduplication)
  │   ├─ Success ──> Continue
  │   └─ Error ──> Log Warning, Use Empty List
  │
  ├─> For Each Finding:
  │   │
  │   ├─> Build Comment Body
  │   │   ├─ Emoji: 🛑
  │   │   ├─ Rule ID
  │   │   ├─ Commit SHA
  │   │   ├─ Fingerprint
  │   │   └─ User Mentions (if configured)
  │   │
  │   ├─> Check Duplicate
  │   │   └─ Compare: body, path, line
  │   │       ├─ Duplicate ──> Skip, Continue to Next
  │   │       └─ New ──> Continue
  │   │
  │   ├─> Create PRComment Object
  │   │   ├─ body
  │   │   ├─ commit_id
  │   │   ├─ path
  │   │   ├─ line
  │   │   └─ side: "RIGHT"
  │   │
  │   └─> Post Comment via GitHub API
  │       ├─ Success ──> Log, Continue
  │       └─ Error ──> Log Warning, Continue (Non-Fatal)
  │
  └─> Log Summary (posted count, skipped count)
```

### 14.6 License Validation Flow (When Enabled)

```
Validate License
  │
  ├─> Check Account Type
  │   ├─> Fetch from GitHub API
  │   │   ├─ Organization ──> Require License
  │   │   ├─ User ──> Skip Validation
  │   │   └─ Unknown ──> Assume Organization
  │
  ├─> Check GITLEAKS_LICENSE Env Var
  │   ├─ Present ──> Continue
  │   └─ Missing ──> EXIT 1 (Fatal)
  │
  ├─> Call Keygen Validate API
  │   ├─ VALID ──> Success, Continue
  │   │
  │   ├─ TOO_MANY_MACHINES ──> EXIT 1 (Fatal)
  │   │
  │   ├─ NO_MACHINE / FINGERPRINT_MISMATCH
  │   │   └─> Attempt Activation
  │   │       ├─> Call Keygen Machines API
  │   │       │   ├─ Success (201) ──> Continue
  │   │       │   └─ Error ──> EXIT 1 (Fatal)
  │   │
  │   └─ Other Error ──> EXIT 1 (Fatal)
  │
  └─> Return Success
```

---

## 15. CONCLUSION

### 15.1 Pseudocode Coverage

This pseudocode document provides comprehensive algorithmic representations for all modules of the SecretScout project:

✅ **Module 1: Main Orchestrator** - Entry point and execution coordination
✅ **Module 2: Event Routing** - All four event types (push, PR, workflow_dispatch, schedule)
✅ **Module 3: Binary Management** - Download, cache, execute gitleaks
✅ **Module 4: SARIF Processing** - Parse SARIF, extract findings, generate fingerprints
✅ **Module 5: PR Comment Management** - Post inline comments with deduplication
✅ **Module 6: Job Summary Generation** - Success, findings, and error summaries
✅ **Module 7: Configuration Management** - Environment variable parsing and validation
✅ **Module 8: License Validation** - Keygen.sh integration (disabled but retained)
✅ **Module 9: GitHub API Integration** - All API endpoints and retry logic
✅ **Error Handling Strategy** - Fatal, non-fatal, and expected error flows
✅ **Control Flow Diagrams** - Text-based diagrams for all major flows

### 15.2 Specification Cross-References

All pseudocode sections include explicit references to the specification:

- **FR-1**: Event Type Support - Covered in Section 5
- **FR-2**: Binary Management - Covered in Section 6
- **FR-3**: SARIF Processing - Covered in Section 7
- **FR-4**: PR Comment Creation - Covered in Section 8
- **FR-5**: Job Summary Generation - Covered in Section 9
- **FR-6**: Artifact Upload - Handled by JS wrapper (noted in Section 4)
- **FR-7**: License Validation - Covered in Section 11
- **FR-8**: Environment Variables - Covered in Sections 4.2 and 10
- **FR-9**: Configuration Files - Covered in Sections 4.2 and 10

### 15.3 Implementation Guidance

This pseudocode provides:

1. **Clear function signatures** with input/output types
2. **Control flow** using standard structures (IF/ELSE, MATCH, FOR, TRY/CATCH)
3. **Error handling paths** for all operations
4. **Data structures** for all key entities
5. **Algorithmic logic** without language-specific syntax
6. **Cross-module dependencies** clearly identified

The pseudocode is ready to be translated into Rust implementation during the Architecture and Completion phases.

### 15.4 Next Steps (Not Part of This Document)

The following phases will follow this pseudocode:

1. **Architecture Phase**: Detailed system design, component interactions, crate structure
2. **Refinement Phase**: Optimization, error handling improvements, performance tuning
3. **Completion Phase**: Actual Rust implementation, testing, deployment

---

**PSEUDOCODE PHASE STATUS:** ✅ COMPLETE

**Document Version:** 1.0
**Date:** October 16, 2025
**Coordinator:** Claude Code Swarm (SPARC Methodology)
