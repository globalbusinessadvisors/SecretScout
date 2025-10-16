# Gitleaks Integration Flow Diagrams
## Visual Reference for Rust Implementation

**Companion Document to**: GITLEAKS_INTEGRATION_SPEC.md
**Date**: 2025-10-15

---

## 1. MAIN EXECUTION FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                         GitHub Action Starts                         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Load Event JSON      │
                    │  (GITHUB_EVENT_PATH)  │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Validate Event Type  │
                    │  (push, PR, etc.)     │
                    └───────────┬───────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
                ▼                               ▼
        Not Supported                    Supported Event
                │                               │
                ▼                               ▼
        ┌──────────────┐          ┌────────────────────────┐
        │  Log Error   │          │  Detect User Type      │
        │  Exit(1)     │          │  (Org vs Personal)     │
        └──────────────┘          └────────┬───────────────┘
                                           │
                                           ▼
                               ┌───────────────────────┐
                               │  License Validation   │
                               │  (Orgs only)          │
                               └───────────┬───────────┘
                                           │
                                           ▼
                               ┌───────────────────────┐
                               │  Determine Gitleaks   │
                               │  Version              │
                               └───────────┬───────────┘
                                           │
                                           ▼
                               ┌───────────────────────┐
                               │  Install/Cache        │
                               │  Gitleaks Binary      │
                               └───────────┬───────────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
            ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
            │ Push Event   │      │ PR Event     │      │ Dispatch/    │
            │              │      │              │      │ Schedule     │
            └──────┬───────┘      └──────┬───────┘      └──────┬───────┘
                   │                     │                     │
                   ▼                     ▼                     ▼
         ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
         │ Extract Commits │   │ Fetch PR        │   │ Full Repo Scan  │
         │ from Event      │   │ Commits (API)   │   │                 │
         └─────────┬───────┘   └─────────┬───────┘   └─────────┬───────┘
                   │                     │                     │
                   └──────────┬──────────┘                     │
                              │                                │
                              └────────────┬───────────────────┘
                                           │
                                           ▼
                                ┌──────────────────────┐
                                │  Execute Gitleaks    │
                                │  Binary              │
                                └──────────┬───────────┘
                                           │
                                           ▼
                                ┌──────────────────────┐
                                │  Capture Exit Code   │
                                └──────────┬───────────┘
                                           │
                    ┌──────────────────────┼──────────────────────┐
                    │                      │                      │
                    ▼                      ▼                      ▼
            ┌──────────────┐      ┌──────────────┐      ┌──────────────┐
            │ Exit Code 0  │      │ Exit Code 2  │      │ Exit Code 1+ │
            │ (No Leaks)   │      │ (Leaks)      │      │ (Error)      │
            └──────┬───────┘      └──────┬───────┘      └──────┬───────┘
                   │                     │                     │
                   ▼                     ▼                     ▼
         ┌─────────────────┐   ┌─────────────────┐   ┌─────────────────┐
         │ Success Summary │   │ Parse SARIF     │   │ Error Summary   │
         │ Upload Artifact │   │ Create Comments │   │ Exit(exitCode)  │
         │ Exit(0)         │   │ Create Summary  │   │                 │
         │                 │   │ Upload Artifact │   │                 │
         │                 │   │ Exit(1)         │   │                 │
         └─────────────────┘   └─────────────────┘   └─────────────────┘
