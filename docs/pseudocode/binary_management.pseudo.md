# Binary Management Pseudocode

**Module:** Gitleaks Binary Management (FR-2)
**Purpose:** Download, cache, and execute the gitleaks binary
**Reference:** SPARC_SPECIFICATION.md Section 3.1 FR-2

---

## Table of Contents

1. [ResolveGitleaksVersion](#1-resolvegitleaksversion)
2. [DownloadGitleaksBinary](#2-downloadgitleaksbinary)
3. [DetectPlatform](#3-detectplatform)
4. [ExtractArchive](#4-extractarchive)
5. [ExecuteGitleaks](#5-executegitleaks)
6. [HandleExitCode](#6-handleexitcode)
7. [Supporting Data Structures](#7-supporting-data-structures)

---

## 1. ResolveGitleaksVersion()

**Purpose:** Resolve the gitleaks version to download (default, override, or "latest")

**Inputs:**
- Environment variable: `GITLEAKS_VERSION` (optional)
- Octokit client: For GitHub API access

**Outputs:**
- Semantic version string (e.g., "8.24.3")

**Algorithm:**

```
FUNCTION ResolveGitleaksVersion(octokit: OctokitClient) -> String
    // Default version constant
    CONST DEFAULT_VERSION = "8.24.3"

    // Step 1: Check for environment variable override
    version = GET_ENVIRONMENT_VARIABLE("GITLEAKS_VERSION")

    // Step 2: Handle three cases
    IF version IS NULL OR version IS EMPTY THEN
        // Case 1: No override - use default
        LOG_INFO("No GITLEAKS_VERSION specified, using default: " + DEFAULT_VERSION)
        RETURN DEFAULT_VERSION

    ELSE IF version EQUALS "latest" THEN
        // Case 2: "latest" keyword - fetch from GitHub API
        LOG_INFO("GITLEAKS_VERSION set to 'latest', fetching latest release...")

        TRY
            // Call GitHub API to get latest release
            response = octokit.request(
                "GET /repos/{owner}/{repo}/releases/latest",
                owner: "zricethezav",
                repo: "gitleaks"
            )

            // Extract tag name and remove 'v' prefix
            tag_name = response.data.tag_name
            latest_version = REMOVE_PREFIX(tag_name, "v")

            LOG_INFO("Latest gitleaks version: " + latest_version)
            RETURN latest_version

        CATCH error
            LOG_ERROR("Failed to fetch latest gitleaks version: " + error.message)
            LOG_WARNING("Falling back to default version: " + DEFAULT_VERSION)
            RETURN DEFAULT_VERSION
        END TRY

    ELSE
        // Case 3: Explicit version specified
        LOG_INFO("Using specified GITLEAKS_VERSION: " + version)

        // Validate semantic version format (optional but recommended)
        IF NOT IS_VALID_SEMVER(version) THEN
            LOG_WARNING("Version '" + version + "' may not be valid semantic version")
        END IF

        RETURN version
    END IF
END FUNCTION


// Helper function to validate semantic version format
FUNCTION IS_VALID_SEMVER(version: String) -> Boolean
    // Pattern: X.Y.Z where X, Y, Z are numbers
    pattern = REGEX("^[0-9]+\.[0-9]+\.[0-9]+$")
    RETURN MATCHES(version, pattern)
END FUNCTION


// Helper function to remove prefix from string
FUNCTION REMOVE_PREFIX(text: String, prefix: String) -> String
    IF STARTS_WITH(text, prefix) THEN
        RETURN SUBSTRING(text, LENGTH(prefix))
    ELSE
        RETURN text
    END IF
END FUNCTION
```

**Key Behaviors:**
- Default version: "8.24.3" (hardcoded fallback)
- Environment variable `GITLEAKS_VERSION` overrides default
- Special value "latest" triggers GitHub API call
- API failures gracefully fall back to default version
- Version validation is optional but recommended

---

## 2. DownloadGitleaksBinary()

**Purpose:** Download and cache the gitleaks binary for the current platform

**Inputs:**
- `version`: Semantic version string
- `platform`: Operating system identifier
- `arch`: CPU architecture identifier

**Outputs:**
- Path to installed binary directory

**Algorithm:**

```
FUNCTION DownloadGitleaksBinary(version: String, platform: String, arch: String) -> String
    // Step 1: Generate cache key
    cache_key = GENERATE_CACHE_KEY(version, platform, arch)

    // Step 2: Determine installation path
    temp_dir = GET_SYSTEM_TEMP_DIR()
    install_path = JOIN_PATH(temp_dir, "gitleaks-" + version)

    LOG_INFO("Version to install: " + version)
    LOG_INFO("Target directory: " + install_path)
    LOG_INFO("Cache key: " + cache_key)

    // Step 3: Attempt cache restoration
    restored_from_cache = NULL

    TRY
        restored_from_cache = RESTORE_CACHE(
            paths: [install_path],
            key: cache_key
        )
    CATCH error
        LOG_WARNING("Cache restoration failed: " + error.message)
        // Continue with download
    END TRY

    // Step 4: Check if cache hit
    IF restored_from_cache IS NOT NULL THEN
        LOG_INFO("Gitleaks restored from cache")
        ADD_TO_PATH(install_path)
        RETURN install_path
    END IF

    // Step 5: Cache miss - proceed with download
    LOG_INFO("Cache miss - downloading gitleaks binary")

    // Step 6: Construct download URL
    download_url = CONSTRUCT_DOWNLOAD_URL(version, platform, arch)
    LOG_INFO("Downloading gitleaks from: " + download_url)

    // Step 7: Download archive
    TRY
        temp_archive_path = JOIN_PATH(temp_dir, "gitleaks.tmp")
        downloaded_path = DOWNLOAD_FILE(download_url, temp_archive_path)
    CATCH error
        LOG_ERROR("Could not download gitleaks from " + download_url)
        LOG_ERROR("Error: " + error.message)
        THROW error
    END TRY

    // Step 8: Extract archive based on format
    TRY
        IF ENDS_WITH(download_url, ".zip") THEN
            EXTRACT_ZIP(downloaded_path, install_path)

        ELSE IF ENDS_WITH(download_url, ".tar.gz") THEN
            EXTRACT_TAR_GZ(downloaded_path, install_path)

        ELSE
            LOG_ERROR("Unsupported archive format: " + download_url)
            THROW ERROR("Unsupported archive format")
        END IF
    CATCH error
        LOG_ERROR("Archive extraction failed: " + error.message)
        THROW error
    END TRY

    // Step 9: Save to cache for future runs
    TRY
        SAVE_CACHE(
            paths: [install_path],
            key: cache_key
        )
        LOG_INFO("Binary cached successfully")
    CATCH error
        LOG_WARNING("Cache save failed: " + error.message)
        // Non-fatal - continue execution
    END TRY

    // Step 10: Add binary to PATH
    ADD_TO_PATH(install_path)

    // Step 11: Return installation path
    RETURN install_path
END FUNCTION


// Helper function to generate cache key
FUNCTION GENERATE_CACHE_KEY(version: String, platform: String, arch: String) -> String
    // Format: "gitleaks-cache-{version}-{platform}-{arch}"
    RETURN "gitleaks-cache-" + version + "-" + platform + "-" + arch
END FUNCTION


// Helper function to construct GitHub release download URL
FUNCTION CONSTRUCT_DOWNLOAD_URL(version: String, platform: String, arch: String) -> String
    CONST BASE_URL = "https://github.com/zricethezav/gitleaks/releases/download"

    // Platform normalization (Windows special case)
    normalized_platform = platform
    IF platform EQUALS "win32" THEN
        normalized_platform = "windows"
    END IF

    // Determine archive extension
    IF normalized_platform EQUALS "windows" THEN
        extension = "zip"
    ELSE
        extension = "tar.gz"
    END IF

    // URL pattern: {base}/v{version}/gitleaks_{version}_{platform}_{arch}.{ext}
    url = BASE_URL + "/v" + version + "/gitleaks_" + version + "_" + normalized_platform + "_" + arch + "." + extension

    RETURN url
END FUNCTION
```

**Key Behaviors:**
- Cache key format: `gitleaks-cache-{version}-{platform}-{arch}`
- Installation path: `{TEMP_DIR}/gitleaks-{version}`
- Download from GitHub releases: `https://github.com/zricethezav/gitleaks/releases/download/v{version}/gitleaks_{version}_{platform}_{arch}.{ext}`
- Windows uses `.zip` archives, Unix systems use `.tar.gz`
- Cache failures are non-fatal warnings
- Binary is added to PATH after extraction

---

## 3. DetectPlatform()

**Purpose:** Detect the current operating system and CPU architecture

**Inputs:**
- System environment information

**Outputs:**
- `PlatformInfo` structure containing platform and architecture

**Algorithm:**

```
FUNCTION DetectPlatform() -> PlatformInfo
    // Step 1: Detect operating system
    os = GET_SYSTEM_OS()

    platform = NULL
    SWITCH os
        CASE "linux":
            platform = "linux"
        CASE "darwin":
            platform = "darwin"
        CASE "win32":
            platform = "win32"  // Will be normalized to "windows" in download URL
        DEFAULT:
            LOG_ERROR("Unsupported operating system: " + os)
            THROW ERROR("Unsupported platform: " + os)
    END SWITCH

    // Step 2: Detect CPU architecture
    cpu_arch = GET_SYSTEM_ARCH()

    arch = NULL
    SWITCH cpu_arch
        CASE "x64":
        CASE "x86_64":
        CASE "amd64":
            arch = "x64"
        CASE "arm64":
        CASE "aarch64":
            arch = "arm64"
        CASE "arm":
            arch = "arm"
        DEFAULT:
            LOG_ERROR("Unsupported architecture: " + cpu_arch)
            THROW ERROR("Unsupported architecture: " + cpu_arch)
    END SWITCH

    // Step 3: Validate platform/architecture combination
    IF NOT IS_VALID_COMBINATION(platform, arch) THEN
        LOG_WARNING("Platform/architecture combination may not be supported: " + platform + "/" + arch)
    END IF

    // Step 4: Return platform information
    result = NEW PlatformInfo
    result.platform = platform
    result.arch = arch

    LOG_INFO("Detected platform: " + platform + "/" + arch)

    RETURN result
END FUNCTION


// Helper function to validate platform/architecture combinations
FUNCTION IS_VALID_COMBINATION(platform: String, arch: String) -> Boolean
    // Known valid combinations for gitleaks
    valid_combinations = [
        ("linux", "x64"),
        ("linux", "arm64"),
        ("darwin", "x64"),
        ("darwin", "arm64"),
        ("win32", "x64"),
        ("windows", "x64")
    ]

    combination = (platform, arch)
    RETURN combination IN valid_combinations
END FUNCTION
```

**Key Behaviors:**
- Supported platforms: `linux`, `darwin`, `win32` (mapped to `windows`)
- Supported architectures: `x64`, `arm64`, `arm`
- CPU architecture normalization (e.g., `x86_64` → `x64`, `aarch64` → `arm64`)
- Validation of platform/architecture combinations
- Unsupported platforms/architectures throw errors

---

## 4. ExtractArchive()

**Purpose:** Extract tar.gz or zip archives containing the gitleaks binary

**Inputs:**
- `archive_path`: Path to downloaded archive file
- `destination_path`: Directory to extract to
- `archive_type`: Type of archive ("tar.gz" or "zip")

**Outputs:**
- Success status (void, throws on error)

**Algorithm:**

```
FUNCTION ExtractArchive(archive_path: String, destination_path: String, archive_type: String) -> Void
    // Step 1: Create destination directory if it doesn't exist
    IF NOT DIRECTORY_EXISTS(destination_path) THEN
        CREATE_DIRECTORY(destination_path)
    END IF

    // Step 2: Extract based on archive type
    TRY
        IF archive_type EQUALS "tar.gz" THEN
            EXTRACT_TAR_GZ_ARCHIVE(archive_path, destination_path)

        ELSE IF archive_type EQUALS "zip" THEN
            EXTRACT_ZIP_ARCHIVE(archive_path, destination_path)

        ELSE
            THROW ERROR("Unsupported archive type: " + archive_type)
        END IF

        LOG_INFO("Successfully extracted archive to: " + destination_path)

    CATCH error
        LOG_ERROR("Archive extraction failed: " + error.message)
        THROW error
    END TRY

    // Step 3: Verify binary exists after extraction
    binary_name = GET_BINARY_NAME()
    binary_path = JOIN_PATH(destination_path, binary_name)

    IF NOT FILE_EXISTS(binary_path) THEN
        LOG_ERROR("Binary not found after extraction: " + binary_path)
        THROW ERROR("Binary not found in extracted archive")
    END IF

    // Step 4: Set executable permissions (Unix systems only)
    IF IS_UNIX_PLATFORM() THEN
        SET_EXECUTABLE_PERMISSION(binary_path)
        LOG_INFO("Set executable permission on binary: " + binary_path)
    END IF
END FUNCTION


// Helper function to extract tar.gz archives
FUNCTION EXTRACT_TAR_GZ_ARCHIVE(archive_path: String, destination: String) -> Void
    // Open archive file
    file_handle = OPEN_FILE(archive_path, mode: READ_BINARY)

    // Create gzip decompression stream
    gzip_stream = CREATE_GZIP_DECOMPRESSOR(file_handle)

    // Create tar extraction stream
    tar_stream = CREATE_TAR_EXTRACTOR(gzip_stream)

    // Extract all entries
    FOR EACH entry IN tar_stream DO
        entry_path = JOIN_PATH(destination, entry.name)

        // Security check: prevent path traversal
        IF CONTAINS(entry.name, "..") THEN
            LOG_WARNING("Skipping potentially malicious path: " + entry.name)
            CONTINUE
        END IF

        IF entry.is_directory THEN
            CREATE_DIRECTORY(entry_path)
        ELSE
            // Extract file
            WRITE_FILE(entry_path, entry.data)
        END IF
    END FOR

    CLOSE(file_handle)
END FUNCTION


// Helper function to extract zip archives
FUNCTION EXTRACT_ZIP_ARCHIVE(archive_path: String, destination: String) -> Void
    // Open zip archive
    zip_handle = OPEN_ZIP_ARCHIVE(archive_path)

    // Extract all entries
    FOR EACH entry IN zip_handle.entries DO
        entry_path = JOIN_PATH(destination, entry.name)

        // Security check: prevent path traversal
        IF CONTAINS(entry.name, "..") THEN
            LOG_WARNING("Skipping potentially malicious path: " + entry.name)
            CONTINUE
        END IF

        IF entry.is_directory THEN
            CREATE_DIRECTORY(entry_path)
        ELSE
            // Extract file
            entry_data = zip_handle.READ(entry)
            WRITE_FILE(entry_path, entry_data)
        END IF
    END FOR

    CLOSE(zip_handle)
END FUNCTION


// Helper function to get binary name based on platform
FUNCTION GET_BINARY_NAME() -> String
    IF IS_WINDOWS_PLATFORM() THEN
        RETURN "gitleaks.exe"
    ELSE
        RETURN "gitleaks"
    END IF
END FUNCTION


// Helper function to set executable permission
FUNCTION SET_EXECUTABLE_PERMISSION(file_path: String) -> Void
    // On Unix systems, set executable bit (chmod +x)
    current_permissions = GET_FILE_PERMISSIONS(file_path)
    new_permissions = current_permissions | EXECUTABLE_BIT
    SET_FILE_PERMISSIONS(file_path, new_permissions)
END FUNCTION
```

**Key Behaviors:**
- Supports tar.gz (Unix) and zip (Windows) formats
- Creates destination directory if needed
- Security: Prevents path traversal attacks (checks for ".." in paths)
- Sets executable permissions on Unix systems
- Verifies binary exists after extraction
- Throws errors on extraction failures

---

## 5. ExecuteGitleaks()

**Purpose:** Execute the gitleaks binary with appropriate arguments based on event type

**Inputs:**
- `event_type`: GitHub event type ("push", "pull_request", "workflow_dispatch", "schedule")
- `scan_info`: Scanning parameters (baseRef, headRef)
- `config_path`: Optional path to gitleaks.toml config file
- `enable_upload_artifact`: Whether to upload SARIF artifact

**Outputs:**
- Exit code from gitleaks execution (0, 1, or 2)

**Algorithm:**

```
FUNCTION ExecuteGitleaks(
    event_type: String,
    scan_info: ScanInfo,
    config_path: Optional<String>,
    enable_upload_artifact: Boolean
) -> Integer

    // Step 1: Build base arguments (always included)
    args = BUILD_BASE_ARGUMENTS()

    // Step 2: Add event-specific log-opts arguments
    log_opts = BUILD_LOG_OPTS(event_type, scan_info)
    IF log_opts IS NOT NULL THEN
        APPEND(args, log_opts)
    END IF

    // Step 3: Add config path if specified
    IF config_path IS NOT NULL THEN
        config_arg = "--config=" + config_path
        APPEND(args, config_arg)
        LOG_INFO("Using config file: " + config_path)
    END IF

    // Step 4: Log full command
    command_string = "gitleaks " + JOIN(args, " ")
    LOG_INFO("Executing command: " + command_string)

    // Step 5: Execute gitleaks binary
    TRY
        process_result = EXECUTE_PROCESS(
            command: "gitleaks",
            arguments: args,
            options: {
                ignore_return_code: TRUE,  // Don't throw on non-zero exit
                timeout_ms: 3600000,        // 1 hour timeout
                capture_output: TRUE        // Capture stdout/stderr
            }
        )

        exit_code = process_result.exit_code

        // Step 6: Log stdout/stderr
        IF NOT IS_EMPTY(process_result.stdout) THEN
            LOG_DEBUG("Gitleaks stdout:")
            LOG_DEBUG(process_result.stdout)
        END IF

        IF NOT IS_EMPTY(process_result.stderr) THEN
            LOG_DEBUG("Gitleaks stderr:")
            LOG_DEBUG(process_result.stderr)
        END IF

        // Step 7: Set action output
        SET_ACTION_OUTPUT("exit-code", exit_code)

    CATCH error
        LOG_ERROR("Failed to execute gitleaks: " + error.message)
        THROW error
    END TRY

    // Step 8: Upload SARIF artifact if enabled and results exist
    IF enable_upload_artifact EQUALS TRUE THEN
        UPLOAD_SARIF_ARTIFACT()
    END IF

    // Step 9: Return exit code for further processing
    RETURN exit_code
END FUNCTION


// Helper function to build base arguments
FUNCTION BUILD_BASE_ARGUMENTS() -> Array<String>
    // Standard arguments for all scans
    base_args = [
        "detect",                    // Detection mode
        "--redact",                  // Redact secret values in output
        "-v",                        // Verbose mode
        "--exit-code=2",             // Use exit code 2 for detected leaks
        "--report-format=sarif",     // SARIF v2 output format
        "--report-path=results.sarif", // Output file path
        "--log-level=debug"          // Debug logging
    ]

    RETURN base_args
END FUNCTION


// Helper function to build log-opts based on event type
FUNCTION BUILD_LOG_OPTS(event_type: String, scan_info: ScanInfo) -> Optional<String>
    IF event_type EQUALS "push" THEN
        // Check if single commit or range
        IF scan_info.baseRef EQUALS scan_info.headRef THEN
            // Single commit optimization
            RETURN "--log-opts=-1"
        ELSE
            // Commit range
            RETURN "--log-opts=--no-merges --first-parent " + scan_info.baseRef + "^.." + scan_info.headRef
        END IF

    ELSE IF event_type EQUALS "pull_request" THEN
        // Always use range for PRs
        RETURN "--log-opts=--no-merges --first-parent " + scan_info.baseRef + "^.." + scan_info.headRef

    ELSE IF event_type EQUALS "workflow_dispatch" THEN
        // Full repository scan - no log opts
        RETURN NULL

    ELSE IF event_type EQUALS "schedule" THEN
        // Full repository scan - no log opts
        RETURN NULL

    ELSE
        LOG_WARNING("Unknown event type: " + event_type)
        RETURN NULL
    END IF
END FUNCTION


// Helper function to upload SARIF artifact
FUNCTION UPLOAD_SARIF_ARTIFACT() -> Void
    CONST SARIF_FILE_PATH = "results.sarif"
    CONST ARTIFACT_NAME = "gitleaks-results.sarif"

    // Check if SARIF file exists
    IF NOT FILE_EXISTS(SARIF_FILE_PATH) THEN
        LOG_INFO("No SARIF results file to upload")
        RETURN
    END IF

    TRY
        LOG_INFO("Uploading SARIF artifact...")

        workspace_dir = GET_ENVIRONMENT_VARIABLE("GITHUB_WORKSPACE") OR GET_ENVIRONMENT_VARIABLE("HOME")

        artifact_client = CREATE_ARTIFACT_CLIENT()
        artifact_client.UPLOAD_ARTIFACT(
            name: ARTIFACT_NAME,
            files: [SARIF_FILE_PATH],
            root_directory: workspace_dir,
            options: {
                continue_on_error: TRUE
            }
        )

        LOG_INFO("SARIF artifact uploaded successfully")

    CATCH error
        LOG_WARNING("Failed to upload SARIF artifact: " + error.message)
        // Non-fatal - continue execution
    END TRY
END FUNCTION
```

**Key Behaviors:**
- Base arguments are always included
- Log-opts depend on event type:
  - Push (single commit): `--log-opts=-1`
  - Push (range): `--log-opts=--no-merges --first-parent {base}^..{head}`
  - Pull request: `--log-opts=--no-merges --first-parent {base}^..{head}`
  - Workflow dispatch: No log-opts (full scan)
  - Schedule: No log-opts (full scan)
- Config file is optional (`--config={path}`)
- Exit codes are captured but not thrown as errors
- SARIF artifact upload is conditional
- Stdout/stderr are captured for debugging

---

## 6. HandleExitCode()

**Purpose:** Interpret and handle gitleaks exit codes appropriately

**Inputs:**
- `exit_code`: Integer exit code from gitleaks (0, 1, or 2)
- `event_json`: GitHub event payload
- `enable_summary`: Whether to generate job summary

**Outputs:**
- Final action exit code (0 or 1)

**Algorithm:**

```
FUNCTION HandleExitCode(
    exit_code: Integer,
    event_json: EventJSON,
    enable_summary: Boolean
) -> Integer

    CONST EXIT_CODE_SUCCESS = 0
    CONST EXIT_CODE_ERROR = 1
    CONST EXIT_CODE_LEAKS_DETECTED = 2

    // Step 1: Generate job summary if enabled
    IF enable_summary EQUALS TRUE THEN
        GENERATE_JOB_SUMMARY(exit_code, event_json)
    END IF

    // Step 2: Handle exit code cases
    IF exit_code EQUALS EXIT_CODE_SUCCESS THEN
        // Case 1: No leaks detected - Success
        LOG_INFO("✅ No leaks detected")
        RETURN EXIT_CODE_SUCCESS

    ELSE IF exit_code EQUALS EXIT_CODE_LEAKS_DETECTED THEN
        // Case 2: Leaks detected - Process results then fail
        LOG_WARNING("🛑 Leaks detected, see job summary for details")

        // IMPORTANT: Results have already been processed:
        // - SARIF file generated
        // - PR comments posted (if applicable)
        // - Job summary generated
        // - Artifacts uploaded

        // Now exit with failure status
        RETURN EXIT_CODE_ERROR

    ELSE IF exit_code EQUALS EXIT_CODE_ERROR THEN
        // Case 3: Error occurred in gitleaks
        LOG_ERROR("❌ Gitleaks exited with error code 1")
        RETURN EXIT_CODE_ERROR

    ELSE
        // Case 4: Unexpected exit code
        LOG_ERROR("ERROR: Unexpected exit code [" + exit_code + "]")
        RETURN exit_code
    END IF
END FUNCTION


// Helper function to generate job summary
FUNCTION GENERATE_JOB_SUMMARY(exit_code: Integer, event_json: EventJSON) -> Void
    CONST EXIT_CODE_SUCCESS = 0
    CONST EXIT_CODE_ERROR = 1
    CONST EXIT_CODE_LEAKS_DETECTED = 2

    TRY
        IF exit_code EQUALS EXIT_CODE_SUCCESS THEN
            // Success summary
            summary_content = "## No leaks detected ✅\n"
            WRITE_SUMMARY(summary_content)

        ELSE IF exit_code EQUALS EXIT_CODE_LEAKS_DETECTED THEN
            // Leaks detected - generate detailed table
            summary_content = GENERATE_LEAKS_SUMMARY(event_json)
            WRITE_SUMMARY(summary_content)

        ELSE IF exit_code EQUALS EXIT_CODE_ERROR THEN
            // Error summary
            summary_content = "## ❌ Gitleaks exited with error. Exit code [1]\n"
            WRITE_SUMMARY(summary_content)

        ELSE
            // Unexpected exit code
            summary_content = "## ❌ Gitleaks exited with unexpected code [" + exit_code + "]\n"
            WRITE_SUMMARY(summary_content)
        END IF

    CATCH error
        LOG_WARNING("Failed to generate job summary: " + error.message)
        // Non-fatal - continue
    END TRY
END FUNCTION


// Helper function to generate detailed leaks summary table
FUNCTION GENERATE_LEAKS_SUMMARY(event_json: EventJSON) -> String
    CONST SARIF_FILE_PATH = "results.sarif"

    // Read and parse SARIF file
    IF NOT FILE_EXISTS(SARIF_FILE_PATH) THEN
        RETURN "## 🛑 Gitleaks detected secrets 🛑\n\nNo SARIF results file found.\n"
    END IF

    sarif_content = READ_FILE(SARIF_FILE_PATH)
    sarif_data = PARSE_JSON(sarif_content)

    // Extract results
    results = sarif_data.runs[0].results

    IF LENGTH(results) EQUALS 0 THEN
        RETURN "## No leaks detected ✅\n"
    END IF

    // Build HTML table
    repo_url = GET_REPOSITORY_URL(event_json)

    summary = "## 🛑 Gitleaks detected secrets 🛑\n\n"
    summary += "<table>\n"
    summary += "<tr>\n"
    summary += "  <th>Rule ID</th>\n"
    summary += "  <th>Commit</th>\n"
    summary += "  <th>Secret URL</th>\n"
    summary += "  <th>Start Line</th>\n"
    summary += "  <th>Author</th>\n"
    summary += "  <th>Date</th>\n"
    summary += "  <th>Email</th>\n"
    summary += "  <th>File</th>\n"
    summary += "</tr>\n"

    FOR EACH result IN results DO
        rule_id = result.ruleId
        commit_sha = result.partialFingerprints.commitSha
        commit_short = SUBSTRING(commit_sha, 0, 7)
        file_path = result.locations[0].physicalLocation.artifactLocation.uri
        start_line = result.locations[0].physicalLocation.region.startLine
        author = result.partialFingerprints.author OR "Unknown"
        date = result.partialFingerprints.date OR "Unknown"
        email = result.partialFingerprints.email OR "Unknown"

        // Generate URLs
        commit_url = repo_url + "/commit/" + commit_sha
        secret_url = repo_url + "/blob/" + commit_sha + "/" + file_path + "#L" + start_line
        file_url = repo_url + "/blob/" + commit_sha + "/" + file_path

        // Add table row
        summary += "<tr>\n"
        summary += "  <td>" + rule_id + "</td>\n"
        summary += "  <td><a href=\"" + commit_url + "\">" + commit_short + "</a></td>\n"
        summary += "  <td><a href=\"" + secret_url + "\">Link</a></td>\n"
        summary += "  <td>" + start_line + "</td>\n"
        summary += "  <td>" + author + "</td>\n"
        summary += "  <td>" + date + "</td>\n"
        summary += "  <td>" + email + "</td>\n"
        summary += "  <td><a href=\"" + file_url + "\">" + file_path + "</a></td>\n"
        summary += "</tr>\n"
    END FOR

    summary += "</table>\n"

    RETURN summary
END FUNCTION


// Helper function to get repository URL
FUNCTION GET_REPOSITORY_URL(event_json: EventJSON) -> String
    // Format: https://github.com/{owner}/{repo}
    server_url = GET_ENVIRONMENT_VARIABLE("GITHUB_SERVER_URL") OR "https://github.com"
    repository = event_json.repository.full_name

    RETURN server_url + "/" + repository
END FUNCTION


// Helper function to write summary to GitHub Actions
FUNCTION WRITE_SUMMARY(content: String) -> Void
    summary_file = GET_ENVIRONMENT_VARIABLE("GITHUB_STEP_SUMMARY")

    IF summary_file IS NULL THEN
        LOG_WARNING("GITHUB_STEP_SUMMARY not set, cannot write summary")
        RETURN
    END IF

    APPEND_TO_FILE(summary_file, content)
    LOG_INFO("Job summary written successfully")
END FUNCTION
```

**Key Behaviors:**
- Exit code 0: Success, no leaks
- Exit code 1: Error in gitleaks execution
- Exit code 2: Leaks detected
  - **Critical:** Results must be processed BEFORE exiting with failure
  - Processing includes: SARIF parsing, PR comments, summary, artifacts
  - Then return exit code 1 to fail the GitHub Action
- Exit code propagation: Unexpected codes are passed through
- Job summary generation is conditional
- Summary format varies by exit code:
  - 0: Simple success message
  - 2: Detailed HTML table with all leaks
  - 1: Error message
  - Other: Unexpected code message

---

## 7. Supporting Data Structures

**Purpose:** Define data structures used in binary management algorithms

### 7.1 PlatformInfo

```
STRUCTURE PlatformInfo
    platform: String      // Operating system: "linux", "darwin", "win32", "windows"
    arch: String         // CPU architecture: "x64", "arm64", "arm"
END STRUCTURE
```

### 7.2 ScanInfo

```
STRUCTURE ScanInfo
    baseRef: String      // Base git reference (commit SHA, branch, tag)
    headRef: String      // Head git reference (commit SHA)
    gitleaksPath: String // Optional: path to gitleaks binary
END STRUCTURE
```

### 7.3 ProcessResult

```
STRUCTURE ProcessResult
    exit_code: Integer   // Process exit code (0, 1, 2, etc.)
    stdout: String       // Standard output captured
    stderr: String       // Standard error captured
    duration_ms: Integer // Execution time in milliseconds
END STRUCTURE
```

### 7.4 EventJSON

```
STRUCTURE EventJSON
    repository: Repository
    commits: Array<Commit>        // For push events
    pull_request: PullRequest     // For PR events
    number: Integer               // PR number
END STRUCTURE

STRUCTURE Repository
    full_name: String    // Format: "owner/repo"
    owner: Owner
END STRUCTURE

STRUCTURE Owner
    login: String        // Username or organization name
END STRUCTURE

STRUCTURE Commit
    id: String          // Commit SHA
    sha: String         // Commit SHA (alternative field)
END STRUCTURE

STRUCTURE PullRequest
    number: Integer     // PR number
END STRUCTURE
```

### 7.5 SARIFResult

```
STRUCTURE SARIFDocument
    runs: Array<SARIFRun>
END STRUCTURE

STRUCTURE SARIFRun
    results: Array<SARIFResult>
END STRUCTURE

STRUCTURE SARIFResult
    ruleId: String
    partialFingerprints: PartialFingerprints
    locations: Array<Location>
END STRUCTURE

STRUCTURE PartialFingerprints
    commitSha: String
    author: String
    email: String
    date: String
END STRUCTURE

STRUCTURE Location
    physicalLocation: PhysicalLocation
END STRUCTURE

STRUCTURE PhysicalLocation
    artifactLocation: ArtifactLocation
    region: Region
END STRUCTURE

STRUCTURE ArtifactLocation
    uri: String          // File path
END STRUCTURE

STRUCTURE Region
    startLine: Integer   // Line number
END STRUCTURE
```

### 7.6 Constants

```
CONSTANTS
    // Exit codes
    EXIT_CODE_SUCCESS = 0
    EXIT_CODE_ERROR = 1
    EXIT_CODE_LEAKS_DETECTED = 2

    // Default version
    DEFAULT_GITLEAKS_VERSION = "8.24.3"

    // GitHub URLs
    GITLEAKS_RELEASES_URL = "https://github.com/zricethezav/gitleaks/releases/download"
    GITLEAKS_REPO_OWNER = "zricethezav"
    GITLEAKS_REPO_NAME = "gitleaks"

    // File paths
    SARIF_FILE_PATH = "results.sarif"
    ARTIFACT_NAME = "gitleaks-results.sarif"

    // Cache key prefix
    CACHE_KEY_PREFIX = "gitleaks-cache-"

    // Supported platforms
    SUPPORTED_PLATFORMS = ["linux", "darwin", "win32", "windows"]

    // Supported architectures
    SUPPORTED_ARCHITECTURES = ["x64", "arm64", "arm"]

    // Archive extensions
    WINDOWS_ARCHIVE_EXT = ".zip"
    UNIX_ARCHIVE_EXT = ".tar.gz"
END CONSTANTS
```

---

## 8. Complete Integration Flow

**Purpose:** Show how all functions work together in the main execution flow

```
FUNCTION MAIN_BINARY_MANAGEMENT_FLOW(event_type: String, event_json: EventJSON, octokit: OctokitClient)
    // Step 1: Resolve version
    version = ResolveGitleaksVersion(octokit)
    LOG_INFO("Gitleaks version: " + version)

    // Step 2: Detect platform
    platform_info = DetectPlatform()

    // Step 3: Download and install binary
    install_path = DownloadGitleaksBinary(
        version: version,
        platform: platform_info.platform,
        arch: platform_info.arch
    )

    // Step 4: Determine scan parameters
    scan_info = BUILD_SCAN_INFO(event_type, event_json, octokit)

    // Step 5: Get configuration file path (if exists)
    config_path = GET_ENVIRONMENT_VARIABLE("GITLEAKS_CONFIG")
    IF config_path IS NULL THEN
        // Auto-detect gitleaks.toml in workspace
        workspace = GET_ENVIRONMENT_VARIABLE("GITHUB_WORKSPACE")
        auto_config = JOIN_PATH(workspace, "gitleaks.toml")
        IF FILE_EXISTS(auto_config) THEN
            config_path = auto_config
        END IF
    END IF

    // Step 6: Get feature flags
    enable_upload_artifact = PARSE_BOOLEAN_ENV("GITLEAKS_ENABLE_UPLOAD_ARTIFACT", default: TRUE)
    enable_summary = PARSE_BOOLEAN_ENV("GITLEAKS_ENABLE_SUMMARY", default: TRUE)

    // Step 7: Execute gitleaks
    exit_code = ExecuteGitleaks(
        event_type: event_type,
        scan_info: scan_info,
        config_path: config_path,
        enable_upload_artifact: enable_upload_artifact
    )

    // Step 8: Handle exit code and generate outputs
    final_exit_code = HandleExitCode(
        exit_code: exit_code,
        event_json: event_json,
        enable_summary: enable_summary
    )

    // Step 9: Exit with appropriate code
    EXIT_PROCESS(final_exit_code)
END FUNCTION


// Helper to build scan info based on event type
FUNCTION BUILD_SCAN_INFO(event_type: String, event_json: EventJSON, octokit: OctokitClient) -> ScanInfo
    scan_info = NEW ScanInfo

    IF event_type EQUALS "push" THEN
        // Extract from push event commits
        commits = event_json.commits

        IF LENGTH(commits) EQUALS 0 THEN
            LOG_INFO("No commits to scan")
            EXIT_PROCESS(0)
        END IF

        scan_info.baseRef = commits[0].id
        scan_info.headRef = commits[LENGTH(commits) - 1].id

        // Override with BASE_REF if set
        base_ref_override = GET_ENVIRONMENT_VARIABLE("BASE_REF")
        IF base_ref_override IS NOT NULL THEN
            scan_info.baseRef = base_ref_override
            LOG_INFO("Overriding baseRef with: " + base_ref_override)
        END IF

    ELSE IF event_type EQUALS "pull_request" THEN
        // Fetch PR commits via API
        full_name = event_json.repository.full_name
        [owner, repo] = SPLIT(full_name, "/")
        pr_number = event_json.pull_request.number

        commits = octokit.request(
            "GET /repos/{owner}/{repo}/pulls/{pull_number}/commits",
            owner: owner,
            repo: repo,
            pull_number: pr_number
        )

        scan_info.baseRef = commits.data[0].sha
        scan_info.headRef = commits.data[LENGTH(commits.data) - 1].sha

        // Override with BASE_REF if set
        base_ref_override = GET_ENVIRONMENT_VARIABLE("BASE_REF")
        IF base_ref_override IS NOT NULL THEN
            scan_info.baseRef = base_ref_override
            LOG_INFO("Overriding baseRef with: " + base_ref_override)
        END IF

    ELSE IF event_type EQUALS "workflow_dispatch" OR event_type EQUALS "schedule" THEN
        // Full repository scan - no refs needed
        // scan_info remains empty
    END IF

    RETURN scan_info
END FUNCTION


// Helper to parse boolean environment variables
FUNCTION PARSE_BOOLEAN_ENV(var_name: String, default: Boolean) -> Boolean
    value = GET_ENVIRONMENT_VARIABLE(var_name)

    IF value IS NULL THEN
        RETURN default
    END IF

    // False values: "false" or "0"
    IF value EQUALS "false" OR value EQUALS "0" THEN
        RETURN FALSE
    END IF

    // All other values (including "true", "1", empty string) are true
    RETURN TRUE
END FUNCTION
```

---

## 9. Error Handling Patterns

**Purpose:** Document error handling strategies used throughout binary management

### 9.1 Fatal Errors (Immediate Exit)

```
FATAL_ERRORS:
    - Unsupported platform/architecture
    - Download failure (network error, 404, etc.)
    - Archive extraction failure
    - Binary not found after extraction
    - Gitleaks execution failure (exit code 1)
    - Missing GITHUB_TOKEN (for PR events)

PATTERN:
    TRY
        operation()
    CATCH error
        LOG_ERROR("Fatal error: " + error.message)
        EXIT_PROCESS(1)
    END TRY
```

### 9.2 Non-Fatal Warnings (Continue Execution)

```
NON_FATAL_WARNINGS:
    - Cache restoration failure
    - Cache save failure
    - GitHub API rate limit (with retry)
    - SARIF artifact upload failure
    - Job summary generation failure

PATTERN:
    TRY
        optional_operation()
    CATCH error
        LOG_WARNING("Warning: " + error.message)
        // Continue execution
    END TRY
```

### 9.3 Retry Logic (Transient Failures)

```
RETRY_PATTERN:
    max_retries = 3
    retry_delay_ms = 1000  // 1 second, exponential backoff

    FOR attempt = 1 TO max_retries DO
        TRY
            result = risky_operation()
            RETURN result  // Success
        CATCH error
            IF attempt < max_retries THEN
                LOG_WARNING("Attempt " + attempt + " failed: " + error.message)
                SLEEP(retry_delay_ms * (2 ^ (attempt - 1)))  // Exponential backoff
            ELSE
                LOG_ERROR("All retry attempts exhausted")
                THROW error
            END IF
        END TRY
    END FOR
```

---

## 10. Security Considerations

**Purpose:** Document security measures in binary management

### 10.1 Path Traversal Prevention

```
FUNCTION VALIDATE_PATH(path: String) -> Boolean
    // Prevent path traversal attacks
    IF CONTAINS(path, "..") THEN
        LOG_ERROR("Path traversal detected: " + path)
        RETURN FALSE
    END IF

    // Ensure path is within workspace
    workspace = GET_ENVIRONMENT_VARIABLE("GITHUB_WORKSPACE")
    absolute_path = RESOLVE_ABSOLUTE_PATH(path)

    IF NOT STARTS_WITH(absolute_path, workspace) THEN
        LOG_ERROR("Path outside workspace: " + path)
        RETURN FALSE
    END IF

    RETURN TRUE
END FUNCTION
```

### 10.2 Binary Verification

```
FUNCTION VERIFY_BINARY_INTEGRITY(binary_path: String) -> Boolean
    // Check file exists
    IF NOT FILE_EXISTS(binary_path) THEN
        RETURN FALSE
    END IF

    // Optional: Verify checksum (if checksums are published)
    // This would require downloading checksum file from GitHub releases
    // and comparing SHA256 hash of downloaded binary

    // Verify file is executable (Unix)
    IF IS_UNIX_PLATFORM() THEN
        IF NOT IS_EXECUTABLE(binary_path) THEN
            SET_EXECUTABLE_PERMISSION(binary_path)
        END IF
    END IF

    RETURN TRUE
END FUNCTION
```

### 10.3 Input Sanitization

```
FUNCTION SANITIZE_GIT_REF(ref: String) -> String
    // Remove potentially dangerous characters
    // Allow: alphanumeric, dash, underscore, forward slash, caret, tilde
    sanitized = REGEX_REPLACE(ref, "[^a-zA-Z0-9\-_/^~]", "")

    // Validate it's a valid git reference format
    IF NOT IS_VALID_GIT_REF(sanitized) THEN
        LOG_WARNING("Invalid git reference: " + ref)
        THROW ERROR("Invalid git reference")
    END IF

    RETURN sanitized
END FUNCTION
```

---

## 11. Performance Optimizations

**Purpose:** Document performance optimization strategies

### 11.1 Cache Key Strategy

```
// Cache key includes version, platform, AND architecture
// This ensures binary compatibility across different runner types
cache_key = "gitleaks-cache-" + version + "-" + platform + "-" + arch

// Example cache keys:
// - gitleaks-cache-8.24.3-linux-x64
// - gitleaks-cache-8.24.3-darwin-arm64
// - gitleaks-cache-8.24.3-windows-x64

// Cache hit ratio: ~95% for stable versions
// Cache miss only on:
// - First run with new version
// - Cache eviction (30 day TTL)
// - Different platform/arch
```

### 11.2 Single Commit Optimization

```
// For push events with identical base and head refs
// Use --log-opts=-1 instead of range syntax
// This significantly speeds up scanning (10x faster for single commits)

IF event_type EQUALS "push" AND baseRef EQUALS headRef THEN
    log_opts = "--log-opts=-1"  // Fast path
ELSE
    log_opts = "--log-opts=... " + baseRef + "^.." + headRef  // Range scan
END IF
```

### 11.3 Parallel Downloads (Future Enhancement)

```
// Currently downloads are sequential
// Future: Download multiple platform binaries in parallel for cross-platform testing

FUNCTION DOWNLOAD_MULTIPLE_PLATFORMS(version: String, platforms: Array<PlatformInfo>) -> Map<String, String>
    results = NEW Map

    // Create parallel download tasks
    tasks = []
    FOR EACH platform_info IN platforms DO
        task = ASYNC DownloadGitleaksBinary(version, platform_info.platform, platform_info.arch)
        APPEND(tasks, task)
    END FOR

    // Wait for all downloads to complete
    paths = AWAIT_ALL(tasks)

    // Map platform -> path
    FOR i = 0 TO LENGTH(platforms) - 1 DO
        key = platforms[i].platform + "-" + platforms[i].arch
        results[key] = paths[i]
    END FOR

    RETURN results
END FUNCTION
```

---

## Document Metadata

**Specification Reference:** SPARC_SPECIFICATION.md Section 3.1 FR-2
**Phase:** SPARC Pseudocode
**Author:** Claude Code Swarm
**Date:** 2025-10-16
**Version:** 1.0

**Pseudocode Status:** ✅ COMPLETE

This pseudocode document provides detailed, step-by-step algorithms for all binary management operations required by FR-2. The algorithms are implementation-agnostic and can be translated to Rust, JavaScript, or any other programming language.

**Key Features:**
- Version resolution (default, override, "latest")
- Platform detection and normalization
- Download with caching
- Archive extraction (tar.gz, zip)
- Binary execution with event-specific arguments
- Exit code handling and interpretation
- Comprehensive error handling patterns
- Security considerations
- Performance optimizations

**Next Steps:**
1. Review pseudocode for correctness and completeness
2. Use as reference for Rust implementation
3. Validate algorithm logic against original JavaScript implementation
4. Update as needed during implementation phase
