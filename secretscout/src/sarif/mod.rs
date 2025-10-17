//! SARIF processing module
//!
//! This module handles parsing SARIF 2.1.0 output from gitleaks and
//! extracting detected secrets with metadata.

pub mod types;

use crate::error::{Result, SarifError};
use std::path::Path;
use types::{DetectedSecret, SarifReport};

/// Parse a SARIF report from a file
pub fn parse_sarif_file(path: impl AsRef<Path>) -> Result<SarifReport> {
    let path = path.as_ref();

    // Check if file exists
    if !path.exists() {
        return Err(SarifError::FileNotFound(path.display().to_string()).into());
    }

    // Read file contents
    let contents = std::fs::read_to_string(path)
        .map_err(|e| SarifError::ParseError(format!("Failed to read file: {}", e)))?;

    parse_sarif_str(&contents)
}

/// Parse a SARIF report from a string
pub fn parse_sarif_str(contents: &str) -> Result<SarifReport> {
    // Parse JSON
    let report: SarifReport = serde_json::from_str(contents)
        .map_err(|e| SarifError::ParseError(format!("Failed to parse JSON: {}", e)))?;

    // Validate structure
    if report.runs.is_empty() {
        return Err(
            SarifError::InvalidStructure("No runs found in SARIF report".to_string()).into(),
        );
    }

    Ok(report)
}

/// Extract detected secrets from a SARIF report
pub fn extract_findings(report: &SarifReport) -> Result<Vec<DetectedSecret>> {
    let mut findings = Vec::new();

    for run in &report.runs {
        for result in &run.results {
            // Skip results without locations
            if result.locations.is_empty() {
                log::warn!("Skipping result without locations: {}", result.rule_id);
                continue;
            }

            // Convert SARIF result to DetectedSecret
            if let Some(secret) = Option::<DetectedSecret>::from(result) {
                findings.push(secret);
            } else {
                log::warn!(
                    "Failed to extract secret from result: {} (missing fingerprints)",
                    result.rule_id
                );
            }
        }
    }

    log::info!("Extracted {} findings from SARIF report", findings.len());

    Ok(findings)
}

/// Parse SARIF file and extract findings in one step
pub fn parse_and_extract(path: impl AsRef<Path>) -> Result<Vec<DetectedSecret>> {
    let report = parse_sarif_file(path)?;
    extract_findings(&report)
}

/// Validate SARIF structure without full parsing
pub fn validate_sarif(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(SarifError::FileNotFound(path.display().to_string()).into());
    }

    // Try to parse - if successful, it's valid
    parse_sarif_file(path)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_sarif() -> String {
        r#"{
            "version": "2.1.0",
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "runs": [
                {
                    "tool": {
                        "driver": {
                            "name": "gitleaks",
                            "version": "8.24.3"
                        }
                    },
                    "results": [
                        {
                            "ruleId": "aws-access-token",
                            "message": {
                                "text": "AWS Access Key detected"
                            },
                            "locations": [
                                {
                                    "physicalLocation": {
                                        "artifactLocation": {
                                            "uri": "src/config.rs"
                                        },
                                        "region": {
                                            "startLine": 42
                                        }
                                    }
                                }
                            ],
                            "partialFingerprints": {
                                "commitSha": "abc123def456",
                                "author": "John Doe",
                                "email": "john@example.com",
                                "date": "2025-10-16T12:00:00Z"
                            }
                        }
                    ]
                }
            ]
        }"#
        .to_string()
    }

    #[test]
    fn test_parse_sarif_str() {
        let sarif = create_test_sarif();
        let report = parse_sarif_str(&sarif).unwrap();

        assert_eq!(report.version, "2.1.0");
        assert_eq!(report.runs.len(), 1);
        assert_eq!(report.runs[0].results.len(), 1);
        assert_eq!(report.runs[0].results[0].rule_id, "aws-access-token");
    }

    #[test]
    fn test_parse_sarif_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(create_test_sarif().as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let report = parse_sarif_file(temp_file.path()).unwrap();
        assert_eq!(report.runs.len(), 1);
    }

    #[test]
    fn test_extract_findings() {
        let sarif = create_test_sarif();
        let report = parse_sarif_str(&sarif).unwrap();
        let findings = extract_findings(&report).unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "aws-access-token");
        assert_eq!(findings[0].file_path, "src/config.rs");
        assert_eq!(findings[0].line_number, 42);
        assert_eq!(findings[0].commit_sha, "abc123def456");
        assert_eq!(findings[0].author, "John Doe");
        assert_eq!(
            findings[0].fingerprint,
            "abc123def456:src/config.rs:aws-access-token:42"
        );
    }

    #[test]
    fn test_parse_and_extract() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(create_test_sarif().as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let findings = parse_and_extract(temp_file.path()).unwrap();
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn test_parse_invalid_sarif() {
        let invalid = r#"{"version": "2.1.0", "runs": []}"#;
        let result = parse_sarif_str(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn test_file_not_found() {
        let result = parse_sarif_file("/nonexistent/path.sarif");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::Error::Sarif(SarifError::FileNotFound(_))
        ));
    }
}