```

---

## 2. GITLEAKS BINARY EXECUTION FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Prepare Gitleaks Execution                        │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Build Base Arguments │
                    │  - detect             │
                    │  - --redact           │
                    │  - -v                 │
                    │  - --exit-code=2      │
                    │  - --report-format... │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Determine Event Type │
                    └───────────┬───────────┘
                                │
                ┌───────────────┼───────────────┐
                │               │               │
                ▼               ▼               ▼
        ┌───────────┐   ┌───────────┐   ┌────────────┐
        │   Push    │   │    PR     │   │  Dispatch  │
        │           │   │           │   │  Schedule  │
        └─────┬─────┘   └─────┬─────┘   └──────┬─────┘
              │               │                │
              ▼               ▼                ▼
     ┌────────────────┐  ┌────────────────┐  No additional
     │ baseRef ==     │  │ Add --log-opts │  arguments
     │ headRef?       │  │ with PR range  │
     └────┬─────┬─────┘  └────────────────┘
          │     │
      Yes │     │ No
          │     │
          ▼     ▼
     ┌────────┐ ┌─────────────────┐
     │--log-  │ │--log-opts=      │
     │opts=-1 │ │--no-merges      │
     │        │ │--first-parent   │
     │        │ │base^..head      │
     └────┬───┘ └────────┬────────┘
          │              │
          └──────┬───────┘
                 │
                 ▼
     ┌────────────────────────┐
     │  Environment Variables │
     │  - GITLEAKS_CONFIG     │
     │  - GITLEAKS_LICENSE    │
     │  - (passed implicitly) │
     └────────────┬───────────┘
                  │
                  ▼
     ┌────────────────────────┐
     │  Execute Command       │
     │  ignoreReturnCode:true │
     │  timeout: 60s          │
     └────────────┬───────────┘
                  │
                  ▼
     ┌────────────────────────┐
     │  Return Exit Code      │
     │  (0, 1, or 2)          │
     └────────────────────────┘
```

---

## 3. SARIF PARSING AND PROCESSING FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Exit Code == 2                                │
│                     (Leaks Detected)                                 │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Read results.sarif   │
                    └───────────┬───────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │  Parse JSON           │
                    └───────────┬───────────┘
                                │
                        ┌───────┴───────┐
                        │               │
                    Success         Failure
                        │               │
                        ▼               ▼
            ┌───────────────────┐  ┌──────────────┐
            │ Extract Results   │  │ Log Error    │
            │ Array             │  │ Skip to Exit │
            └─────────┬─────────┘  └──────────────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Iterate Results   │
            └─────────┬─────────┘
                      │
        ┌─────────────┴─────────────┐
        │                           │
        ▼                           ▼
┌───────────────────┐      ┌────────────────────┐
│ For Each Result:  │      │ Collect for        │
│                   │      │ Summary Table      │
│ 1. Extract Fields │      │                    │
│   - ruleId        │      │ Fields:            │
│   - commitSha     │      │ - Rule ID          │
│   - filePath      │      │ - Commit (short)   │
│   - startLine     │      │ - Secret URL       │
│   - author        │      │ - Start Line       │
│   - email         │      │ - Author           │
│   - date          │      │ - Date             │
│                   │      │ - Email            │
│ 2. Generate       │      │ - File             │
│    Fingerprint    │      │                    │
│    (SHA:file:     │      │ Generate URLs:     │
│     rule:line)    │      │ - Commit URL       │
│                   │      │ - Secret URL       │
│ 3. Construct      │      │ - File URL         │
│    Comment Body   │      │                    │
│                   │      └────────────────────┘
└─────────┬─────────┘
          │
          ▼
  ┌───────────────────┐
  │ Is PR Event?      │
  └─────┬─────────────┘
        │
    ┌───┴───┐
    │       │
   Yes     No
    │       │
    ▼       └──────────┐
┌─────────────────┐    │
│ Create PR       │    │
│ Comment         │    │
│ (see flow 4)    │    │
└─────────────────┘    │
                       │
          ┌────────────┘
          │
          ▼
  ┌───────────────────┐
  │ Generate Summary  │
  │ Table (HTML)      │
  └─────────┬─────────┘
            │
            ▼
  ┌───────────────────┐
  │ Write to GitHub   │
  │ Actions Summary   │
  └─────────┬─────────┘
            │
            ▼
  ┌───────────────────┐
  │ Upload Artifact   │
  │ (results.sarif)   │
  └─────────┬─────────┘
            │
            ▼
  ┌───────────────────┐
  │ Exit(1)           │
  │ (Fail Workflow)   │
  └───────────────────┘
