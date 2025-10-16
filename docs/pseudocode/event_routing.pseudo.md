# Event Routing System - Pseudocode

**Module:** Event Routing and Dispatching (FR-1)
**Purpose:** Route GitHub Actions events to appropriate scan handlers
**Source Reference:** SPARC_SPECIFICATION.md Section 3.1 FR-1
**Date:** October 16, 2025
**Version:** 1.0

---

## Table of Contents

1. [Main Event Dispatcher](#1-main-event-dispatcher)
2. [Push Event Handler](#2-push-event-handler)
3. [Pull Request Event Handler](#3-pull-request-event-handler)
4. [Workflow Dispatch Handler](#4-workflow-dispatch-handler)
5. [Schedule Event Handler](#5-schedule-event-handler)
6. [Supporting Algorithms](#6-supporting-algorithms)
7. [Data Structures](#7-data-structures)
8. [Error Conditions](#8-error-conditions)

---

## 1. Main Event Dispatcher

### ALGORITHM: EventDispatcher
**Purpose:** Main entry point that routes events to appropriate handlers

**INPUT:**
- event_name (string): GitHub event type from GITHUB_EVENT_NAME
- event_payload (JSON): Event data from GITHUB_EVENT_PATH file
- environment_vars (Map<string, string>): All environment variables

**OUTPUT:**
- exit_code (integer): 0 (success), 1 (error), 2 (secrets found)

**STEPS:**

```
1. VALIDATE_INPUT:
   a. IF event_name is NULL OR empty THEN
      LOG_ERROR("Missing GITHUB_EVENT_NAME environment variable")
      RETURN 1

   b. IF event_payload is NULL OR invalid JSON THEN
      LOG_ERROR("Invalid or missing GITHUB_EVENT_PATH file")
      RETURN 1

   c. supported_events = ["push", "pull_request", "workflow_dispatch", "schedule"]
   d. IF event_name NOT IN supported_events THEN
      LOG_ERROR("Event type [" + event_name + "] is not supported")
      RETURN 1

2. PARSE_CONFIGURATION:
   config = CALL ParseConfiguration(environment_vars)
   // Returns: ScanConfiguration object with all settings

3. DETERMINE_REPOSITORY_CONTEXT:
   repo_context = CALL DetermineRepositoryContext(event_name, event_payload, environment_vars)
   // Handles special case for schedule events where repository may be undefined

4. VALIDATE_GITHUB_TOKEN:
   IF event_name == "pull_request" THEN
      IF environment_vars["GITHUB_TOKEN"] is NULL OR empty THEN
         LOG_ERROR("GITHUB_TOKEN is required for pull_request events")
         RETURN 1

5. CHECK_LICENSE_REQUIREMENT:
   // Note: Currently disabled in implementation but retained for specification
   IF config.license_validation_enabled THEN
      user_type = CALL GetGitHubUserType(repo_context.owner, environment_vars["GITHUB_TOKEN"])
      IF user_type == "Organization" THEN
         IF environment_vars["GITLEAKS_LICENSE"] is NULL OR empty THEN
            LOG_ERROR("GITLEAKS_LICENSE is required for organization accounts")
            RETURN 1
         validation_result = CALL ValidateLicense(environment_vars["GITLEAKS_LICENSE"], repo_context)
         IF validation_result.is_valid == FALSE THEN
            LOG_ERROR("License validation failed: " + validation_result.error_message)
            RETURN 1

6. SETUP_GITLEAKS:
   gitleaks_version = environment_vars["GITLEAKS_VERSION"] OR "8.24.3"
   IF gitleaks_version == "latest" THEN
      gitleaks_version = CALL FetchLatestGitleaksVersion(environment_vars["GITHUB_TOKEN"])
   gitleaks_path = CALL InstallGitleaks(gitleaks_version)
   IF gitleaks_path is NULL THEN
      LOG_ERROR("Failed to install gitleaks binary")
      RETURN 1

7. ROUTE_TO_EVENT_HANDLER:
   scan_result = NULL

   CASE event_name OF:
      "push":
         scan_result = CALL HandlePushEvent(event_payload, config, gitleaks_path)

      "pull_request":
         scan_result = CALL HandlePullRequestEvent(event_payload, config, gitleaks_path, environment_vars)

      "workflow_dispatch":
         scan_result = CALL HandleWorkflowDispatchEvent(event_payload, config, gitleaks_path)

      "schedule":
         scan_result = CALL HandleScheduleEvent(event_payload, config, gitleaks_path)

      DEFAULT:
         LOG_ERROR("Unexpected event type after validation: " + event_name)
         RETURN 1
   END CASE

8. GENERATE_JOB_SUMMARY:
   IF config.enable_summary == TRUE THEN
      CALL GenerateJobSummary(scan_result, repo_context)

9. UPLOAD_ARTIFACT:
   IF config.enable_upload_artifact == TRUE AND scan_result.exit_code == 2 THEN
      CALL UploadSarifArtifact("results.sarif")

10. RETURN_EXIT_CODE:
    RETURN scan_result.exit_code
```

---

## 2. Push Event Handler

### ALGORITHM: HandlePushEvent
**Purpose:** Process push events with incremental scanning

**INPUT:**
- event_payload (JSON): Push event payload
- config (ScanConfiguration): Configuration settings
- gitleaks_path (string): Path to gitleaks binary

**OUTPUT:**
- ScanResult: Contains exit_code, findings, and metadata

**STEPS:**

```
1. VALIDATE_EVENT_PAYLOAD:
   IF event_payload["commits"] is NULL THEN
      LOG_ERROR("Missing commits array in push event payload")
      RETURN ScanResult(exit_code=1, error="Invalid event payload")

2. CHECK_EMPTY_COMMITS:
   IF LENGTH(event_payload["commits"]) == 0 THEN
      LOG_INFO("No commits to scan in push event")
      RETURN ScanResult(exit_code=0, message="No commits")

3. DETERMINE_COMMIT_RANGE:
   commits = event_payload["commits"]
   base_ref = commits[0]["id"]
   head_ref = commits[LENGTH(commits) - 1]["id"]

   // Validate commit SHAs
   IF NOT IsValidCommitSHA(base_ref) OR NOT IsValidCommitSHA(head_ref) THEN
      LOG_ERROR("Invalid commit SHA in push event")
      RETURN ScanResult(exit_code=1, error="Invalid commit SHA")

4. APPLY_BASE_REF_OVERRIDE:
   IF config.base_ref_override is NOT NULL THEN
      LOG_INFO("Overriding baseRef with: " + config.base_ref_override)
      base_ref = config.base_ref_override

5. GENERATE_LOG_OPTS:
   IF base_ref == head_ref THEN
      // Single commit optimization
      log_opts = "-1"
      LOG_INFO("Scanning single commit: " + head_ref)
   ELSE
      // Multiple commits - use range with merge filtering
      log_opts = "--no-merges --first-parent " + base_ref + "^.." + head_ref
      LOG_INFO("Scanning commit range: " + base_ref + "^.." + head_ref)

6. BUILD_GITLEAKS_ARGUMENTS:
   args = [
      "detect",
      "--redact",
      "-v",
      "--exit-code=2",
      "--report-format=sarif",
      "--report-path=results.sarif",
      "--log-level=debug",
      "--log-opts=" + log_opts
   ]

   // Add config file if specified
   IF config.gitleaks_config_path is NOT NULL THEN
      args.APPEND("--config=" + config.gitleaks_config_path)

7. EXECUTE_GITLEAKS_SCAN:
   LOG_INFO("Executing: gitleaks " + JOIN(args, " "))
   execution_result = CALL ExecuteProcess(gitleaks_path, args)

   // Capture stdout, stderr, and exit code
   stdout = execution_result.stdout
   stderr = execution_result.stderr
   exit_code = execution_result.exit_code

   LOG_DEBUG("Gitleaks stdout: " + stdout)
   LOG_DEBUG("Gitleaks stderr: " + stderr)
   LOG_INFO("Gitleaks exit code: " + exit_code)

8. HANDLE_EXIT_CODE:
   CASE exit_code OF:
      0:
         LOG_INFO("No secrets detected")
         RETURN ScanResult(exit_code=0, message="No leaks detected")

      1:
         LOG_ERROR("Gitleaks execution error")
         RETURN ScanResult(exit_code=1, error=stderr)

      2:
         LOG_WARNING("Secrets detected in commits")
         sarif_results = CALL ParseSarifFile("results.sarif")
         RETURN ScanResult(
            exit_code=2,
            findings=sarif_results,
            base_ref=base_ref,
            head_ref=head_ref
         )

      DEFAULT:
         LOG_ERROR("Unexpected gitleaks exit code: " + exit_code)
         RETURN ScanResult(exit_code=exit_code, error="Unexpected exit code")
   END CASE
```

---

## 3. Pull Request Event Handler

### ALGORITHM: HandlePullRequestEvent
**Purpose:** Process pull request events with PR-specific logic and commenting

**INPUT:**
- event_payload (JSON): Pull request event payload
- config (ScanConfiguration): Configuration settings
- gitleaks_path (string): Path to gitleaks binary
- environment_vars (Map<string, string>): Environment variables

**OUTPUT:**
- ScanResult: Contains exit_code, findings, comments posted

**STEPS:**

```
1. VALIDATE_EVENT_PAYLOAD:
   IF event_payload["pull_request"] is NULL THEN
      LOG_ERROR("Missing pull_request object in event payload")
      RETURN ScanResult(exit_code=1, error="Invalid PR event payload")

   pr_number = event_payload["pull_request"]["number"]
   IF pr_number is NULL THEN
      LOG_ERROR("Missing pull request number")
      RETURN ScanResult(exit_code=1, error="Missing PR number")

2. EXTRACT_REPOSITORY_INFO:
   full_name = event_payload["repository"]["full_name"]
   [owner, repo] = SPLIT(full_name, "/")

   IF owner is NULL OR repo is NULL THEN
      LOG_ERROR("Invalid repository full_name: " + full_name)
      RETURN ScanResult(exit_code=1, error="Invalid repository")

3. FETCH_PR_COMMITS:
   github_token = environment_vars["GITHUB_TOKEN"]
   api_url = environment_vars["GITHUB_API_URL"] OR "https://api.github.com"

   TRY:
      commits_response = CALL GitHubAPI(
         method="GET",
         url=api_url + "/repos/" + owner + "/" + repo + "/pulls/" + pr_number + "/commits",
         auth_token=github_token
      )
   CATCH error:
      LOG_ERROR("Failed to fetch PR commits: " + error.message)
      RETURN ScanResult(exit_code=1, error="GitHub API error")

   IF LENGTH(commits_response.data) == 0 THEN
      LOG_INFO("No commits found in pull request")
      RETURN ScanResult(exit_code=0, message="No commits in PR")

4. DETERMINE_COMMIT_RANGE:
   pr_commits = commits_response.data
   base_ref = pr_commits[0]["sha"]
   head_ref = pr_commits[LENGTH(pr_commits) - 1]["sha"]

   // Validate commit SHAs
   IF NOT IsValidCommitSHA(base_ref) OR NOT IsValidCommitSHA(head_ref) THEN
      LOG_ERROR("Invalid commit SHA in PR commits")
      RETURN ScanResult(exit_code=1, error="Invalid commit SHA")

5. APPLY_BASE_REF_OVERRIDE:
   IF config.base_ref_override is NOT NULL THEN
      LOG_INFO("Overriding baseRef with: " + config.base_ref_override)
      base_ref = config.base_ref_override

6. GENERATE_LOG_OPTS:
   // Always use range scan for PRs (even single commit)
   log_opts = "--no-merges --first-parent " + base_ref + "^.." + head_ref
   LOG_INFO("Scanning PR commit range: " + base_ref + "^.." + head_ref)

7. BUILD_GITLEAKS_ARGUMENTS:
   args = [
      "detect",
      "--redact",
      "-v",
      "--exit-code=2",
      "--report-format=sarif",
      "--report-path=results.sarif",
      "--log-level=debug",
      "--log-opts=" + log_opts
   ]

   IF config.gitleaks_config_path is NOT NULL THEN
      args.APPEND("--config=" + config.gitleaks_config_path)

8. EXECUTE_GITLEAKS_SCAN:
   LOG_INFO("Executing: gitleaks " + JOIN(args, " "))
   execution_result = CALL ExecuteProcess(gitleaks_path, args)
   exit_code = execution_result.exit_code

   LOG_INFO("Gitleaks exit code: " + exit_code)

9. HANDLE_SCAN_RESULTS:
   IF exit_code == 0 THEN
      LOG_INFO("No secrets detected in PR")
      RETURN ScanResult(exit_code=0, message="No leaks detected")

   ELSE IF exit_code == 1 THEN
      LOG_ERROR("Gitleaks execution error")
      RETURN ScanResult(exit_code=1, error=execution_result.stderr)

   ELSE IF exit_code == 2 THEN
      LOG_WARNING("Secrets detected in PR")
      sarif_results = CALL ParseSarifFile("results.sarif")

      // Post PR comments if enabled
      IF config.enable_comments == TRUE THEN
         posted_comments = CALL PostPRComments(
            owner=owner,
            repo=repo,
            pr_number=pr_number,
            sarif_results=sarif_results,
            github_token=github_token,
            api_url=api_url,
            notify_users=config.notify_user_list
         )

         RETURN ScanResult(
            exit_code=2,
            findings=sarif_results,
            comments_posted=posted_comments,
            base_ref=base_ref,
            head_ref=head_ref
         )
      ELSE
         LOG_INFO("PR comments disabled via configuration")
         RETURN ScanResult(
            exit_code=2,
            findings=sarif_results,
            base_ref=base_ref,
            head_ref=head_ref
         )

   ELSE
      LOG_ERROR("Unexpected gitleaks exit code: " + exit_code)
      RETURN ScanResult(exit_code=exit_code, error="Unexpected exit code")
```

---

## 4. Workflow Dispatch Handler

### ALGORITHM: HandleWorkflowDispatchEvent
**Purpose:** Process manual workflow dispatch events with full repository scan

**INPUT:**
- event_payload (JSON): Workflow dispatch event payload
- config (ScanConfiguration): Configuration settings
- gitleaks_path (string): Path to gitleaks binary

**OUTPUT:**
- ScanResult: Contains exit_code and findings

**STEPS:**

```
1. VALIDATE_EVENT_PAYLOAD:
   IF event_payload["repository"] is NULL THEN
      LOG_ERROR("Missing repository information in workflow_dispatch event")
      RETURN ScanResult(exit_code=1, error="Invalid event payload")

2. LOG_SCAN_MODE:
   LOG_INFO("Starting full repository scan (workflow_dispatch event)")
   LOG_INFO("No log-opts will be used - scanning entire git history")

3. BUILD_GITLEAKS_ARGUMENTS:
   // No log-opts for full repository scan
   args = [
      "detect",
      "--redact",
      "-v",
      "--exit-code=2",
      "--report-format=sarif",
      "--report-path=results.sarif",
      "--log-level=debug"
   ]

   // Add config file if specified
   IF config.gitleaks_config_path is NOT NULL THEN
      args.APPEND("--config=" + config.gitleaks_config_path)

4. EXECUTE_GITLEAKS_SCAN:
   LOG_INFO("Executing: gitleaks " + JOIN(args, " "))
   execution_result = CALL ExecuteProcess(gitleaks_path, args)

   stdout = execution_result.stdout
   stderr = execution_result.stderr
   exit_code = execution_result.exit_code

   LOG_DEBUG("Gitleaks stdout: " + stdout)
   LOG_DEBUG("Gitleaks stderr: " + stderr)
   LOG_INFO("Gitleaks exit code: " + exit_code)

5. HANDLE_EXIT_CODE:
   CASE exit_code OF:
      0:
         LOG_INFO("Full repository scan complete - no secrets detected")
         RETURN ScanResult(exit_code=0, message="No leaks detected")

      1:
         LOG_ERROR("Gitleaks execution error during full scan")
         RETURN ScanResult(exit_code=1, error=stderr)

      2:
         LOG_WARNING("Secrets detected in repository")
         sarif_results = CALL ParseSarifFile("results.sarif")
         RETURN ScanResult(
            exit_code=2,
            findings=sarif_results,
            scan_type="full_repository"
         )

      DEFAULT:
         LOG_ERROR("Unexpected gitleaks exit code: " + exit_code)
         RETURN ScanResult(exit_code=exit_code, error="Unexpected exit code")
   END CASE
```

---

## 5. Schedule Event Handler

### ALGORITHM: HandleScheduleEvent
**Purpose:** Process scheduled events with special repository handling

**INPUT:**
- event_payload (JSON): Schedule event payload
- config (ScanConfiguration): Configuration settings
- gitleaks_path (string): Path to gitleaks binary

**OUTPUT:**
- ScanResult: Contains exit_code and findings

**STEPS:**

```
1. HANDLE_UNDEFINED_REPOSITORY:
   // Special case: schedule events may have undefined repository
   IF event_payload["repository"] is NULL OR event_payload["repository"] is UNDEFINED THEN
      LOG_INFO("Repository undefined in schedule event - reconstructing from environment")

      // Reconstruct repository object from environment variables
      repo_owner = GET_ENV("GITHUB_REPOSITORY_OWNER")
      repo_full_name = GET_ENV("GITHUB_REPOSITORY")

      IF repo_owner is NULL OR repo_full_name is NULL THEN
         LOG_ERROR("Cannot determine repository from environment variables")
         RETURN ScanResult(exit_code=1, error="Missing repository information")

      // Extract repo name from full_name (remove owner prefix)
      repo_name = REPLACE(repo_full_name, repo_owner + "/", "")

      // Reconstruct repository object
      event_payload["repository"] = {
         "owner": {
            "login": repo_owner
         },
         "full_name": repo_full_name,
         "name": repo_name
      }

      LOG_INFO("Reconstructed repository: " + repo_full_name)

2. LOG_SCAN_MODE:
   LOG_INFO("Starting scheduled full repository scan")
   LOG_INFO("No log-opts will be used - scanning entire git history")

3. BUILD_GITLEAKS_ARGUMENTS:
   // No log-opts for full repository scan
   args = [
      "detect",
      "--redact",
      "-v",
      "--exit-code=2",
      "--report-format=sarif",
      "--report-path=results.sarif",
      "--log-level=debug"
   ]

   // Add config file if specified
   IF config.gitleaks_config_path is NOT NULL THEN
      args.APPEND("--config=" + config.gitleaks_config_path)

4. EXECUTE_GITLEAKS_SCAN:
   LOG_INFO("Executing: gitleaks " + JOIN(args, " "))
   execution_result = CALL ExecuteProcess(gitleaks_path, args)

   stdout = execution_result.stdout
   stderr = execution_result.stderr
   exit_code = execution_result.exit_code

   LOG_DEBUG("Gitleaks stdout: " + stdout)
   LOG_DEBUG("Gitleaks stderr: " + stderr)
   LOG_INFO("Gitleaks exit code: " + exit_code)

5. HANDLE_EXIT_CODE:
   CASE exit_code OF:
      0:
         LOG_INFO("Scheduled scan complete - no secrets detected")
         RETURN ScanResult(exit_code=0, message="No leaks detected")

      1:
         LOG_ERROR("Gitleaks execution error during scheduled scan")
         RETURN ScanResult(exit_code=1, error=stderr)

      2:
         LOG_WARNING("Secrets detected in scheduled scan")
         sarif_results = CALL ParseSarifFile("results.sarif")
         RETURN ScanResult(
            exit_code=2,
            findings=sarif_results,
            scan_type="scheduled_full_scan"
         )

      DEFAULT:
         LOG_ERROR("Unexpected gitleaks exit code: " + exit_code)
         RETURN ScanResult(exit_code=exit_code, error="Unexpected exit code")
   END CASE
```

---

## 6. Supporting Algorithms

### ALGORITHM: ParseConfiguration
**Purpose:** Parse and validate all configuration from environment variables

**INPUT:**
- environment_vars (Map<string, string>): All environment variables

**OUTPUT:**
- ScanConfiguration: Validated configuration object

**STEPS:**

```
1. PARSE_GITLEAKS_VERSION:
   version = environment_vars["GITLEAKS_VERSION"]
   IF version is NULL OR version is EMPTY THEN
      version = "8.24.3"  // Default version
   LOG_INFO("Gitleaks version: " + version)

2. PARSE_CONFIG_PATH:
   config_path = environment_vars["GITLEAKS_CONFIG"]
   IF config_path is NULL OR config_path is EMPTY THEN
      // Auto-detect gitleaks.toml in workspace
      workspace = environment_vars["GITHUB_WORKSPACE"]
      auto_config_path = workspace + "/gitleaks.toml"
      IF FileExists(auto_config_path) THEN
         config_path = auto_config_path
         LOG_INFO("Auto-detected config file: " + config_path)
      ELSE
         config_path = NULL
         LOG_INFO("No config file specified - using gitleaks defaults")
   ELSE
      IF NOT FileExists(config_path) THEN
         LOG_ERROR("Specified config file does not exist: " + config_path)
         config_path = NULL
      ELSE
         LOG_INFO("Using config file: " + config_path)

3. PARSE_BOOLEAN_FLAGS:
   // Parse GITLEAKS_ENABLE_SUMMARY (default: true)
   enable_summary = TRUE
   summary_value = environment_vars["GITLEAKS_ENABLE_SUMMARY"]
   IF summary_value == "false" OR summary_value == "0" THEN
      enable_summary = FALSE
   LOG_INFO("Enable summary: " + enable_summary)

   // Parse GITLEAKS_ENABLE_UPLOAD_ARTIFACT (default: true)
   enable_upload_artifact = TRUE
   artifact_value = environment_vars["GITLEAKS_ENABLE_UPLOAD_ARTIFACT"]
   IF artifact_value == "false" OR artifact_value == "0" THEN
      enable_upload_artifact = FALSE
   LOG_INFO("Enable upload artifact: " + enable_upload_artifact)

   // Parse GITLEAKS_ENABLE_COMMENTS (default: true)
   enable_comments = TRUE
   comments_value = environment_vars["GITLEAKS_ENABLE_COMMENTS"]
   IF comments_value == "false" OR comments_value == "0" THEN
      enable_comments = FALSE
   LOG_INFO("Enable PR comments: " + enable_comments)

4. PARSE_NOTIFY_USER_LIST:
   notify_users = environment_vars["GITLEAKS_NOTIFY_USER_LIST"]
   IF notify_users is NOT NULL AND notify_users is NOT EMPTY THEN
      // Parse comma-separated list and trim whitespace
      user_list = SPLIT(notify_users, ",")
      user_list = MAP(user_list, TRIM)
      user_list = FILTER(user_list, NOT_EMPTY)
      LOG_INFO("Notify users: " + JOIN(user_list, ", "))
   ELSE
      user_list = []

5. PARSE_BASE_REF_OVERRIDE:
   base_ref_override = environment_vars["BASE_REF"]
   IF base_ref_override is NOT NULL AND base_ref_override is NOT EMPTY THEN
      LOG_INFO("Base ref override: " + base_ref_override)
   ELSE
      base_ref_override = NULL

6. PARSE_LICENSE_SETTINGS:
   license_key = environment_vars["GITLEAKS_LICENSE"]
   license_validation_enabled = FALSE  // Currently disabled in implementation

   IF license_key is NOT NULL AND license_key is NOT EMPTY THEN
      LOG_INFO("License key present (validation currently disabled)")
   ELSE
      LOG_INFO("No license key provided")

7. CREATE_CONFIGURATION_OBJECT:
   RETURN ScanConfiguration {
      gitleaks_version: version,
      gitleaks_config_path: config_path,
      enable_summary: enable_summary,
      enable_upload_artifact: enable_upload_artifact,
      enable_comments: enable_comments,
      notify_user_list: user_list,
      base_ref_override: base_ref_override,
      license_key: license_key,
      license_validation_enabled: license_validation_enabled
   }
```

### ALGORITHM: DetermineRepositoryContext
**Purpose:** Extract repository information from event payload

**INPUT:**
- event_name (string): GitHub event type
- event_payload (JSON): Event data
- environment_vars (Map<string, string>): Environment variables

**OUTPUT:**
- RepositoryContext: Repository owner, name, full_name, url

**STEPS:**

```
1. HANDLE_SCHEDULE_EVENT:
   IF event_name == "schedule" THEN
      // Schedule events may have undefined repository
      IF event_payload["repository"] is NULL OR event_payload["repository"] is UNDEFINED THEN
         owner = environment_vars["GITHUB_REPOSITORY_OWNER"]
         full_name = environment_vars["GITHUB_REPOSITORY"]

         IF owner is NULL OR full_name is NULL THEN
            THROW Error("Cannot determine repository from schedule event")

         // Reconstruct context
         [owner_part, name_part] = SPLIT(full_name, "/")

         RETURN RepositoryContext {
            owner: owner_part,
            name: name_part,
            full_name: full_name,
            url: environment_vars["GITHUB_SERVER_URL"] + "/" + full_name
         }

2. EXTRACT_FROM_PAYLOAD:
   IF event_payload["repository"] is NULL THEN
      THROW Error("Missing repository in event payload")

   repo = event_payload["repository"]

   IF repo["owner"] is NULL OR repo["owner"]["login"] is NULL THEN
      THROW Error("Missing repository owner in event payload")

   owner = repo["owner"]["login"]
   full_name = repo["full_name"]

   // Extract name from full_name if not directly available
   IF repo["name"] is NOT NULL THEN
      name = repo["name"]
   ELSE
      [_, name] = SPLIT(full_name, "/")

3. CONSTRUCT_REPOSITORY_URL:
   server_url = environment_vars["GITHUB_SERVER_URL"] OR "https://github.com"
   repo_url = server_url + "/" + full_name

4. RETURN_CONTEXT:
   RETURN RepositoryContext {
      owner: owner,
      name: name,
      full_name: full_name,
      url: repo_url
   }
```

### ALGORITHM: PostPRComments
**Purpose:** Post inline review comments on pull request for detected secrets

**INPUT:**
- owner (string): Repository owner
- repo (string): Repository name
- pr_number (integer): Pull request number
- sarif_results (SarifReport): Parsed SARIF findings
- github_token (string): GitHub API authentication token
- api_url (string): GitHub API base URL
- notify_users (Array<string>): List of users to mention

**OUTPUT:**
- CommentResult: Number of comments posted, errors encountered

**STEPS:**

```
1. FETCH_EXISTING_COMMENTS:
   TRY:
      comments_response = CALL GitHubAPI(
         method="GET",
         url=api_url + "/repos/" + owner + "/" + repo + "/pulls/" + pr_number + "/comments",
         auth_token=github_token
      )
      existing_comments = comments_response.data
   CATCH error:
      LOG_WARNING("Failed to fetch existing comments: " + error.message)
      existing_comments = []

2. BUILD_EXISTING_COMMENTS_MAP:
   // Create map for efficient deduplication
   comment_map = {}
   FOR EACH comment IN existing_comments DO
      key = comment.path + ":" + comment.original_line + ":" + comment.body
      comment_map[key] = TRUE

3. ITERATE_FINDINGS:
   comments_posted = 0
   comments_skipped = 0
   errors = []

   FOR EACH result IN sarif_results.runs[0].results DO
      // Extract information from SARIF result
      rule_id = result.ruleId
      commit_sha = result.partialFingerprints.commitSha
      file_path = result.locations[0].physicalLocation.artifactLocation.uri
      start_line = result.locations[0].physicalLocation.region.startLine

      // Generate fingerprint for .gitleaksignore
      fingerprint = commit_sha + ":" + file_path + ":" + rule_id + ":" + start_line

4. BUILD_COMMENT_BODY:
      comment_body = "🛑 **Gitleaks** has detected a secret with rule-id `" + rule_id + "` in commit " + commit_sha + ".\n"
      comment_body += "If this secret is a _true_ positive, please rotate the secret ASAP.\n\n"
      comment_body += "If this secret is a _false_ positive, you can add the fingerprint below to your `.gitleaksignore` file and commit the change to this branch.\n\n"
      comment_body += "```\n"
      comment_body += "echo " + fingerprint + " >> .gitleaksignore\n"
      comment_body += "```\n"

      // Add user mentions if configured
      IF LENGTH(notify_users) > 0 THEN
         comment_body += "\n\ncc " + JOIN(notify_users, ",")

5. CHECK_FOR_DUPLICATE:
      dedup_key = file_path + ":" + start_line + ":" + comment_body
      IF comment_map[dedup_key] == TRUE THEN
         LOG_INFO("Skipping duplicate comment for " + file_path + ":" + start_line)
         comments_skipped += 1
         CONTINUE

6. POST_COMMENT:
      proposed_comment = {
         owner: owner,
         repo: repo,
         pull_number: pr_number,
         body: comment_body,
         commit_id: commit_sha,
         path: file_path,
         side: "RIGHT",
         line: start_line
      }

      TRY:
         CALL GitHubAPI(
            method="POST",
            url=api_url + "/repos/" + owner + "/" + repo + "/pulls/" + pr_number + "/comments",
            auth_token=github_token,
            body=proposed_comment
         )
         comments_posted += 1
         LOG_INFO("Posted comment on " + file_path + ":" + start_line)

      CATCH error:
         LOG_WARNING("Failed to post comment on " + file_path + ":" + start_line)
         LOG_WARNING("Error: " + error.message)
         LOG_WARNING("Likely due to diff size limitations")
         errors.APPEND(error)
   END FOR

7. RETURN_RESULTS:
   LOG_INFO("Comments posted: " + comments_posted)
   LOG_INFO("Comments skipped (duplicates): " + comments_skipped)

   RETURN CommentResult {
      posted: comments_posted,
      skipped: comments_skipped,
      errors: errors
   }
```

### ALGORITHM: IsValidCommitSHA
**Purpose:** Validate commit SHA format

**INPUT:**
- sha (string): Commit SHA to validate

**OUTPUT:**
- boolean: TRUE if valid, FALSE otherwise

**STEPS:**

```
1. CHECK_LENGTH:
   IF sha is NULL OR LENGTH(sha) != 40 THEN
      RETURN FALSE

2. CHECK_HEX_CHARACTERS:
   hex_pattern = "^[0-9a-fA-F]{40}$"
   RETURN REGEX_MATCH(sha, hex_pattern)
```

---

## 7. Data Structures

### ScanConfiguration
```
STRUCTURE ScanConfiguration:
   gitleaks_version: string
   gitleaks_config_path: string OR NULL
   enable_summary: boolean
   enable_upload_artifact: boolean
   enable_comments: boolean
   notify_user_list: Array<string>
   base_ref_override: string OR NULL
   license_key: string OR NULL
   license_validation_enabled: boolean
```

### ScanResult
```
STRUCTURE ScanResult:
   exit_code: integer (0, 1, or 2)
   message: string OR NULL
   error: string OR NULL
   findings: SarifReport OR NULL
   base_ref: string OR NULL
   head_ref: string OR NULL
   scan_type: string OR NULL
   comments_posted: CommentResult OR NULL
```

### RepositoryContext
```
STRUCTURE RepositoryContext:
   owner: string
   name: string
   full_name: string
   url: string
```

### CommentResult
```
STRUCTURE CommentResult:
   posted: integer
   skipped: integer
   errors: Array<Error>
```

### ExecutionResult
```
STRUCTURE ExecutionResult:
   stdout: string
   stderr: string
   exit_code: integer
```

---

## 8. Error Conditions

### Critical Errors (Exit 1 Immediately)

1. **Unsupported Event Type**
   - Condition: event_name NOT IN ["push", "pull_request", "workflow_dispatch", "schedule"]
   - Action: Log error, exit 1
   - Message: "Event type [X] is not supported"

2. **Missing GITHUB_TOKEN for PR Events**
   - Condition: event_name == "pull_request" AND GITHUB_TOKEN is NULL/empty
   - Action: Log error, exit 1
   - Message: "GITHUB_TOKEN is required for pull_request events"

3. **Missing License for Organizations**
   - Condition: user_type == "Organization" AND GITLEAKS_LICENSE is NULL/empty
   - Action: Log error, exit 1
   - Message: "GITLEAKS_LICENSE is required for organization accounts"

4. **Invalid Event Payload**
   - Condition: Event JSON is malformed or missing required fields
   - Action: Log error, exit 1
   - Message: "Invalid or missing event payload"

5. **Gitleaks Installation Failure**
   - Condition: Cannot download or extract gitleaks binary
   - Action: Log error, exit 1
   - Message: "Failed to install gitleaks binary"

6. **Gitleaks Execution Error (Exit Code 1)**
   - Condition: Gitleaks exits with code 1
   - Action: Log error, exit 1
   - Message: "Gitleaks execution error: [stderr]"

### Non-Fatal Errors (Log Warning, Continue)

1. **GitHub API User Lookup Failure**
   - Condition: GET /users/{username} fails
   - Action: Log warning, assume organization, require license
   - Message: "Failed to determine user type, enforcing license validation"

2. **Cache Operation Failure**
   - Condition: Cache save/restore fails
   - Action: Log warning, download fresh binary
   - Message: "Cache operation failed: [error]"

3. **PR Comment Creation Failure**
   - Condition: POST /repos/{owner}/{repo}/pulls/{number}/comments fails
   - Action: Log warning, continue (secrets still in summary/artifacts)
   - Message: "Failed to post PR comment (likely diff too large)"

4. **Artifact Upload Failure**
   - Condition: Artifact upload API fails
   - Action: Log warning, continue (continueOnError=true)
   - Message: "Failed to upload artifact: [error]"

### Special Cases

1. **Empty Commits (Push Event)**
   - Condition: event_payload.commits.length == 0
   - Action: Log info, exit 0 (success)
   - Message: "No commits to scan"

2. **Secrets Detected (Exit Code 2)**
   - Condition: Gitleaks exits with code 2
   - Action: Process results (comments, summary, artifacts), THEN exit 1
   - Message: "Secrets detected, see job summary for details"

3. **Schedule Event with Undefined Repository**
   - Condition: event_name == "schedule" AND event_payload.repository is NULL
   - Action: Reconstruct repository from environment variables
   - Message: "Repository undefined in schedule event - reconstructing"

4. **Single Commit Push**
   - Condition: base_ref == head_ref in push event
   - Action: Use `--log-opts=-1` instead of range
   - Message: "Scanning single commit"

---

## Implementation Notes

### Exit Code Flow
```
┌─────────────────────────────────────────────────────┐
│ Gitleaks Exit Code                                  │
├─────────────────────────────────────────────────────┤
│ 0 → No secrets     → Action exits 0 (SUCCESS)       │
│ 1 → Error          → Action exits 1 (FAILURE)       │
│ 2 → Secrets found  → Process results → Exit 1       │
│                      (comments, summary, artifacts) │
└─────────────────────────────────────────────────────┘
```

### Event-Specific Log-Opts
```
┌──────────────────────┬─────────────────────────────────────┐
│ Event Type           │ Log-Opts                            │
├──────────────────────┼─────────────────────────────────────┤
│ push (single)        │ -1                                  │
│ push (range)         │ --no-merges --first-parent BASE^..HEAD │
│ pull_request         │ --no-merges --first-parent BASE^..HEAD │
│ workflow_dispatch    │ (none - full scan)                  │
│ schedule             │ (none - full scan)                  │
└──────────────────────┴─────────────────────────────────────┘
```

### Boolean Parsing Logic
```
FALSE values:
  - "false" (string)
  - "0" (string)

TRUE values:
  - "true" (string)
  - "1" (string)
  - Any other non-empty value
  - NULL/undefined (defaults to TRUE)
```

### Commit SHA Validation
```
Valid SHA: 40-character hexadecimal string
Pattern: ^[0-9a-fA-F]{40}$
Example: abc123def456789012345678901234567890abcd
```

---

## End of Pseudocode Document

**Status:** Complete
**Next Phase:** Architecture Design (if proceeding with SPARC methodology)
**Related Documents:**
- /workspaces/SecretScout/docs/SPARC_SPECIFICATION.md
- /workspaces/SecretScout/gitleaks-action/src/index.js (original implementation)
- /workspaces/SecretScout/gitleaks-action/src/gitleaks.js (original implementation)
