# Configuration Management Pseudocode

**Module:** Configuration Management
**Specification Reference:** SPARC_SPECIFICATION.md Section 3.2 (FR-8, FR-9)
**Purpose:** Environment variable parsing, configuration file discovery, and validation
**Date:** 2025-10-16

---

## Table of Contents

1. [Overview](#overview)
2. [Data Structures](#data-structures)
3. [Core Algorithms](#core-algorithms)
4. [Validation Functions](#validation-functions)
5. [Helper Functions](#helper-functions)
6. [Error Handling](#error-handling)
7. [Integration Points](#integration-points)

---

## Overview

The Configuration Management module is responsible for:
- Loading and parsing environment variables (FR-8)
- Discovering and validating configuration files (FR-9)
- Providing type-safe access to configuration values
- Implementing special boolean parsing logic
- Validating paths and security constraints
- Managing default values

### Key Behaviors

**Boolean Parsing (CRITICAL):**
- False values: "false", "0" (case-insensitive)
- True values: All other values including "true", "1", empty string, undefined
- This matches the original JavaScript implementation behavior

**Configuration File Priority:**
1. GITLEAKS_CONFIG environment variable (explicit path)
2. gitleaks.toml in repository root (auto-detect)
3. No configuration (use gitleaks built-in defaults)

---

## Data Structures

### Configuration

```rust
STRUCTURE Configuration:
    // Required (conditional)
    github_token: Option<String>           // Required for PR events
    gitleaks_license: Option<String>       // Required for organizations

    // Optional with defaults
    gitleaks_version: String               // Default: "8.24.3"
    gitleaks_config: Option<PathBuf>       // Default: Auto-detect or None

    // Feature toggles (default: true)
    enable_summary: Boolean                // Default: true
    enable_upload_artifact: Boolean        // Default: true
    enable_comments: Boolean               // Default: true

    // Optional configuration
    notify_user_list: Option<Vec<String>>  // Parsed from comma-separated string
    base_ref: Option<String>               // Override for base git ref

    // Environment context
    github_workspace: PathBuf              // GITHUB_WORKSPACE
    github_event_path: PathBuf             // GITHUB_EVENT_PATH
    github_event_name: String              // GITHUB_EVENT_NAME
    github_repository: String              // GITHUB_REPOSITORY
    github_repository_owner: String        // GITHUB_REPOSITORY_OWNER
END STRUCTURE


STRUCTURE ConfigurationError:
    error_type: ErrorType
    message: String
    field_name: Option<String>
END STRUCTURE

ENUM ErrorType:
    MISSING_REQUIRED_VAR
    INVALID_PATH
    INVALID_VALUE
    SECURITY_VIOLATION
    IO_ERROR
END ENUM
```

---

## Core Algorithms

### LoadConfiguration

**Purpose:** Main entry point for loading all configuration from environment variables

**Specification Reference:** FR-8, FR-9

```pseudocode
FUNCTION LoadConfiguration() -> Result<Configuration, ConfigurationError>:
    INPUT: None (reads from environment)
    OUTPUT: Configuration object or error

    BEGIN
        // Step 1: Load required context variables
        TRY:
            workspace = GetRequiredEnvVar("GITHUB_WORKSPACE")
            event_path = GetRequiredEnvVar("GITHUB_EVENT_PATH")
            event_name = GetRequiredEnvVar("GITHUB_EVENT_NAME")
            repository = GetRequiredEnvVar("GITHUB_REPOSITORY")
            repository_owner = GetRequiredEnvVar("GITHUB_REPOSITORY_OWNER")
        CATCH error:
            RETURN Error(MISSING_REQUIRED_VAR, error.message)
        END TRY

        // Step 2: Validate workspace path exists
        workspace_path = ParsePath(workspace)
        IF NOT PathExists(workspace_path):
            RETURN Error(INVALID_PATH, "GITHUB_WORKSPACE does not exist: " + workspace)
        END IF

        // Step 3: Validate event path exists
        event_path_obj = ParsePath(event_path)
        IF NOT PathExists(event_path_obj):
            RETURN Error(INVALID_PATH, "GITHUB_EVENT_PATH does not exist: " + event_path)
        END IF

        // Step 4: Load optional variables with defaults
        gitleaks_version = GetOptionalEnvVar("GITLEAKS_VERSION", "8.24.3")

        // Step 5: Parse feature toggles (special boolean logic)
        enable_summary = ParseBooleanValue(
            GetOptionalEnvVar("GITLEAKS_ENABLE_SUMMARY", "true")
        )
        enable_upload_artifact = ParseBooleanValue(
            GetOptionalEnvVar("GITLEAKS_ENABLE_UPLOAD_ARTIFACT", "true")
        )
        enable_comments = ParseBooleanValue(
            GetOptionalEnvVar("GITLEAKS_ENABLE_COMMENTS", "true")
        )

        // Step 6: Load conditional variables
        github_token = GetOptionalEnvVar("GITHUB_TOKEN", NULL)
        gitleaks_license = GetOptionalEnvVar("GITLEAKS_LICENSE", NULL)

        // Step 7: Parse user notification list
        notify_list_raw = GetOptionalEnvVar("GITLEAKS_NOTIFY_USER_LIST", NULL)
        notify_user_list = NULL
        IF notify_list_raw IS NOT NULL AND notify_list_raw IS NOT EMPTY:
            notify_user_list = ParseNotifyUserList(notify_list_raw)
        END IF

        // Step 8: Discover configuration file
        gitleaks_config = DiscoverConfigFile(workspace_path)

        // Step 9: Get base ref override
        base_ref = GetOptionalEnvVar("BASE_REF", NULL)

        // Step 10: Construct configuration object
        config = Configuration{
            github_token: github_token,
            gitleaks_license: gitleaks_license,
            gitleaks_version: gitleaks_version,
            gitleaks_config: gitleaks_config,
            enable_summary: enable_summary,
            enable_upload_artifact: enable_upload_artifact,
            enable_comments: enable_comments,
            notify_user_list: notify_user_list,
            base_ref: base_ref,
            github_workspace: workspace_path,
            github_event_path: event_path_obj,
            github_event_name: event_name,
            github_repository: repository,
            github_repository_owner: repository_owner
        }

        // Step 11: Validate configuration consistency
        validation_result = ValidateConfiguration(config)
        IF validation_result IS Error:
            RETURN validation_result
        END IF

        RETURN Success(config)
    END
END FUNCTION
```

**Complexity:** O(n) where n is number of environment variables
**Error Cases:**
- Missing required environment variables
- Invalid path values
- Path traversal attempts
- Non-existent paths

---

### ParseEnvironmentVariables

**Purpose:** Parse all environment variables into structured format

**Specification Reference:** FR-8

```pseudocode
FUNCTION ParseEnvironmentVariables() -> Result<Map<String, String>, ConfigurationError>:
    INPUT: None (reads from system environment)
    OUTPUT: Map of environment variable names to values

    BEGIN
        env_vars = EmptyMap()

        // Define all recognized environment variables
        var_names = [
            "GITHUB_TOKEN",
            "GITHUB_WORKSPACE",
            "GITHUB_EVENT_PATH",
            "GITHUB_EVENT_NAME",
            "GITHUB_REPOSITORY",
            "GITHUB_REPOSITORY_OWNER",
            "GITLEAKS_LICENSE",
            "GITLEAKS_VERSION",
            "GITLEAKS_CONFIG",
            "GITLEAKS_ENABLE_SUMMARY",
            "GITLEAKS_ENABLE_UPLOAD_ARTIFACT",
            "GITLEAKS_ENABLE_COMMENTS",
            "GITLEAKS_NOTIFY_USER_LIST",
            "BASE_REF"
        ]

        // Read each variable from environment
        FOR EACH var_name IN var_names:
            value = GetEnvVariable(var_name)
            IF value IS NOT NULL:
                env_vars[var_name] = value
            END IF
        END FOR

        RETURN Success(env_vars)
    END
END FUNCTION
```

**Complexity:** O(n) where n is number of variables
**Error Cases:** None (returns empty map if no variables found)

---

### ParseBooleanValue

**Purpose:** Parse boolean values with special logic matching original implementation

**Specification Reference:** FR-8 (Boolean Parsing section)

**CRITICAL:** This function implements the exact boolean parsing logic from the original JavaScript implementation:
- "false" (any case) → false
- "0" → false
- All other values → true (including "true", "1", empty string, undefined)

```pseudocode
FUNCTION ParseBooleanValue(value: String) -> Boolean:
    INPUT: value - String value to parse as boolean
    OUTPUT: Boolean value

    BEGIN
        // Handle NULL/undefined (treated as true by default)
        IF value IS NULL:
            RETURN true
        END IF

        // Trim whitespace
        trimmed = Trim(value)

        // Convert to lowercase for comparison
        lower = ToLowerCase(trimmed)

        // Check for explicit false values
        IF lower == "false":
            RETURN false
        END IF

        IF lower == "0":
            RETURN false
        END IF

        // All other values are true
        // This includes:
        // - "true"
        // - "1"
        // - Empty string ""
        // - Any other non-empty string
        RETURN true
    END
END FUNCTION
```

**Examples:**
```
ParseBooleanValue("false") → false
ParseBooleanValue("FALSE") → false
ParseBooleanValue("0") → false
ParseBooleanValue("true") → true
ParseBooleanValue("TRUE") → true
ParseBooleanValue("1") → true
ParseBooleanValue("") → true
ParseBooleanValue("yes") → true
ParseBooleanValue("anything") → true
ParseBooleanValue(NULL) → true
```

**Complexity:** O(1)
**Error Cases:** None (always returns boolean)

---

### DiscoverConfigFile

**Purpose:** Discover gitleaks configuration file with priority order

**Specification Reference:** FR-9

**Priority Order:**
1. GITLEAKS_CONFIG environment variable (explicit path)
2. gitleaks.toml in repository root (auto-detect)
3. None (use gitleaks built-in defaults)

```pseudocode
FUNCTION DiscoverConfigFile(workspace: PathBuf) -> Option<PathBuf>:
    INPUT: workspace - GITHUB_WORKSPACE path
    OUTPUT: Path to configuration file or NULL

    BEGIN
        // Step 1: Check explicit GITLEAKS_CONFIG environment variable
        explicit_config = GetOptionalEnvVar("GITLEAKS_CONFIG", NULL)

        IF explicit_config IS NOT NULL:
            // User provided explicit path
            config_path = ResolveConfigPath(workspace, explicit_config)

            // Validate the path
            validation_result = ValidatePath(config_path, workspace)
            IF validation_result IS Error:
                LOG_WARNING("GITLEAKS_CONFIG path invalid: " + validation_result.message)
                LOG_WARNING("Falling back to auto-detection")
                // Continue to auto-detection
            ELSE:
                // Verify file exists and is readable
                IF PathExists(config_path) AND IsReadable(config_path):
                    LOG_INFO("Using explicit config: " + config_path)
                    RETURN Some(config_path)
                ELSE:
                    LOG_WARNING("GITLEAKS_CONFIG file not found or not readable: " + config_path)
                    LOG_WARNING("Falling back to auto-detection")
                    // Continue to auto-detection
                END IF
            END IF
        END IF

        // Step 2: Auto-detect gitleaks.toml in repository root
        auto_detect_path = JoinPath(workspace, "gitleaks.toml")

        IF PathExists(auto_detect_path) AND IsReadable(auto_detect_path):
            LOG_INFO("Auto-detected config: " + auto_detect_path)
            RETURN Some(auto_detect_path)
        END IF

        // Step 3: No configuration found - use gitleaks defaults
        LOG_INFO("No gitleaks config found, using gitleaks built-in defaults")
        RETURN None
    END
END FUNCTION
```

**Complexity:** O(1) - constant number of file system checks
**Error Cases:**
- Invalid paths logged as warnings
- Missing files logged as info
- No errors thrown (graceful fallback)

---

### ValidatePath

**Purpose:** Validate path for security and correctness

**Specification Reference:** Section 10.1 (Input Validation)

```pseudocode
FUNCTION ValidatePath(path: PathBuf, workspace: PathBuf) -> Result<(), ConfigurationError>:
    INPUT:
        path - Path to validate
        workspace - GITHUB_WORKSPACE (allowed root)
    OUTPUT: Success or error

    BEGIN
        // Step 1: Check for path traversal attack
        path_str = ToString(path)
        IF Contains(path_str, ".."):
            RETURN Error(
                SECURITY_VIOLATION,
                "Path contains '..' (path traversal): " + path_str
            )
        END IF

        // Step 2: Resolve to absolute path
        TRY:
            absolute_path = CanonicalPath(path)
        CATCH error:
            RETURN Error(
                INVALID_PATH,
                "Cannot resolve path: " + path_str + " - " + error.message
            )
        END TRY

        // Step 3: Ensure path is within workspace
        workspace_absolute = CanonicalPath(workspace)

        IF NOT StartsWith(absolute_path, workspace_absolute):
            RETURN Error(
                SECURITY_VIOLATION,
                "Path is outside GITHUB_WORKSPACE: " + path_str
            )
        END IF

        // Step 4: Check path exists
        IF NOT PathExists(absolute_path):
            RETURN Error(
                INVALID_PATH,
                "Path does not exist: " + path_str
            )
        END IF

        // Step 5: Check path is readable
        IF NOT IsReadable(absolute_path):
            RETURN Error(
                INVALID_PATH,
                "Path is not readable: " + path_str
            )
        END IF

        RETURN Success()
    END
END FUNCTION
```

**Security Checks:**
1. Path traversal (.. sequences)
2. Absolute path resolution
3. Workspace boundary enforcement
4. Existence verification
5. Permission verification

**Complexity:** O(1) - constant number of checks
**Error Cases:**
- Path traversal attempts
- Paths outside workspace
- Non-existent paths
- Unreadable paths

---

### GetRequiredEnvVar

**Purpose:** Get required environment variable or fail

**Specification Reference:** FR-8

```pseudocode
FUNCTION GetRequiredEnvVar(name: String) -> Result<String, ConfigurationError>:
    INPUT: name - Environment variable name
    OUTPUT: Variable value or error

    BEGIN
        value = GetEnvVariable(name)

        IF value IS NULL:
            RETURN Error(
                MISSING_REQUIRED_VAR,
                "Required environment variable not set: " + name
            )
        END IF

        // Trim whitespace
        trimmed = Trim(value)

        IF IsEmpty(trimmed):
            RETURN Error(
                MISSING_REQUIRED_VAR,
                "Required environment variable is empty: " + name
            )
        END IF

        RETURN Success(trimmed)
    END
END FUNCTION
```

**Complexity:** O(1)
**Error Cases:**
- Variable not set
- Variable is empty or whitespace only

---

### GetOptionalEnvVar

**Purpose:** Get optional environment variable with default value

**Specification Reference:** FR-8

```pseudocode
FUNCTION GetOptionalEnvVar(name: String, default: String) -> String:
    INPUT:
        name - Environment variable name
        default - Default value if not set (may be NULL)
    OUTPUT: Variable value or default

    BEGIN
        value = GetEnvVariable(name)

        IF value IS NULL:
            RETURN default
        END IF

        // Trim whitespace
        trimmed = Trim(value)

        // If empty after trimming, return default
        IF IsEmpty(trimmed):
            RETURN default
        END IF

        RETURN trimmed
    END
END FUNCTION
```

**Complexity:** O(1)
**Error Cases:** None (returns default)

---

## Validation Functions

### ValidateConfiguration

**Purpose:** Validate configuration object for consistency and requirements

**Specification Reference:** FR-8

```pseudocode
FUNCTION ValidateConfiguration(config: Configuration) -> Result<(), ConfigurationError>:
    INPUT: config - Configuration object to validate
    OUTPUT: Success or error

    BEGIN
        // Step 1: Validate event-specific requirements
        IF config.github_event_name == "pull_request":
            // PR events require GITHUB_TOKEN
            IF config.github_token IS NULL:
                RETURN Error(
                    MISSING_REQUIRED_VAR,
                    "GITHUB_TOKEN is required for pull_request events"
                )
            END IF
        END IF

        // Step 2: Validate gitleaks version format
        IF NOT IsValidVersionString(config.gitleaks_version):
            RETURN Error(
                INVALID_VALUE,
                "Invalid GITLEAKS_VERSION format: " + config.gitleaks_version
            )
        END IF

        // Step 3: Validate repository format (owner/repo)
        IF NOT Contains(config.github_repository, "/"):
            RETURN Error(
                INVALID_VALUE,
                "Invalid GITHUB_REPOSITORY format (expected owner/repo): " +
                config.github_repository
            )
        END IF

        // Step 4: Validate event name
        valid_events = ["push", "pull_request", "workflow_dispatch", "schedule"]
        IF config.github_event_name NOT IN valid_events:
            RETURN Error(
                INVALID_VALUE,
                "Unsupported event type: " + config.github_event_name
            )
        END IF

        // Step 5: Validate notify user list format (if present)
        IF config.notify_user_list IS NOT NULL:
            FOR EACH user IN config.notify_user_list:
                IF NOT StartsWith(user, "@"):
                    RETURN Error(
                        INVALID_VALUE,
                        "User in notify list must start with @: " + user
                    )
                END IF
            END FOR
        END IF

        RETURN Success()
    END
END FUNCTION
```

**Validation Rules:**
1. GITHUB_TOKEN required for PR events
2. Version string format valid
3. Repository format is "owner/repo"
4. Event name is supported
5. User notifications start with @

**Complexity:** O(n) where n is number of users in notify list
**Error Cases:**
- Missing required fields
- Invalid format values
- Unsupported event types

---

### IsValidVersionString

**Purpose:** Validate gitleaks version string format

**Specification Reference:** FR-8

```pseudocode
FUNCTION IsValidVersionString(version: String) -> Boolean:
    INPUT: version - Version string to validate
    OUTPUT: True if valid, false otherwise

    BEGIN
        // Special case: "latest" is always valid
        IF version == "latest":
            RETURN true
        END IF

        // Semantic versioning format: X.Y.Z
        // Also accept vX.Y.Z format

        // Remove leading 'v' if present
        clean_version = version
        IF StartsWith(version, "v"):
            clean_version = Substring(version, 1)
        END IF

        // Split by dots
        parts = Split(clean_version, ".")

        // Must have exactly 3 parts (major.minor.patch)
        IF Length(parts) != 3:
            RETURN false
        END IF

        // Each part must be a non-negative integer
        FOR EACH part IN parts:
            IF NOT IsNumeric(part):
                RETURN false
            END IF

            number = ParseInteger(part)
            IF number < 0:
                RETURN false
            END IF
        END FOR

        RETURN true
    END
END FUNCTION
```

**Valid Examples:**
- "8.24.3"
- "v8.24.3"
- "latest"
- "8.0.0"

**Invalid Examples:**
- "8.24" (missing patch)
- "v8" (incomplete)
- "abc" (not numeric)
- "-1.0.0" (negative)

**Complexity:** O(1) - constant length version string
**Error Cases:** None (returns boolean)

---

## Helper Functions

### ResolveConfigPath

**Purpose:** Resolve configuration path relative to workspace

**Specification Reference:** FR-9

```pseudocode
FUNCTION ResolveConfigPath(workspace: PathBuf, config_path: String) -> PathBuf:
    INPUT:
        workspace - GITHUB_WORKSPACE path
        config_path - Configuration file path (absolute or relative)
    OUTPUT: Resolved absolute path

    BEGIN
        // Check if path is already absolute
        IF IsAbsolutePath(config_path):
            RETURN ParsePath(config_path)
        END IF

        // Path is relative - resolve relative to workspace
        RETURN JoinPath(workspace, config_path)
    END
END FUNCTION
```

**Complexity:** O(1)
**Error Cases:** None (pure path resolution)

---

### ParseNotifyUserList

**Purpose:** Parse comma-separated user notification list

**Specification Reference:** FR-8

```pseudocode
FUNCTION ParseNotifyUserList(user_list: String) -> Vec<String>:
    INPUT: user_list - Comma-separated list of GitHub usernames
    OUTPUT: Vector of username strings

    BEGIN
        // Handle empty or whitespace-only string
        trimmed = Trim(user_list)
        IF IsEmpty(trimmed):
            RETURN EmptyVector()
        END IF

        // Split by comma
        raw_users = Split(trimmed, ",")

        users = EmptyVector()
        FOR EACH raw_user IN raw_users:
            // Trim whitespace from each user
            user = Trim(raw_user)

            // Skip empty entries
            IF IsEmpty(user):
                CONTINUE
            END IF

            // Ensure user starts with @ (add if missing)
            IF NOT StartsWith(user, "@"):
                user = "@" + user
            END IF

            // Add to list (deduplicate)
            IF user NOT IN users:
                Append(users, user)
            END IF
        END FOR

        RETURN users
    END
END FUNCTION
```

**Examples:**
```
ParseNotifyUserList("@user1,@user2,@user3") → ["@user1", "@user2", "@user3"]
ParseNotifyUserList("user1, user2, user3") → ["@user1", "@user2", "@user3"]
ParseNotifyUserList("@user1,,@user2") → ["@user1", "@user2"]
ParseNotifyUserList("") → []
ParseNotifyUserList("@user1,@user1") → ["@user1"] (deduplicated)
```

**Complexity:** O(n) where n is number of users
**Error Cases:** None (graceful handling)

---

### GetEnvVariable

**Purpose:** Low-level environment variable access

```pseudocode
FUNCTION GetEnvVariable(name: String) -> Option<String>:
    INPUT: name - Environment variable name
    OUTPUT: Variable value or NULL if not set

    BEGIN
        // Platform-specific environment variable access
        // In Rust: std::env::var(name).ok()
        // In JavaScript: process.env[name]

        value = SystemGetEnv(name)
        RETURN value
    END
END FUNCTION
```

**Complexity:** O(1)
**Error Cases:** None (returns NULL if not set)

---

## Error Handling

### Error Reporting Strategy

```pseudocode
FUNCTION ReportConfigurationError(error: ConfigurationError) -> String:
    INPUT: error - Configuration error object
    OUTPUT: Formatted error message

    BEGIN
        MATCH error.error_type:
            CASE MISSING_REQUIRED_VAR:
                message = "Configuration Error: " + error.message
                message = message + "\n\nRequired environment variables:"
                message = message + "\n  - GITHUB_TOKEN (for PR events)"
                message = message + "\n  - GITHUB_WORKSPACE"
                message = message + "\n  - GITHUB_EVENT_PATH"
                message = message + "\n  - GITHUB_EVENT_NAME"
                message = message + "\n  - GITHUB_REPOSITORY"
                message = message + "\n  - GITHUB_REPOSITORY_OWNER"
                RETURN message

            CASE INVALID_PATH:
                message = "Configuration Error: " + error.message
                IF error.field_name IS NOT NULL:
                    message = message + "\nField: " + error.field_name
                END IF
                message = message + "\n\nEnsure all paths are:"
                message = message + "\n  - Within GITHUB_WORKSPACE"
                message = message + "\n  - Readable by the action"
                message = message + "\n  - Without '..' path traversal"
                RETURN message

            CASE INVALID_VALUE:
                message = "Configuration Error: " + error.message
                IF error.field_name IS NOT NULL:
                    message = message + "\nField: " + error.field_name
                END IF
                RETURN message

            CASE SECURITY_VIOLATION:
                message = "SECURITY ERROR: " + error.message
                message = message + "\n\nThis action has blocked a potential security issue."
                message = message + "\nPlease review your configuration."
                RETURN message

            CASE IO_ERROR:
                message = "I/O Error: " + error.message
                RETURN message
        END MATCH
    END
END FUNCTION
```

### Error Recovery Strategies

```pseudocode
// Strategy 1: Graceful Fallback for Optional Configuration
IF DiscoverConfigFile() fails:
    LOG_WARNING("Config discovery failed, using defaults")
    CONTINUE with NULL config

// Strategy 2: Fail Fast for Required Configuration
IF GetRequiredEnvVar() fails:
    ReportConfigurationError()
    EXIT with code 1

// Strategy 3: Validation with Context
IF ValidatePath() fails:
    LOG_WARNING("Path validation failed: " + path)
    IF path is GITLEAKS_CONFIG:
        LOG_WARNING("Falling back to auto-detection")
        TRY auto-detection
    ELSE:
        FAIL with error
    END IF
```

---

## Integration Points

### Integration with Main Entry Point

```pseudocode
FUNCTION Main():
    BEGIN
        // Step 1: Load configuration (FIRST STEP)
        config_result = LoadConfiguration()

        IF config_result IS Error:
            error_message = ReportConfigurationError(config_result.error)
            LOG_ERROR(error_message)
            EXIT 1
        END IF

        config = config_result.value

        // Step 2: Log configuration (sanitized)
        LogConfiguration(config)

        // Step 3: Continue with event routing
        // Pass config to EventRouter or other modules
        ...
    END
END FUNCTION
```

### Integration with Event Router

```pseudocode
FUNCTION RouteEvent(config: Configuration, event_data: EventData):
    BEGIN
        // Configuration provides:
        // - Event type (config.github_event_name)
        // - Feature toggles (config.enable_*)
        // - Tokens and credentials
        // - Paths (workspace, event file)

        MATCH config.github_event_name:
            CASE "push":
                RETURN HandlePushEvent(config, event_data)
            CASE "pull_request":
                RETURN HandlePullRequestEvent(config, event_data)
            // etc.
        END MATCH
    END
END FUNCTION
```

### Integration with Binary Management

```pseudocode
FUNCTION DownloadGitleaks(config: Configuration):
    BEGIN
        // Configuration provides:
        // - Version to download (config.gitleaks_version)
        // - Workspace for caching (config.github_workspace)

        version = config.gitleaks_version
        cache_dir = JoinPath(config.github_workspace, ".gitleaks-cache")

        // Use version from config
        binary_path = DownloadBinary(version, cache_dir)
        RETURN binary_path
    END
END FUNCTION
```

### Integration with Gitleaks Execution

```pseudocode
FUNCTION BuildGitleaksArguments(config: Configuration, log_opts: Option<String>):
    BEGIN
        args = [
            "detect",
            "--redact",
            "-v",
            "--exit-code=2",
            "--report-format=sarif",
            "--report-path=results.sarif",
            "--log-level=debug"
        ]

        // Add config file if discovered
        IF config.gitleaks_config IS NOT NULL:
            Append(args, "--config=" + ToString(config.gitleaks_config))
        END IF

        // Add log opts if provided
        IF log_opts IS NOT NULL:
            Append(args, "--log-opts=" + log_opts)
        END IF

        RETURN args
    END
END FUNCTION
```

---

## Usage Examples

### Example 1: Basic Configuration Loading

```pseudocode
// Environment:
// GITHUB_WORKSPACE=/home/runner/work/repo
// GITHUB_EVENT_PATH=/home/runner/work/_temp/event.json
// GITHUB_EVENT_NAME=push
// GITHUB_REPOSITORY=owner/repo
// GITHUB_REPOSITORY_OWNER=owner

config = LoadConfiguration()
// Result:
// - gitleaks_version = "8.24.3" (default)
// - enable_summary = true (default)
// - enable_comments = true (default)
// - gitleaks_config = None (no config found)
```

### Example 2: Boolean Parsing

```pseudocode
// Environment:
// GITLEAKS_ENABLE_SUMMARY=false
// GITLEAKS_ENABLE_UPLOAD_ARTIFACT=0
// GITLEAKS_ENABLE_COMMENTS=true

enable_summary = ParseBooleanValue("false")        // → false
enable_artifact = ParseBooleanValue("0")           // → false
enable_comments = ParseBooleanValue("true")        // → true
```

### Example 3: Configuration File Discovery

```pseudocode
// Scenario A: Explicit config
// Environment: GITLEAKS_CONFIG=custom-config.toml
config_path = DiscoverConfigFile(workspace)
// Result: /home/runner/work/repo/custom-config.toml

// Scenario B: Auto-detect
// Environment: (no GITLEAKS_CONFIG)
// File exists: /home/runner/work/repo/gitleaks.toml
config_path = DiscoverConfigFile(workspace)
// Result: /home/runner/work/repo/gitleaks.toml

// Scenario C: No config
// Environment: (no GITLEAKS_CONFIG)
// File does not exist: gitleaks.toml
config_path = DiscoverConfigFile(workspace)
// Result: None (use gitleaks defaults)
```

### Example 4: Path Validation

```pseudocode
// Valid path
result = ValidatePath(
    "/home/runner/work/repo/config.toml",
    "/home/runner/work/repo"
)
// Result: Success

// Invalid path (traversal)
result = ValidatePath(
    "/home/runner/work/repo/../../../etc/passwd",
    "/home/runner/work/repo"
)
// Result: Error(SECURITY_VIOLATION, "Path contains '..'")

// Invalid path (outside workspace)
result = ValidatePath(
    "/tmp/config.toml",
    "/home/runner/work/repo"
)
// Result: Error(SECURITY_VIOLATION, "Path is outside GITHUB_WORKSPACE")
```

### Example 5: User List Parsing

```pseudocode
// Input: "@alice, @bob, charlie"
users = ParseNotifyUserList("@alice, @bob, charlie")
// Result: ["@alice", "@bob", "@charlie"]

// Input: "alice,bob,charlie" (no @ prefix)
users = ParseNotifyUserList("alice,bob,charlie")
// Result: ["@alice", "@bob", "@charlie"]

// Input: "" (empty)
users = ParseNotifyUserList("")
// Result: []
```

---

## Performance Considerations

### Time Complexity
- **LoadConfiguration:** O(n) where n = number of environment variables (~14 variables = O(1))
- **ParseBooleanValue:** O(1) - simple string comparison
- **DiscoverConfigFile:** O(1) - constant number of file system operations
- **ValidatePath:** O(1) - constant number of checks
- **ParseNotifyUserList:** O(n) where n = number of users in list

### Space Complexity
- **Configuration struct:** O(1) - fixed size structure
- **Environment variables map:** O(n) where n = number of variables
- **User list:** O(n) where n = number of users

### Optimization Strategies
1. **Lazy Loading:** Configuration is loaded once at startup
2. **Early Validation:** Fail fast on invalid configuration
3. **Minimal File I/O:** Only check file existence, don't read contents
4. **String Interning:** Reuse common string values (true/false)

---

## Security Considerations

### Path Security
1. **Path Traversal Prevention:** Reject paths with ".." sequences
2. **Workspace Containment:** Ensure all paths are within GITHUB_WORKSPACE
3. **Canonical Path Resolution:** Resolve symlinks and relative paths
4. **Permission Verification:** Check read permissions before use

### Secret Protection
1. **No Logging:** Never log GITHUB_TOKEN or GITLEAKS_LICENSE values
2. **Sanitized Display:** Mask secrets in error messages
3. **Memory Clearing:** Clear sensitive strings after use (language-dependent)

### Input Validation
1. **Type Checking:** Validate data types match expectations
2. **Range Checking:** Validate numeric values are non-negative
3. **Format Validation:** Check version strings, repository format
4. **Injection Prevention:** Sanitize all user-provided values

---

## Testing Strategy

### Unit Tests

```pseudocode
TEST "ParseBooleanValue handles false values":
    ASSERT ParseBooleanValue("false") == false
    ASSERT ParseBooleanValue("FALSE") == false
    ASSERT ParseBooleanValue("0") == false
END TEST

TEST "ParseBooleanValue handles true values":
    ASSERT ParseBooleanValue("true") == true
    ASSERT ParseBooleanValue("TRUE") == true
    ASSERT ParseBooleanValue("1") == true
    ASSERT ParseBooleanValue("") == true
    ASSERT ParseBooleanValue("anything") == true
END TEST

TEST "DiscoverConfigFile priority order":
    // Test explicit config takes priority
    SetEnv("GITLEAKS_CONFIG", "custom.toml")
    CreateFile(workspace + "/custom.toml")
    CreateFile(workspace + "/gitleaks.toml")

    config = DiscoverConfigFile(workspace)
    ASSERT EndsWith(config, "custom.toml")
END TEST

TEST "ValidatePath rejects traversal":
    result = ValidatePath("../etc/passwd", workspace)
    ASSERT result IS Error
    ASSERT result.error_type == SECURITY_VIOLATION
END TEST

TEST "ParseNotifyUserList handles various formats":
    ASSERT ParseNotifyUserList("@alice,@bob") == ["@alice", "@bob"]
    ASSERT ParseNotifyUserList("alice, bob") == ["@alice", "@bob"]
    ASSERT ParseNotifyUserList("") == []
    ASSERT ParseNotifyUserList("@alice,,@bob") == ["@alice", "@bob"]
END TEST

TEST "GetRequiredEnvVar fails on missing":
    UnsetEnv("MISSING_VAR")
    result = GetRequiredEnvVar("MISSING_VAR")
    ASSERT result IS Error
    ASSERT result.error_type == MISSING_REQUIRED_VAR
END TEST

TEST "GetOptionalEnvVar returns default":
    UnsetEnv("OPTIONAL_VAR")
    result = GetOptionalEnvVar("OPTIONAL_VAR", "default_value")
    ASSERT result == "default_value"
END TEST

TEST "IsValidVersionString validates formats":
    ASSERT IsValidVersionString("8.24.3") == true
    ASSERT IsValidVersionString("v8.24.3") == true
    ASSERT IsValidVersionString("latest") == true
    ASSERT IsValidVersionString("8.24") == false
    ASSERT IsValidVersionString("abc") == false
END TEST
```

### Integration Tests

```pseudocode
TEST "LoadConfiguration with minimal environment":
    SetupMinimalEnvironment()
    config = LoadConfiguration()
    ASSERT config IS Success
    ASSERT config.gitleaks_version == "8.24.3"
    ASSERT config.enable_summary == true
END TEST

TEST "LoadConfiguration with PR event requires token":
    SetupMinimalEnvironment()
    SetEnv("GITHUB_EVENT_NAME", "pull_request")
    UnsetEnv("GITHUB_TOKEN")

    result = LoadConfiguration()
    ASSERT result IS Error
    // Note: This is validated later, not in LoadConfiguration itself
END TEST

TEST "Configuration file discovery flow":
    SetupMinimalEnvironment()
    workspace = GetEnv("GITHUB_WORKSPACE")

    // Test with explicit config
    CreateFile(workspace + "/my-config.toml")
    SetEnv("GITLEAKS_CONFIG", "my-config.toml")
    config = LoadConfiguration()
    ASSERT EndsWith(config.gitleaks_config, "my-config.toml")

    // Test with auto-detect
    UnsetEnv("GITLEAKS_CONFIG")
    CreateFile(workspace + "/gitleaks.toml")
    config = LoadConfiguration()
    ASSERT EndsWith(config.gitleaks_config, "gitleaks.toml")

    // Test with no config
    DeleteFile(workspace + "/gitleaks.toml")
    config = LoadConfiguration()
    ASSERT config.gitleaks_config IS NULL
END TEST
```

---

## Logging and Debugging

### Debug Logging

```pseudocode
FUNCTION LogConfiguration(config: Configuration):
    INPUT: config - Configuration object
    OUTPUT: None (logs to console)

    BEGIN
        LOG_INFO("=== Configuration Loaded ===")
        LOG_INFO("Event: " + config.github_event_name)
        LOG_INFO("Repository: " + config.github_repository)
        LOG_INFO("Workspace: " + ToString(config.github_workspace))
        LOG_INFO("Gitleaks Version: " + config.gitleaks_version)

        IF config.gitleaks_config IS NOT NULL:
            LOG_INFO("Config File: " + ToString(config.gitleaks_config))
        ELSE:
            LOG_INFO("Config File: (using gitleaks defaults)")
        END IF

        LOG_INFO("Feature Toggles:")
        LOG_INFO("  - Summary: " + ToString(config.enable_summary))
        LOG_INFO("  - Artifacts: " + ToString(config.enable_upload_artifact))
        LOG_INFO("  - Comments: " + ToString(config.enable_comments))

        IF config.notify_user_list IS NOT NULL:
            user_count = Length(config.notify_user_list)
            LOG_INFO("Notification Users: " + ToString(user_count) + " users")
        END IF

        IF config.base_ref IS NOT NULL:
            LOG_INFO("Base Ref Override: " + config.base_ref)
        END IF

        // DO NOT LOG: github_token, gitleaks_license
        LOG_INFO("===========================")
    END
END FUNCTION
```

### Error Context

```pseudocode
FUNCTION AddErrorContext(error: ConfigurationError, context: String) -> ConfigurationError:
    INPUT:
        error - Original error
        context - Additional context string
    OUTPUT: Enhanced error with context

    BEGIN
        enhanced_message = error.message + "\nContext: " + context

        RETURN ConfigurationError{
            error_type: error.error_type,
            message: enhanced_message,
            field_name: error.field_name
        }
    END
END FUNCTION
```

---

## Future Enhancements

### Potential Improvements

1. **Configuration Caching**
   - Cache configuration validation results
   - Avoid redundant file system checks

2. **Configuration Schema Validation**
   - Validate gitleaks.toml structure
   - Provide early feedback on invalid config files

3. **Configuration Merging**
   - Support multiple config sources
   - Allow action inputs to override environment variables

4. **Configuration Profiles**
   - Support named configuration profiles
   - Switch between profiles via environment variable

5. **Enhanced Validation**
   - Regex validation for user lists
   - URL validation for custom endpoints
   - Whitelist validation for allowed values

---

## References

- **SPARC Specification:** `/workspaces/SecretScout/docs/SPARC_SPECIFICATION.md`
- **Functional Requirements:** Section 3.2 (FR-8, FR-9)
- **Security Requirements:** Section 10.1 (Input Validation)
- **Original Implementation:** `gitleaks-action` JavaScript source code

---

**Document Status:** Complete
**Version:** 1.0
**Date:** 2025-10-16
**Next Phase:** Architecture Design