```

---

## 4. PR COMMENT CREATION FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                  For Each SARIF Result (PR Event)                    │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Check Feature Flag    │
                    │ GITLEAKS_ENABLE_      │
                    │ COMMENTS == true?     │
                    └───────────┬───────────┘
                                │
                        ┌───────┴───────┐
                        │               │
                      Yes              No
                        │               │
                        ▼               ▼
            ┌───────────────────┐  ┌──────────┐
            │ Extract Data:     │  │ Skip     │
            │ - commitSha       │  │ Comments │
            │ - filePath        │  └──────────┘
            │ - ruleId          │
            │ - startLine       │
            └─────────┬─────────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Generate          │
            │ Fingerprint       │
            └─────────┬─────────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Build Comment     │
            │ Object:           │
            │ - owner           │
            │ - repo            │
            │ - pull_number     │
            │ - commit_id       │
            │ - path            │
            │ - side: "RIGHT"   │
            │ - line            │
            │ - body (template) │
            └─────────┬─────────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Check GITLEAKS_   │
            │ NOTIFY_USER_LIST  │
            └─────────┬─────────┘
                      │
                ┌─────┴─────┐
                │           │
              Set        Not Set
                │           │
                ▼           │
      ┌─────────────────┐  │
      │ Append cc list  │  │
      │ to comment body │  │
      └─────────┬───────┘  │
                │           │
                └─────┬─────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Fetch Existing    │
            │ PR Comments       │
            │ (GitHub API)      │
            └─────────┬─────────┘
                      │
                      ▼
            ┌───────────────────┐
            │ Check for         │
            │ Duplicate:        │
            │ - Same body       │
            │ - Same path       │
            │ - Same line       │
            └─────────┬─────────┘
                      │
                ┌─────┴─────┐
                │           │
           Duplicate     Unique
                │           │
                ▼           ▼
         ┌──────────┐  ┌────────────────────┐
         │ Skip     │  │ Create Review      │
         │ Comment  │  │ Comment (API)      │
         └──────────┘  └──────────┬─────────┘
                                  │
                          ┌───────┴───────┐
                          │               │
                      Success         Error
                          │               │
                          ▼               ▼
                    ┌──────────┐  ┌────────────────┐
                    │ Continue │  │ Log Warning    │
                    │          │  │ (likely large  │
                    │          │  │  diff issue)   │
                    │          │  │ Continue       │
                    └──────────┘  └────────────────┘
```

---

## 5. CONFIGURATION DISCOVERY FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                    Gitleaks Needs Configuration                      │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Check --config flag   │
                    │ (not used by action)  │
                    └───────────┬───────────┘
                                │
                              Not Set
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Check GITLEAKS_CONFIG │
                    │ env variable          │
                    └───────────┬───────────┘
                                │
                        ┌───────┴───────┐
                        │               │
                      Set            Not Set
                        │               │
                        ▼               ▼
            ┌───────────────────┐  ┌────────────────────┐
            │ Use specified     │  │ Check GITLEAKS_    │
            │ file path         │  │ CONFIG_TOML        │
            └───────────────────┘  │ (file content)     │
                                   └──────────┬─────────┘
                                              │
                                      ┌───────┴───────┐
                                      │               │
                                    Set            Not Set
                                      │               │
                                      ▼               ▼
                          ┌───────────────────┐  ┌──────────────────┐
                          │ Parse TOML        │  │ Check for        │
                          │ content from env  │  │ .gitleaks.toml   │
                          └───────────────────┘  │ in repo root     │
                                                 └──────┬───────────┘
                                                        │
                                                ┌───────┴───────┐
                                                │               │
                                             Exists       Not Exists
                                                │               │
                                                ▼               ▼
                                    ┌───────────────────┐  ┌──────────────┐
                                    │ Use .gitleaks.    │  │ Use Default  │
                                    │ toml from root    │  │ Gitleaks     │
                                    │                   │  │ Config       │
                                    └───────────────────┘  └──────────────┘
