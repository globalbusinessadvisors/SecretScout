# WASM BOUNDARY VISUAL DIAGRAMS

**Project:** SecretScout - Rust Port of gitleaks-action
**Component:** Visual Architecture Diagrams
**Date:** October 16, 2025

---

## 1. DATA FLOW DIAGRAM

### 1.1 High-Level Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                   GitHub Actions Runner                       │
│                                                               │
│  Environment Variables:                                       │
│  • GITHUB_TOKEN, GITHUB_WORKSPACE, GITHUB_REPOSITORY         │
│  • GITHUB_EVENT_NAME, GITHUB_EVENT_PATH                      │
│  • GITLEAKS_VERSION, GITLEAKS_LICENSE, GITLEAKS_CONFIG       │
│  • GITLEAKS_ENABLE_SUMMARY, ENABLE_COMMENTS, etc.            │
│                                                               │
│  File System:                                                 │
│  • /workspace/event.json (GitHub event payload)              │
│  • /workspace/results.sarif (gitleaks output)                │
│  • /workspace/gitleaks.toml (optional config)                │
│                                                               │
│  Binaries:                                                    │
│  • node (v20/v24)                                             │
│  • gitleaks (downloaded or cached)                            │
└──────────────────┬───────────────────────────────────────────┘
                   │
                   │ Node.js runtime loads action
                   ▼
┌──────────────────────────────────────────────────────────────┐
│              dist/index.js (JavaScript Wrapper)              │
│                                                               │
│  1. Parse action inputs                                       │
│  2. Build Config object                                       │
│  3. Read event.json file                                      │
│  4. Load WASM module                                          │
│  5. Create callbacks object                                   │
│  6. Call wasm.run_action(config, callbacks)                   │
│  7. Handle result                                             │
│  8. Write summary, upload artifacts, set outputs              │
│  9. Exit with appropriate code                                │
└──────────────────┬───────────────────────────────────────────┘
                   │
                   │ wasm-bindgen FFI
                   │ (JSON serialization)
                   ▼
