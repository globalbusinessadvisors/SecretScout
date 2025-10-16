//! WASM bindings for SecretScout
//!
//! This module provides JavaScript-compatible bindings for the WASM target.

use wasm_bindgen::prelude::*;
use crate::{config, error, sarif, outputs};
use serde::{Deserialize, Serialize};

/// Initialize panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}

/// Parse SARIF from JSON string
#[wasm_bindgen]
pub fn parse_sarif(sarif_json: &str) -> Result<JsValue, JsValue> {
    let report = sarif::parse_sarif_str(sarif_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&report)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Extract findings from SARIF JSON string
#[wasm_bindgen]
pub fn extract_findings(sarif_json: &str) -> Result<JsValue, JsValue> {
    let report = sarif::parse_sarif_str(sarif_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let findings = sarif::extract_findings(&report)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    serde_wasm_bindgen::to_value(&findings)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generate success summary
#[wasm_bindgen]
pub fn generate_success_summary() -> String {
    outputs::generate_success_summary()
}

/// Generate error summary
#[wasm_bindgen]
pub fn generate_error_summary(exit_code: i32) -> String {
    outputs::generate_error_summary(exit_code)
}

/// Generate findings summary from findings JSON array
#[wasm_bindgen]
pub fn generate_findings_summary(repository_json: &str, findings_json: &str) -> Result<String, JsValue> {
    #[derive(Deserialize)]
    struct Repo {
        owner: String,
        name: String,
        full_name: String,
        html_url: String,
    }

    let repo: Repo = serde_json::from_str(repository_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    let repository = crate::events::Repository {
        owner: repo.owner,
        name: repo.name,
        full_name: repo.full_name,
        html_url: repo.html_url,
    };

    let findings: Vec<sarif::types::DetectedSecret> = serde_json::from_str(findings_json)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(outputs::generate_findings_summary(&repository, &findings))
}

/// Build comment body
#[wasm_bindgen]
pub fn build_comment_body(
    rule_id: &str,
    commit_sha: &str,
    fingerprint: &str,
    notify_users: JsValue,
) -> Result<String, JsValue> {
    let users: Vec<String> = serde_wasm_bindgen::from_value(notify_users)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(crate::github::build_comment_body(
        rule_id,
        commit_sha,
        fingerprint,
        &users,
    ))
}

/// Check if comment is duplicate
#[wasm_bindgen]
pub fn is_duplicate_comment(
    existing_comments: JsValue,
    new_body: &str,
    new_path: &str,
    new_line: u32,
) -> Result<bool, JsValue> {
    let comments: Vec<serde_json::Value> = serde_wasm_bindgen::from_value(existing_comments)
        .map_err(|e| JsValue::from_str(&e.to_string()))?;

    Ok(crate::github::is_duplicate_comment(
        &comments,
        new_body,
        new_path,
        new_line,
    ))
}

/// Validate git reference
#[wasm_bindgen]
pub fn validate_git_ref(git_ref: &str) -> Result<(), JsValue> {
    config::Config::validate_git_ref(git_ref)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Generate fingerprint
#[wasm_bindgen]
pub fn generate_fingerprint(
    commit_sha: &str,
    file_path: &str,
    rule_id: &str,
    line_number: u32,
) -> String {
    sarif::types::DetectedSecret::generate_fingerprint(commit_sha, file_path, rule_id, line_number)
}

/// Build gitleaks download URL
#[wasm_bindgen]
pub fn build_download_url(version: &str, platform: &str, arch: &str) -> Result<String, JsValue> {
    use crate::binary::{Platform, Architecture};

    let plat = match platform {
        "linux" => Platform::Linux,
        "darwin" => Platform::Darwin,
        "windows" => Platform::Windows,
        _ => return Err(JsValue::from_str(&format!("Unsupported platform: {}", platform))),
    };

    let architecture = match arch {
        "x64" => Architecture::X64,
        "arm64" => Architecture::Arm64,
        "arm" => Architecture::Arm,
        _ => return Err(JsValue::from_str(&format!("Unsupported architecture: {}", arch))),
    };

    Ok(crate::binary::build_download_url(version, plat, architecture))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_success_summary() {
        let summary = generate_success_summary();
        assert!(summary.contains("No leaks detected"));
    }

    #[test]
    fn test_generate_fingerprint() {
        let fp = generate_fingerprint("abc123", "src/main.rs", "test-rule", 42);
        assert_eq!(fp, "abc123:src/main.rs:test-rule:42");
    }

    #[test]
    #[cfg(target_arch = "wasm32")]  // Only run this test on WASM target
    fn test_validate_git_ref() {
        assert!(validate_git_ref("main").is_ok());
        assert!(validate_git_ref("abc123").is_ok());
        assert!(validate_git_ref("main; echo bad").is_err());
    }
}