```

---

## 6. EVENT-SPECIFIC COMMIT RANGE DETECTION

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Event Type Decision                          │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
        ┌───────────────────────┼───────────────────────┐
        │                       │                       │
        ▼                       ▼                       ▼
┌───────────────┐       ┌───────────────┐      ┌──────────────────┐
│  Push Event   │       │   PR Event    │      │ Dispatch/        │
│               │       │               │      │ Schedule Event   │
└───────┬───────┘       └───────┬───────┘      └────────┬─────────┘
        │                       │                       │
        ▼                       ▼                       │
┌───────────────┐       ┌───────────────┐              │
│ Check commits │       │ Fetch PR      │              │
│ array length  │       │ commits via   │              │
│               │       │ GitHub API    │              │
└───────┬───────┘       └───────┬───────┘              │
        │                       │                       │
    ┌───┴────┐                  │                       │
    │        │                  │                       │
  0 commits  >0 commits          │                       │
    │        │                  │                       │
    ▼        ▼                  ▼                       │
┌────────┐ ┌─────────────┐  ┌──────────────┐          │
│ Exit 0 │ │ Extract:    │  │ Extract:     │          │
│        │ │ baseRef =   │  │ baseRef =    │          │
│        │ │ commits[0]  │  │ commits[0]   │          │
│        │ │             │  │              │          │
│        │ │ headRef =   │  │ headRef =    │          │
│        │ │ commits[-1] │  │ commits[-1]  │          │
│        │ └──────┬──────┘  └──────┬───────┘          │
│        │        │                │                   │
│        │        ▼                │                   │
│        │ ┌──────────────┐       │                   │
│        │ │ Check BASE_  │       │                   │
│        │ │ REF env var  │       │                   │
│        │ └──────┬───────┘       │                   │
│        │        │                │                   │
│        │    ┌───┴───┐            │                   │
│        │    │       │            │                   │
│        │   Set   Not Set         │                   │
│        │    │       │            │                   │
│        │    ▼       │            │                   │
│        │ ┌──────┐  │            │                   │
│        │ │Override│ │            │                   │
│        │ │baseRef │ │            │                   │
│        │ └───┬────┘ │            │                   │
│        │     │      │            │                   │
│        │     └──┬───┘            │                   │
│        │        │                │                   │
│        │        ▼                ▼                   │
│        │  ┌─────────────────────────┐               │
│        │  │ Compare base == head?   │               │
│        │  └────────┬────────────────┘               │
│        │           │                                 │
│        │      ┌────┴────┐                            │
│        │      │         │                            │
│        │    Equal   Not Equal                        │
│        │      │         │                            │
│        │      ▼         ▼                            │
│        │  ┌────────┐ ┌──────────────────┐           │
│        │  │--log-  │ │--log-opts=       │           │
│        │  │opts=-1 │ │--no-merges       │           │
│        │  │        │ │--first-parent    │           │
│        │  │        │ │base^..head       │           │
│        │  └────────┘ └──────────────────┘           │
│        │                                             │
│        └─────────────────────────────────────────────┤
│                                                      │
│                                                      ▼
│                                           ┌───────────────────┐
│                                           │ No --log-opts     │
│                                           │ (Full repo scan)  │
└───────────────────────────────────────────┴───────────────────┘
```

---

## 7. GITHUB ACTIONS SUMMARY GENERATION

```
┌─────────────────────────────────────────────────────────────────────┐
│                     After Gitleaks Execution                         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Check Feature Flag    │
                    │ GITLEAKS_ENABLE_      │
                    │ SUMMARY == true?      │
                    └───────────┬───────────┘
                                │
                        ┌───────┴───────┐
                        │               │
                      Yes              No
                        │               │
                        ▼               ▼
            ┌───────────────────┐  ┌──────────┐
            │ Check Exit Code   │  │ Skip     │
            └─────────┬─────────┘  │ Summary  │
                      │            └──────────┘
        ┌─────────────┼─────────────┐
        │             │             │
        ▼             ▼             ▼
┌───────────┐  ┌──────────┐  ┌──────────────┐
│ Code = 0  │  │ Code = 2 │  │ Code = 1 or  │
│           │  │          │  │ Other        │
└─────┬─────┘  └────┬─────┘  └──────┬───────┘
      │            │                │
      ▼            ▼                ▼
┌──────────────────┐  ┌──────────────────────┐  ┌────────────────┐
│ Success Summary  │  │ Leaks Summary        │  │ Error Summary  │
│                  │  │                      │  │                │
│ ┌──────────────┐ │  │ ┌──────────────────┐ │  │ ┌────────────┐ │
│ │   Heading:   │ │  │ │    Heading:      │ │  │ │  Heading:  │ │
│ │ No leaks ✅  │ │  │ │ Gitleaks detected│ │  │ │ Error ❌   │ │
│ └──────────────┘ │  │ │ secrets 🛑       │ │  │ │ Exit [N]   │ │
│                  │  │ └──────────────────┘ │  │ └────────────┘ │
│                  │  │                      │  │                │
│                  │  │ ┌──────────────────┐ │  │                │
│                  │  │ │   Table:         │ │  │                │
│                  │  │ │                  │ │  │                │
│                  │  │ │ Header Row:      │ │  │                │
│                  │  │ │ - Rule ID        │ │  │                │
│                  │  │ │ - Commit         │ │  │                │
│                  │  │ │ - Secret URL     │ │  │                │
│                  │  │ │ - Start Line     │ │  │                │
│                  │  │ │ - Author         │ │  │                │
│                  │  │ │ - Date           │ │  │                │
│                  │  │ │ - Email          │ │  │                │
│                  │  │ │ - File           │ │  │                │
│                  │  │ │                  │ │  │                │
│                  │  │ │ Data Rows:       │ │  │                │
│                  │  │ │ For each result  │ │  │                │
│                  │  │ │ from SARIF       │ │  │                │
│                  │  │ └──────────────────┘ │  │                │
│                  │  │                      │  │                │
└──────────────────┘  └──────────────────────┘  └────────────────┘
        │                      │                        │
        └──────────────────────┼────────────────────────┘
                               │
                               ▼
                    ┌───────────────────────┐
                    │ Write to GitHub       │
                    │ Actions Summary API   │
                    └───────────────────────┘
```

