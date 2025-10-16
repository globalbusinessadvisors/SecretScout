//! SARIF 2.1.0 type definitions
//!
//! Complete type-safe representations of SARIF structures with serde support.

use serde::{Deserialize, Serialize};

/// Root SARIF document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SarifReport {
    #[serde(rename = "$schema")]
    pub schema: Option<String>,
    pub version: String,
    pub runs: Vec<Run>,
}

/// A single run of an analysis tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub tool: Tool,
    #[serde(default)]
    pub results: Vec<Result>,
}

/// Information about the analysis tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    pub driver: Driver,
}

/// Driver information (the analysis tool itself)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Driver {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub information_uri: Option<String>,
}

/// A single result from the analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Result {
    pub rule_id: String,
    pub message: Message,
    #[serde(default)]
    pub locations: Vec<Location>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_fingerprints: Option<PartialFingerprints>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

/// Message associated with a result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub text: String,
}

/// Location where a result was found
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub physical_location: PhysicalLocation,
}

/// Physical location in source code
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PhysicalLocation {
    pub artifact_location: ArtifactLocation,
    pub region: Region,
}

/// Location of an artifact (file)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactLocation {
    pub uri: String,
}

/// Region within an artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Region {
    pub start_line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<ArtifactContent>,
}

/// Artifact content (code snippet)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactContent {
    pub text: String,
}

/// Partial fingerprints for result identification
///
/// Gitleaks includes commit metadata here
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialFingerprints {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// Domain model for a detected secret
///
/// This is our internal representation extracted from SARIF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedSecret {
    pub rule_id: String,
    pub file_path: String,
    pub line_number: u32,
    pub commit_sha: String,
    pub author: String,
    pub email: String,
    pub date: String,
    pub fingerprint: String,
}

impl DetectedSecret {
    /// Generate fingerprint for .gitleaksignore
    ///
    /// Format: {commit}:{file}:{rule}:{line}
    pub fn generate_fingerprint(
        commit_sha: &str,
        file_path: &str,
        rule_id: &str,
        line_number: u32,
    ) -> String {
        format!("{}:{}:{}:{}", commit_sha, file_path, rule_id, line_number)
    }

    /// Get short commit SHA (first 7 characters)
    pub fn short_sha(&self) -> &str {
        if self.commit_sha.len() >= 7 {
            &self.commit_sha[..7]
        } else {
            &self.commit_sha
        }
    }

    /// Create URL to commit
    pub fn commit_url(&self, repo_url: &str) -> String {
        format!("{}/commit/{}", repo_url, self.commit_sha)
    }

    /// Create URL to secret location
    pub fn secret_url(&self, repo_url: &str) -> String {
        format!(
            "{}/blob/{}/{}#L{}",
            repo_url, self.commit_sha, self.file_path, self.line_number
        )
    }

    /// Create URL to file
    pub fn file_url(&self, repo_url: &str) -> String {
        format!("{}/blob/{}/{}", repo_url, self.commit_sha, self.file_path)
    }
}

impl From<&Result> for Option<DetectedSecret> {
    fn from(result: &Result) -> Self {
        // Extract first location (required)
        let location = result.locations.first()?;
        let file_path = location.physical_location.artifact_location.uri.clone();
        let line_number = location.physical_location.region.start_line;

        // Extract partial fingerprints (commit metadata)
        let fingerprints = result.partial_fingerprints.as_ref()?;
        let commit_sha = fingerprints.commit_sha.as_deref().unwrap_or("unknown").to_string();
        let author = fingerprints.author.as_deref().unwrap_or("unknown").to_string();
        let email = fingerprints.email.as_deref().unwrap_or("unknown").to_string();
        let date = fingerprints.date.as_deref().unwrap_or("unknown").to_string();

        // Generate fingerprint
        let fingerprint = DetectedSecret::generate_fingerprint(
            &commit_sha,
            &file_path,
            &result.rule_id,
            line_number,
        );

        Some(DetectedSecret {
            rule_id: result.rule_id.clone(),
            file_path,
            line_number,
            commit_sha,
            author,
            email,
            date,
            fingerprint,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_fingerprint() {
        let fp = DetectedSecret::generate_fingerprint(
            "abc123",
            "src/main.rs",
            "aws-access-token",
            42,
        );
        assert_eq!(fp, "abc123:src/main.rs:aws-access-token:42");
    }

    #[test]
    fn test_short_sha() {
        let secret = DetectedSecret {
            rule_id: "test".to_string(),
            file_path: "test.rs".to_string(),
            line_number: 1,
            commit_sha: "abcdef1234567890".to_string(),
            author: "test".to_string(),
            email: "test@example.com".to_string(),
            date: "2025-10-16".to_string(),
            fingerprint: "test".to_string(),
        };

        assert_eq!(secret.short_sha(), "abcdef1");
    }

    #[test]
    fn test_urls() {
        let secret = DetectedSecret {
            rule_id: "test".to_string(),
            file_path: "src/main.rs".to_string(),
            line_number: 42,
            commit_sha: "abc123".to_string(),
            author: "test".to_string(),
            email: "test@example.com".to_string(),
            date: "2025-10-16".to_string(),
            fingerprint: "test".to_string(),
        };

        let repo_url = "https://github.com/owner/repo";

        assert_eq!(
            secret.commit_url(repo_url),
            "https://github.com/owner/repo/commit/abc123"
        );
        assert_eq!(
            secret.secret_url(repo_url),
            "https://github.com/owner/repo/blob/abc123/src/main.rs#L42"
        );
        assert_eq!(
            secret.file_url(repo_url),
            "https://github.com/owner/repo/blob/abc123/src/main.rs"
        );
    }
}