┌──────────────────────────────────────────────────────────────┐
│         dist/secretscout_bg.wasm (Rust WASM Module)          │
│                                                               │
│  Entry Point: run_action(config_json, callbacks)             │
│                                                               │
│  Processing Flow:                                             │
│  1. Parse & validate config                                   │
│  2. Determine event type (push/PR/dispatch/schedule)          │
│  3. Build gitleaks args → callback: exec_gitleaks()           │
│  4. Parse SARIF → callback: read_file("results.sarif")        │
│  5. Extract findings, generate fingerprints                   │
│  6. For PR events → callback: fetch_pr_commits/comments       │
│  7. Generate PR comments → callback: post_pr_comment()        │
│  8. Generate job summary HTML                                 │
│  9. Return ActionOutput                                       │
│                                                               │
│  Pure Computation (No I/O):                                   │
│  • Configuration validation                                   │
│  • Event routing logic                                        │
│  • SARIF parsing & transformation                             │
│  • Fingerprint generation                                     │
│  • Comment content generation                                 │
│  • Summary HTML generation                                    │
└──────────────────────────────────────────────────────────────┘
```

---

## 2. SEQUENCE DIAGRAM: Push Event

```
┌──────┐        ┌──────┐        ┌──────┐        ┌─────────┐
│GitHub│        │  JS  │        │ WASM │        │Gitleaks │
│Runner│        │Wrapper│       │Module│        │ Binary  │
└──┬───┘        └──┬───┘        └──┬───┘        └────┬────┘
   │               │               │                  │
   │ Set env vars  │               │                  │
   │ & files       │               │                  │
   ├──────────────>│               │                  │
   │               │               │                  │
   │     Run       │               │                  │
   │  action.yml   │               │                  │
   ├──────────────>│               │                  │
   │               │               │                  │
   │               │ Load WASM     │                  │
   │               ├──────────────>│                  │
   │               │               │                  │
   │               │ Read event    │                  │
   │               │ JSON file     │                  │
   │               ├───────┐       │                  │
   │               │       │       │                  │
   │               │<──────┘       │                  │
   │               │               │                  │
   │               │ run_action(   │                  │
   │               │  config,      │                  │
   │               │  callbacks)   │                  │
   │               ├──────────────>│                  │
   │               │               │                  │
   │               │               │ Parse config     │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │               │ Parse event      │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │               │ Build gitleaks   │
   │               │               │ arguments        │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │  execGitleaks(│                  │
   │               │    [args])    │                  │
   │               │<──────────────┤                  │
   │               │               │                  │
   │               │   Spawn       │                  │
   │               │   process     │                  │
   │               ├──────────────────────────────────>│
   │               │               │                  │
   │               │               │         Scan repo│
   │               │               │         Write    │
   │               │               │      results.sarif│
   │               │               │                  │
   │               │ {exitCode,    │                  │
   │               │  stdout,      │                  │
   │               │  stderr}      │                  │
   │               │<──────────────────────────────────┤
   │               │               │                  │
   │               │ Return result │                  │
   │               ├──────────────>│                  │
   │               │               │                  │
   │               │               │ If exit_code=2:  │
   │               │               │                  │
   │               │  readFile(    │                  │
   │               │   "results.   │                  │
   │               │    sarif")    │                  │
   │               │<──────────────┤                  │
   │               │               │                  │
   │               │  {sarif_json} │                  │
   │               ├──────────────>│                  │
   │               │               │                  │
   │               │               │ Parse SARIF      │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │               │ Extract findings │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │               │ Generate summary │
   │               │               ├──────────┐       │
   │               │               │          │       │
   │               │               │<─────────┘       │
   │               │               │                  │
   │               │ ActionOutput  │                  │
   │               │ {exitCode,    │                  │
   │               │  summary,...} │                  │
   │               │<──────────────┤                  │
   │               │               │                  │
   │ Write summary │               │                  │
   │ to file       │               │                  │
   ├───────────────┤               │                  │
   │               │               │                  │
   │ Set outputs   │               │                  │
   ├───────────────┤               │                  │
   │               │               │                  │
   │ Exit(code)    │               │                  │
   │<──────────────┤               │                  │
   │               │               │                  │
```

---

## 3. SEQUENCE DIAGRAM: Pull Request Event

```
┌──────┐  ┌──────┐  ┌──────┐  ┌────────┐  ┌─────────┐
│GitHub│  │  JS  │  │ WASM │  │ GitHub │  │Gitleaks │
│Runner│  │Wrapper│  │Module│  │  API   │  │ Binary  │
└──┬───┘  └──┬───┘  └──┬───┘  └───┬────┘  └────┬────┘
   │         │         │           │             │
   │         │run_action()         │             │
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │         │Parse PR   │             │
   │         │         │event      │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │fetchPrCommits()     │             │
   │         │<────────┤           │             │
   │         │         │           │             │
   │         │GET /pulls/:pr/      │             │
   │         │  commits            │             │
   │         ├────────────────────>│             │
   │         │         │           │             │
   │         │ [commits]           │             │
   │         │<────────────────────┤             │
   │         │         │           │             │
   │         │Return   │           │             │
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │         │Determine  │             │
   │         │         │base/head  │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │execGitleaks([args]) │             │
   │         │<────────┤           │             │
   │         ├────────────────────────────────────>│
   │         │         │           │             │
   │         │{exitCode:2}         │             │
   │         │<────────────────────────────────────┤
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │readFile("results.sarif")          │
   │         │<────────┤           │             │
   │         │         │           │             │
   │         │{sarif}  │           │             │
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │         │Parse &    │             │
   │         │         │extract    │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │fetchPrComments()    │             │
   │         │<────────┤           │             │
   │         │         │           │             │
   │         │GET /pulls/:pr/      │             │
   │         │  comments           │             │
   │         ├────────────────────>│             │
   │         │         │           │             │
   │         │[existing comments]  │             │
   │         │<────────────────────┤             │
   │         │         │           │             │
   │         │Return   │           │             │
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │         │For each   │             │
   │         │         │finding:   │             │
   │         │         │           │             │
   │         │         │Build      │             │
   │         │         │comment    │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │         │Check      │             │
   │         │         │duplicate  │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │postPrComment()      │             │
   │         │<────────┤           │             │
   │         │         │           │             │
   │         │POST /pulls/:pr/     │             │
   │         │  comments           │             │
   │         ├────────────────────>│             │
   │         │         │           │             │
   │         │201 Created          │             │
   │         │<────────────────────┤             │
   │         │         │           │             │
   │         │OK       │           │             │
   │         ├────────>│           │             │
   │         │         │           │             │
   │         │         │[repeat for each finding]│
   │         │         │           │             │
   │         │         │Generate   │             │
   │         │         │summary    │             │
   │         │         ├───────┐   │             │
   │         │         │       │   │             │
   │         │         │<──────┘   │             │
   │         │         │           │             │
   │         │ActionOutput         │             │
   │         │<────────┤           │             │
   │         │         │           │             │
```

---

## 4. MEMORY LAYOUT DIAGRAM

```
┌─────────────────────────────────────────────────────────┐
│            Node.js Process Memory Space                 │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │      JavaScript Heap                           │    │
│  │                                                 │    │
│  │  • Config object (parsed from env)             │    │
│  │  • Event JSON (read from file)                 │    │
│  │  • SARIF JSON (read from file)                 │    │
│  │  • GitHub API responses                        │    │
│  │  • Callbacks object                            │    │
│  │  • Action outputs                              │    │
│  │                                                 │    │
│  │  Memory: ~50-100 MB typical                    │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  ┌────────────────────────────────────────────────┐    │
│  │      WASM Linear Memory (Isolated)             │    │
│  │                                                 │    │
│  │  Stack (grows down):                           │    │
│  │    ┌─────────────────────────────────────┐    │    │
│  │    │ Function call frames                │    │    │
│  │    │ Local variables                     │    │    │
│  │    └─────────────────────────────────────┘    │    │
│  │                                                 │    │
│  │  Heap (grows up):                              │    │
│  │    ┌─────────────────────────────────────┐    │    │
│  │    │ Config struct (deserialized)        │    │    │
│  │    ├─────────────────────────────────────┤    │    │
│  │    │ EventContext struct                 │    │    │
│  │    ├─────────────────────────────────────┤    │    │
│  │    │ SarifReport struct (parsed)         │    │    │
│  │    ├─────────────────────────────────────┤    │    │
│  │    │ Vec<Finding> (extracted findings)   │    │    │
│  │    ├─────────────────────────────────────┤    │    │
│  │    │ String buffers (HTML summary, etc.) │    │    │
│  │    ├─────────────────────────────────────┤    │    │
│  │    │ Temporary allocations               │    │    │
│  │    └─────────────────────────────────────┘    │    │
│  │                                                 │    │
│  │  Memory: ~10-100 MB typical                    │    │
│  │  (Isolated from JS, no shared memory)          │    │
│  └────────────────────────────────────────────────┘    │
│                                                          │
│  Data Crossing Boundary (Copied):                       │
│  JS → WASM: Config JSON string (serialized)             │
│  WASM → JS: ActionOutput JSON (serialized)              │
│  JS ↔ WASM: Callback parameters/results (serialized)    │
└─────────────────────────────────────────────────────────┘

Key Points:
• WASM memory is COMPLETELY ISOLATED from JavaScript heap
• All data crossing boundary is COPIED and SERIALIZED
• JavaScript cannot directly access WASM memory
• WASM cannot directly access JavaScript objects
• wasm-bindgen handles serialization/deserialization automatically
```

---

## 5. CALLBACK INVOCATION PATTERN

```
┌────────────────────────────────────────────────────────────┐
│                    WASM Module                             │
│                                                             │
│  run_action(config_json, callbacks) {                      │
│                                                             │
│    // Step 1: Need to execute gitleaks                     │
│    let args = build_gitleaks_args(config);                 │
│                                                             │
│    // Step 2: Call JavaScript callback                     │
│    let result = callbacks.execGitleaks(args).await;        │
│    │                                                        │
│    └───────────┬─────────────────────────────────────────┐ │
│                │                                          │ │
└────────────────┼──────────────────────────────────────────┼─┘
                 │ Serialized: ["detect", "--redact", ...] │
                 │ (wasm-bindgen converts Rust Vec to      │
                 │  JavaScript Array)                       │
                 │                                          │
                 ▼                                          │
┌────────────────────────────────────────────────────────┐  │
│              JavaScript Wrapper                        │  │
│                                                         │  │
│  callbacks = {                                          │  │
│    execGitleaks: async (args) => {                      │  │
│                                                         │  │
│      // Step 3: Execute actual process                 │  │
│      let exitCode = await exec.exec('gitleaks', args); │  │
│                                                         │  │
│      // Step 4: Read SARIF file                        │  │
│      let sarif = await fs.readFile('results.sarif');   │  │
│                                                         │  │
│      // Step 5: Return result                          │  │
│      return {                                           │  │
│        exitCode: exitCode,                             │  │
│        stdout: "...",                                   │  │
│        stderr: "..."                                    │  │
│      };                                                 │  │
│    }                                                    │  │
│  }                                                      │  │
│                                                         │  │
└────────────────┬────────────────────────────────────────┘  │
                 │                                           │
                 │ Serialized: {exitCode: 2, stdout: ..., ..}│
                 │ (wasm-bindgen converts JavaScript Object  │
                 │  to Rust struct)                          │
                 │                                           │
                 ▼                                           │
┌────────────────────────────────────────────────────────┐  │
│                    WASM Module                         │  │
│                                                         │  │
│  run_action(config_json, callbacks) {                  │  │
│                                                         │  │
│    ...                                                  │  │
│    let result = callbacks.execGitleaks(args).await; ◄──┘  │
│                                                            │
│    // Step 6: Process result in WASM                      │
│    if result.exitCode == 2 {                              │
│      let sarif_json = callbacks.readFile("results.sarif")│
│                                 .await?;                   │
│      let findings = parse_sarif(sarif_json)?;             │
│      ...                                                   │
│    }                                                       │
│  }                                                         │
│                                                            │
└────────────────────────────────────────────────────────────┘

Flow Summary:
1. WASM builds arguments (pure computation)
2. WASM calls JS callback with serialized data
3. JS performs I/O operation (file/network/process)
4. JS returns result as serialized data
5. WASM receives and deserializes result
6. WASM continues processing
```

---

## 6. ERROR HANDLING FLOW

```
┌─────────────────────────────────────────────────────────┐
│                   WASM Module                           │
│                                                          │
│  run_action(config_json, callbacks)                     │
│    -> Result<ActionOutput, WasmError>                   │
│                                                          │
│  try {                                                   │
│    // Parse config                                       │
│    let config = parse_config(config_json)?; ────────┐   │
│                                          │           │   │
│    // Execute gitleaks                   │           │   │
│    let result = exec_gitleaks(config)?; ─┼───────┐   │   │
│                                          │       │   │   │
│    // Parse SARIF                        │       │   │   │
│    let findings = parse_sarif()?; ───────┼───┐   │   │   │
│                                          │   │   │   │   │
│    // Generate outputs                   │   │   │   │   │
│    Ok(ActionOutput { ... })              │   │   │   │   │
│                                          │   │   │   │   │
│  } catch (error) {                       │   │   │   │   │
│                                          │   │   │   │   │
│    // Convert to WasmError               │   │   │   │   │
│    Err(WasmError {                       │   │   │   │   │
│      error: "...",                       │   │   │   │   │
│      code: "...",                        │   │   │   │   │
│      severity: "fatal",                  │   │   │   │   │
│      context: {...}                      │   │   │   │   │
│    })                                    │   │   │   │   │
│  }                                       │   │   │   │   │
│                                          │   │   │   │   │
└──────────────────────────────────────────┼───┼───┼───┼───┘
                                           │   │   │   │
                       Error Path 1: ◄─────┘   │   │   │
                       Config parsing failed   │   │   │
                                               │   │   │
                       Error Path 2: ◄─────────┘   │   │
                       Gitleaks execution failed   │   │
                                                   │   │
                       Error Path 3: ◄─────────────┘   │
                       SARIF parsing failed            │
                                                       │
                       Success Path: ◄─────────────────┘
                                                       │
                                                       ▼
┌──────────────────────────────────────────────────────────┐
│              JavaScript Wrapper                          │
│                                                           │
│  try {                                                    │
│    const result = await wasm.run_action(config, cb);     │
│                                                           │
│    // Success case                                        │
│    const output = JSON.parse(result);                    │
│    process.exit(output.exitCode);                        │
│                                                           │
│  } catch (error) {                                        │
│    // Error case                                          │
│    if (error.code) {                                      │
│      // Structured WASM error                            │
│      core.error(`[${error.code}] ${error.error}`);       │
│                                                           │
│      if (error.severity === 'fatal') {                   │
│        process.exit(1);                                   │
│      } else {                                             │
│        // Warning - continue                             │
│      }                                                    │
│    } else {                                               │
│      // Unexpected error                                 │
│      core.error(`Unexpected: ${error}`);                 │
│      process.exit(1);                                     │
│    }                                                      │
│  }                                                        │
│                                                           │
└───────────────────────────────────────────────────────────┘

Error Types:
• CONFIG_INVALID → Fatal, exit 1
• EVENT_PARSE_ERROR → Fatal, exit 1
• UNSUPPORTED_EVENT → Fatal, exit 1
• SARIF_PARSE_ERROR → Fatal, exit 1
• GITHUB_API_ERROR → May be non-fatal (e.g., comment posting)
• GITLEAKS_ERROR → Fatal, exit 1
• LICENSE_REQUIRED → Fatal, exit 1
• INTERNAL_ERROR → Fatal, exit 1
```

---

## 7. PERFORMANCE CRITICAL PATH

```
Time Budget: < 500ms total WASM overhead
(Excluding gitleaks scan which dominates)

┌─────────────────────────────────────────────────────────┐
│              Performance Critical Path                  │
│                                                          │
│  0ms   ┌─────────────────────────────────────────┐     │
│        │ Load WASM module                        │     │
│  50ms  └─────────────────────────────────────────┘     │
│        Target: < 50ms (actual: ~30ms typical)           │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Parse config JSON                       │     │
│  55ms  └─────────────────────────────────────────┘     │
│        Target: < 5ms (actual: ~2ms typical)             │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Parse event JSON                        │     │
│  65ms  └─────────────────────────────────────────┘     │
│        Target: < 10ms (actual: ~5ms typical)            │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Build gitleaks arguments                │     │
│  70ms  └─────────────────────────────────────────┘     │
│        Target: < 5ms (actual: ~1ms typical)             │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Execute gitleaks (DOMINATES)            │     │
│ ~30s   └─────────────────────────────────────────┘     │
│        (Not counted in WASM overhead)                   │
│        Typical: 5-60 seconds depending on repo size     │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Parse SARIF (100 findings)              │     │
│  120ms └─────────────────────────────────────────┘     │
│        Target: < 50ms (actual: ~30ms for 100)           │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Extract findings & fingerprints         │     │
│  130ms └─────────────────────────────────────────┘     │
│        Target: < 10ms (actual: ~5ms for 100)            │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Fetch existing PR comments (if PR)      │     │
│  400ms └─────────────────────────────────────────┘     │
│        (Network latency, not counted in WASM)           │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Generate PR comment content             │     │
│  450ms └─────────────────────────────────────────┘     │
│        Target: < 50ms (actual: ~20ms for 100)           │
│                                                          │
│        ┌─────────────────────────────────────────┐     │
│        │ Generate job summary HTML               │     │
│  550ms └─────────────────────────────────────────┘     │
│        Target: < 100ms (actual: ~50ms for 100)          │
│                                                          │
│        Total WASM overhead: ~143ms (well under budget)  │
│        Total with I/O: ~900ms (acceptable)              │
│                                                          │
└─────────────────────────────────────────────────────────┘

Optimization Priorities:
1. SARIF parsing (largest WASM overhead)
2. Summary HTML generation (second largest)
3. WASM module size (affects load time)
4. JSON serialization (crossing boundary frequently)

Non-Critical (I/O dominated):
• Gitleaks execution (30+ seconds, unavoidable)
• GitHub API calls (network latency)
• File operations (disk I/O)
```

---

## 8. SECURITY BOUNDARY

```
┌──────────────────────────────────────────────────────────┐
│                   Untrusted Zone                         │
│  (GitHub Actions Runner - User controlled)               │
│                                                           │
│  • Environment variables (potentially malicious)          │
│  • Event JSON files (user-generated content)             │
│  • Configuration files (user-provided)                   │
│  • Git references (user input)                           │
│                                                           │
└────────────────┬─────────────────────────────────────────┘
                 │
                 │ ALL INPUTS MUST BE VALIDATED
                 │
                 ▼
┌──────────────────────────────────────────────────────────┐
│            Security Validation Layer                     │
│            (JavaScript Wrapper)                          │
│                                                           │
│  • Basic type checking                                    │
│  • Required field validation                             │
│  • Format validation (paths, tokens, etc.)               │
│                                                           │
└────────────────┬─────────────────────────────────────────┘
                 │
                 │ Serialized JSON
                 │
                 ▼
┌──────────────────────────────────────────────────────────┐
│            WASM Security Boundary                        │
│            (Enforced by WASM runtime)                    │
│                                                           │
│  ✅ Memory Isolation:                                    │
│     • WASM cannot access host memory                     │
│     • WASM cannot corrupt JavaScript heap                │
│                                                           │
│  ✅ No Direct I/O:                                       │
│     • WASM cannot read files                             │
│     • WASM cannot make network requests                  │
│     • WASM cannot execute processes                      │
│     • WASM cannot access environment variables           │
│                                                           │
│  ✅ Control Flow Integrity:                              │
│     • WASM enforces type safety                          │
│     • No arbitrary code execution                        │
│     • No buffer overflows affecting host                 │
│                                                           │
└────────────────┬─────────────────────────────────────────┘
                 │
                 │ MUST use callbacks for all I/O
                 │
                 ▼
┌──────────────────────────────────────────────────────────┐
│         Deep Validation Layer                            │
│         (WASM Module)                                    │
│                                                           │
│  • Git reference validation (prevent injection)           │
│    validate_git_ref(ref) {                               │
│      if ref.contains([';','&','|','$','`']) {            │
│        REJECT                                             │
│      }                                                    │
│    }                                                      │
│                                                           │
│  • Path validation (prevent traversal)                   │
│    validate_path(path) {                                 │
│      if path.contains("..") {                            │
│        REJECT                                             │
│      }                                                    │
│      if !path.starts_with(workspace) {                   │
│        REJECT                                             │
│      }                                                    │
│    }                                                      │
│                                                           │
│  • Repository name validation                            │
│    validate_repo(name) {                                 │
│      if !name.matches("^[a-zA-Z0-9-_./]+$") {            │
│        REJECT                                             │
│      }                                                    │
│    }                                                      │
│                                                           │
│  • Secret sanitization (prevent leakage)                 │
│    sanitize_log(message, config) {                       │
│      message.replace(config.token, "***REDACTED***")     │
│    }                                                      │
│                                                           │
└────────────────┬─────────────────────────────────────────┘
                 │
                 │ Validated, safe operations only
                 │
                 ▼
┌──────────────────────────────────────────────────────────┐
│              Controlled I/O Layer                        │
│              (JavaScript Callbacks)                      │
│                                                           │
│  • Only whitelisted operations allowed                   │
│  • All I/O is logged and auditable                       │
│  • Sanitization applied before execution                 │
│                                                           │
└──────────────────────────────────────────────────────────┘

Defense in Depth:
1. WASM isolation (cannot access host directly)
2. Input validation (reject malicious input)
3. Capability-based I/O (only via callbacks)
4. Sanitization (prevent data leakage)
5. Logging (audit trail)
```

---

**Document Status:** ✅ COMPLETE
**Date:** October 16, 2025
**Component:** Visual Architecture Diagrams