---

## 8. ARTIFACT UPLOAD FLOW

```
┌─────────────────────────────────────────────────────────────────────┐
│                     After Gitleaks Execution                         │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Check Feature Flag    │
                    │ GITLEAKS_ENABLE_      │
                    │ UPLOAD_ARTIFACT?      │
                    └───────────┬───────────┘
                                │
                        ┌───────┴───────┐
                        │               │
                      Yes              No
                        │               │
                        ▼               ▼
            ┌───────────────────┐  ┌──────────┐
            │ Check if          │  │ Skip     │
            │ results.sarif     │  │ Upload   │
            │ exists            │  └──────────┘
            └─────────┬─────────┘
                      │
              ┌───────┴───────┐
              │               │
           Exists         Not Exists
              │               │
              ▼               ▼
  ┌───────────────────┐  ┌──────────┐
  │ Create Artifact   │  │ Log      │
  │ Client            │  │ Warning  │
  └─────────┬─────────┘  └──────────┘
            │
            ▼
  ┌───────────────────┐
  │ Configure Upload: │
  │                   │
  │ - Name:           │
  │   gitleaks-       │
  │   results.sarif   │
  │                   │
  │ - Files:          │
  │   [results.sarif] │
  │                   │
  │ - Root Dir:       │
  │   $HOME           │
  │                   │
  │ - Options:        │
  │   continueOnError │
  │   = true          │
  └─────────┬─────────┘
            │
            ▼
  ┌───────────────────┐
  │ Call Upload API   │
  └─────────┬─────────┘
            │
    ┌───────┴───────┐
    │               │
  Success         Error
    │               │
    ▼               ▼
┌─────────┐   ┌──────────┐
│ Continue│   │ Log      │
│         │   │ Warning  │
│         │   │ Continue │
└─────────┘   └──────────┘
```

---

## 9. LICENSE VALIDATION FLOW (Currently Disabled)

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Start Validation                            │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │ Detect User Type      │
                    │ (GET /users/{user})   │
                    └───────────┬───────────┘
                                │
                ┌───────────────┴───────────────┐
                │                               │
                ▼                               ▼
        ┌───────────────┐             ┌─────────────────┐
        │ Organization  │             │ Personal User   │
        └───────┬───────┘             └────────┬────────┘
                │                              │
                ▼                              ▼
    ┌───────────────────┐          ┌──────────────────┐
    │ shouldValidate =  │          │ shouldValidate = │
    │ true              │          │ false            │
    └───────┬───────────┘          └────────┬─────────┘
            │                               │
            ▼                               │
┌───────────────────────┐                   │
│ Check GITLEAKS_       │                   │
│ LICENSE env var       │                   │
└───────────┬───────────┘                   │
            │                               │
    ┌───────┴───────┐                       │
    │               │                       │
  Set            Not Set                    │
    │               │                       │
    ▼               ▼                       │
┌─────────┐   ┌──────────┐                 │
│ Validate│   │ Error:   │                 │
│ with    │   │ License  │                 │
│ Keygen  │   │ Required │                 │
│ API     │   │ Exit(1)  │                 │
└─────────┘   └──────────┘                 │
    │                                      │
    │                                      │
    └──────────────┬───────────────────────┘
                   │
                   ▼
        ┌──────────────────┐
        │ Continue to Scan │
        └──────────────────┘
```

---

## 10. ERROR HANDLING DECISION TREE

```
                           ┌─────────────┐
                           │   Error     │
                           │  Occurred?  │
                           └──────┬──────┘
                                  │
        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
┌───────────────┐       ┌──────────────────┐      ┌─────────────────┐
│ Binary        │       │ SARIF Parsing    │      │ GitHub API      │
│ Execution     │       │ Error            │      │ Error           │
│ Error         │       │                  │      │                 │
└───────┬───────┘       └────────┬─────────┘      └────────┬────────┘
        │                        │                         │
        ▼                        ▼                         ▼
┌───────────────┐       ┌──────────────────┐      ┌─────────────────┐
│ Exit Code 1?  │       │ File not found   │      │ Comment Creation│
│               │       │ or Invalid JSON? │      │ Failed?         │
└───────┬───────┘       └────────┬─────────┘      └────────┬────────┘
        │                        │                         │
        ▼                        ▼                         ▼
┌───────────────┐       ┌──────────────────┐      ┌─────────────────┐
│ Log Error     │       │ Log Error        │      │ Log Warning     │
│ Set Summary   │       │ Skip Comment     │      │ Continue with   │
│ Exit(1)       │       │ Skip Summary     │      │ Other Comments  │
│               │       │ Continue         │      │                 │
└───────────────┘       └──────────────────┘      └─────────────────┘

        ┌─────────────────────────┼─────────────────────────┐
        │                         │                         │
        ▼                         ▼                         ▼
┌───────────────┐       ┌──────────────────┐      ┌─────────────────┐
│ Artifact      │       │ User Type        │      │ License         │
│ Upload Error  │       │ Detection Error  │      │ Validation Error│
└───────┬───────┘       └────────┬─────────┘      └────────┬────────┘
        │                        │                         │
        ▼                        ▼                         ▼
┌───────────────┐       ┌──────────────────┐      ┌─────────────────┐
│ Log Warning   │       │ Log Warning      │      │ Log Error       │
│ Continue      │       │ Assume needs     │      │ Exit(1)         │
│ (continueOn   │       │ license          │      │ (if org)        │
│  Error=true)  │       │ Continue         │      │                 │
└───────────────┘       └──────────────────┘      └─────────────────┘
```

---

## 11. DATA FLOW: SARIF → GITHUB OUTPUTS

```
┌─────────────────────────────────────────────────────────────────────┐
│                         results.sarif File                           │
│                                                                      │
│  {                                                                   │
│    "runs": [{                                                        │
│      "results": [                                                    │
│        {                                                             │
│          "ruleId": "aws-access-token",                               │
│          "locations": [{                                             │
│            "physicalLocation": {                                     │
│              "artifactLocation": {"uri": "src/file.js"},             │
│              "region": {"startLine": 42}                             │
│            }                                                          │
│          }],                                                          │
│          "partialFingerprints": {                                    │
│            "commitSha": "abc123...",                                 │
│            "author": "John Doe",                                     │
│            "email": "john@example.com"                               │
│          }                                                            │
│        }                                                             │
│      ]                                                               │
│    }]                                                                │
│  }                                                                   │
└────────────────────────────┬─────────────────────────────────────────┘
                             │
                             │ Parse
                             │
                             ▼
        ┌────────────────────────────────────────┐
        │         Extracted Data Points          │
        │                                        │
        │  ruleId = "aws-access-token"           │
        │  commitSha = "abc123..."               │
        │  filePath = "src/file.js"              │
        │  startLine = 42                        │
        │  author = "John Doe"                   │
        │  email = "john@example.com"            │
        └────────────────┬───────────────────────┘
                         │
                         │ Transform
                         │
        ┌────────────────┼────────────────┐
        │                │                │
        ▼                ▼                ▼
┌──────────────┐  ┌─────────────┐  ┌───────────────┐
│ Fingerprint  │  │ PR Comment  │  │ Summary Table │
│              │  │             │  │ Row           │
└──────┬───────┘  └──────┬──────┘  └───────┬───────┘
       │                 │                 │
       ▼                 ▼                 ▼
┌──────────────┐  ┌─────────────┐  ┌───────────────┐
│ abc123...:   │  │ Comment     │  │ | aws-access- │
│ src/file.js: │  │ Body:       │  │   token       │
│ aws-access-  │  │             │  │ | abc123 (🔗) │
│ token:42     │  │ 🛑 Gitleaks │  │ | View (🔗)   │
│              │  │ detected... │  │ | 42          │
│              │  │             │  │ | John Doe    │
│              │  │ Fingerprint:│  │ | 2025-...    │
│              │  │ abc123...:  │  │ | john@...    │
│              │  │ src/...:... │  │ | src/... (🔗)│
│              │  │             │  │               │
│              │  │ ---         │  │               │
│              │  │             │  │               │
│              │  │ Posted to:  │  │               │
│              │  │ - Commit:   │  │               │
│              │  │   abc123    │  │               │
│              │  │ - File:     │  │               │
│              │  │   src/file  │  │               │
│              │  │ - Line: 42  │  │               │
└──────────────┘  └─────────────┘  └───────────────┘
                         │                 │
                         │                 │
                         ▼                 ▼
                  ┌─────────────┐   ┌──────────────┐
                  │ GitHub API  │   │ GitHub       │
                  │ POST        │   │ Actions      │
                  │ /pulls/     │   │ Summary API  │
                  │ comments    │   │              │
                  └─────────────┘   └──────────────┘
```

---

## 12. COMPLETE SYSTEM ARCHITECTURE

```
┌─────────────────────────────────────────────────────────────────────┐
│                       GitHub Actions Runner                          │
│                                                                      │
│  ┌────────────────────────────────────────────────────────────────┐ │
│  │                    Gitleaks Action (Rust)                      │ │
│  │                                                                │ │
│  │  ┌──────────────┐   ┌──────────────┐   ┌──────────────────┐  │ │
│  │  │   Event      │   │ Environment  │   │  Configuration   │  │ │
│  │  │   Handler    │   │   Parser     │   │    Manager       │  │ │
│  │  └──────┬───────┘   └──────┬───────┘   └────────┬─────────┘  │ │
│  │         │                  │                    │            │ │
│  │         └──────────────────┼────────────────────┘            │ │
│  │                            │                                 │ │
│  │                            ▼                                 │ │
│  │                   ┌────────────────┐                         │ │
│  │                   │  Binary        │                         │ │
│  │                   │  Executor      │                         │ │
│  │                   └────────┬───────┘                         │ │
│  │                            │                                 │ │
│  │                            ▼                                 │ │
│  └────────────────────────────┼─────────────────────────────────┘ │
│                               │                                   │
│                               ▼                                   │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                    Gitleaks Binary                         │   │
│  │                                                            │   │
│  │  Inputs:                        Outputs:                  │   │
│  │  - CLI Args                     - Exit Code (0,1,2)       │   │
│  │  - GITLEAKS_CONFIG env          - results.sarif           │   │
│  │  - GITLEAKS_LICENSE env         - stdout/stderr logs      │   │
│  │  - Git Repository               │                         │   │
│  └────────────────────┬───────────────────────────────────────┘   │
│                       │                                           │
│                       ▼                                           │
│  ┌────────────────────────────────────────────────────────────┐   │
│  │                 Gitleaks Action (Rust)                     │   │
│  │                                                            │   │
│  │  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐  │   │
│  │  │    SARIF     │   │    GitHub    │   │   Summary    │  │   │
│  │  │    Parser    │   │     API      │   │  Generator   │  │   │
│  │  │              │   │   Client     │   │              │  │   │
│  │  └──────┬───────┘   └──────┬───────┘   └──────┬───────┘  │   │
│  │         │                  │                  │          │   │
│  │         │                  │                  │          │   │
│  │         └──────────────────┼──────────────────┘          │   │
│  │                            │                             │   │
│  │                            ▼                             │   │
│  │                   ┌────────────────┐                     │   │
│  │                   │  Artifact      │                     │   │
│  │                   │  Uploader      │                     │   │
│  │                   └────────────────┘                     │   │
│  └────────────────────────────────────────────────────────────┘   │
│                               │                                   │
└───────────────────────────────┼───────────────────────────────────┘
                                │
                                ▼
                    ┌───────────────────────┐
                    │    GitHub Platform    │
                    │                       │
                    │  - PR Comments        │
                    │  - Job Summary        │
                    │  - Artifacts          │
                    │  - Workflow Status    │
                    └───────────────────────┘
```

---

## 13. SEQUENCE DIAGRAM: PULL REQUEST SCAN

```
Developer      GitHub        Action        Gitleaks      GitHub API
   │              │            │              │              │
   │─Create PR────>│            │              │              │
   │              │            │              │              │
   │              │─Trigger────>│              │              │
   │              │ PR Event   │              │              │
   │              │            │              │              │
   │              │            │─Fetch Commits─────────────>│
   │              │            │              │              │
   │              │            │<────PR Commits─────────────│
   │              │            │              │              │
   │              │            │─Execute──────>│              │
   │              │            │ detect       │              │
   │              │            │ base^..head  │              │
   │              │            │              │              │
   │              │            │              │─Scan Repo────┐
   │              │            │              │              │
   │              │            │              │<─────────────┘
   │              │            │              │              │
   │              │            │<─Exit Code 2─│              │
   │              │            │ results.sarif│              │
   │              │            │              │              │
   │              │            │─Parse SARIF──┐              │
   │              │            │              │              │
   │              │            │<─────────────┘              │
   │              │            │              │              │
   │              │            │─Fetch Comments──────────────>│
   │              │            │              │              │
   │              │            │<────Existing Comments───────│
   │              │            │              │              │
   │              │            │─Check Duplicates─┐          │
   │              │            │              │   │          │
   │              │            │<─────────────────┘          │
   │              │            │              │              │
   │              │            │─Post Comment────────────────>│
   │              │            │              │              │
   │              │<───────────│──────────────│──Comment─────│
   │─Notification─│            │              │              │
   │  Email       │            │              │              │
   │              │            │              │              │
   │              │            │─Generate Summary─┐          │
   │              │            │              │   │          │
   │              │            │<─────────────────┘          │
   │              │            │              │              │
   │              │            │─Upload Artifact──────────────>│
   │              │            │              │              │
   │              │            │─Fail Workflow>│              │
   │              │<─Workflow Failed          │              │
   │              │            │              │              │
   │<─Status Check Failed─────│              │              │
   │  (PR blocked)            │              │              │
```

---

## LEGEND

### Flow Diagram Symbols

- `┌─┐` : Process/Action Box
- `│ │` : Vertical Connection
- `─` : Horizontal Connection
- `┼` : Intersection
- `▼` : Flow Direction
- `?` : Decision Point
- `─>` : Direction of Flow
- `<─` : Return/Response

### Status Indicators

- `✅` : Success State
- `🛑` : Error/Leak Detected State
- `❌` : Failure State
- `🔗` : Link/URL

### Component Types

- **Handler**: Event processing logic
- **Parser**: Data transformation logic
- **Client**: External API communication
- **Generator**: Output creation logic
- **Executor**: Binary/process execution

---

## IMPLEMENTATION CHECKLIST

Use these flows to implement the Rust port:

### Phase 1: Core Execution
- [ ] Event JSON parsing (Flow 1)
- [ ] Binary execution (Flow 2)
- [ ] Exit code handling (Flow 10)

### Phase 2: SARIF Processing
- [ ] SARIF parsing (Flow 3)
- [ ] Fingerprint generation (Data Flow 11)

### Phase 3: GitHub Integration
- [ ] PR comment creation (Flow 4)
- [ ] Comment deduplication (Flow 4)
- [ ] Summary generation (Flow 7)
- [ ] Artifact upload (Flow 8)

### Phase 4: Event Handling
- [ ] Push event logic (Flow 6)
- [ ] PR event logic (Flow 6)
- [ ] Schedule event logic (Flow 6)

### Phase 5: Configuration
- [ ] Config discovery (Flow 5)
- [ ] Environment variable parsing
- [ ] Feature flag handling (Flow 7, 8)

---

**Document Version**: 1.0
**Last Updated**: 2025-10-15
**Companion to**: GITLEAKS_INTEGRATION_SPEC.md
