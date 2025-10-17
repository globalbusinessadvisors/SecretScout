//! Error types for SecretScout
//!
//! This module defines a comprehensive error hierarchy using thiserror,
//! with WASM-compatible serialization and proper severity levels.

#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

/// Error severity levels for proper error handling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum ErrorSeverity {
    /// Fatal errors that should cause immediate exit with code 1
    Fatal,
    /// Non-fatal errors that should be logged as warnings and allow continuation
    NonFatal,
    /// Expected errors that represent normal flow control (e.g., no commits to scan)
    Expected,
}

/// Root error type for the entire application
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Event processing error: {0}")]
    Event(#[from] EventError),

    #[error("Binary management error: {0}")]
    Binary(#[from] BinaryError),

    #[error("SARIF processing error: {0}")]
    Sarif(#[from] SarifError),

    #[error("GitHub API error: {0}")]
    GitHub(#[from] GitHubError),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("JSON error: {0}")]
    Json(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("License validation error: {0}")]
    License(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Configuration-related errors
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid environment variable value for {key}: {value}")]
    InvalidEnvVar { key: String, value: String },

    #[error("Invalid boolean value: {0} (expected 'true', 'false', '0', or '1')")]
    InvalidBoolean(String),

    #[error("Invalid git reference: {0}")]
    InvalidGitRef(String),

    #[error("Invalid file path: {0}")]
    InvalidPath(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Path outside workspace: {0}")]
    OutsideWorkspace(String),

    #[error("Invalid repository format: {0} (expected 'owner/repo')")]
    InvalidRepository(String),
}

/// Event processing errors
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum EventError {
    #[error("Unsupported event type: {0}")]
    UnsupportedEvent(String),

    #[error("Invalid event JSON: {0}")]
    InvalidEventJson(String),

    #[error("Missing required field in event: {0}")]
    MissingField(String),

    #[error("No commits found in event")]
    NoCommits,

    #[error("Failed to fetch PR commits: {0}")]
    FetchPRCommits(String),

    #[error("Invalid PR number: {0}")]
    InvalidPRNumber(i64),
}

/// Binary management errors
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum BinaryError {
    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Unsupported architecture: {0}")]
    UnsupportedArchitecture(String),

    #[error("Failed to download binary: {0}")]
    DownloadFailed(String),

    #[error("Failed to extract archive: {0}")]
    ExtractionFailed(String),

    #[error("Binary not found in archive")]
    BinaryNotFound,

    #[error("Failed to execute gitleaks: {0}")]
    ExecutionFailed(String),

    #[error("Gitleaks exited with error code {code}: {stderr}")]
    GitleaksError { code: i32, stderr: String },

    #[error("Failed to make binary executable: {0}")]
    ChmodFailed(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Failed to resolve latest version: {0}")]
    VersionResolution(String),
}

/// SARIF processing errors
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum SarifError {
    #[error("SARIF file not found: {0}")]
    FileNotFound(String),

    #[error("Failed to parse SARIF JSON: {0}")]
    ParseError(String),

    #[error("Invalid SARIF structure: {0}")]
    InvalidStructure(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("No results found in SARIF report")]
    NoResults,
}

/// GitHub API errors
#[derive(thiserror::Error, Debug)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub enum GitHubError {
    #[error("API request failed with status {status}: {message}")]
    RequestFailed { status: u16, message: String },

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Failed to parse API response: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Cannot comment on line (diff too large or file not in PR)")]
    DiffTooLarge,

    #[error("Max retries exceeded")]
    MaxRetriesExceeded,
}

impl Error {
    /// Returns the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            // Fatal errors
            Error::Config(ConfigError::MissingEnvVar(_)) => ErrorSeverity::Fatal,
            Error::Config(ConfigError::InvalidEnvVar { .. }) => ErrorSeverity::Fatal,
            Error::Config(ConfigError::PathTraversal(_)) => ErrorSeverity::Fatal,
            Error::Config(ConfigError::OutsideWorkspace(_)) => ErrorSeverity::Fatal,
            Error::Event(EventError::UnsupportedEvent(_)) => ErrorSeverity::Fatal,
            Error::Event(EventError::NoCommits) => ErrorSeverity::Expected,
            Error::Binary(BinaryError::UnsupportedPlatform(_)) => ErrorSeverity::Fatal,
            Error::Binary(BinaryError::UnsupportedArchitecture(_)) => ErrorSeverity::Fatal,
            Error::Binary(BinaryError::DownloadFailed(_)) => ErrorSeverity::Fatal,
            Error::Binary(BinaryError::GitleaksError { .. }) => ErrorSeverity::Fatal,
            Error::Sarif(SarifError::FileNotFound(_)) => ErrorSeverity::Fatal,
            Error::Sarif(SarifError::ParseError(_)) => ErrorSeverity::Fatal,
            Error::GitHub(GitHubError::AuthenticationFailed(_)) => ErrorSeverity::Fatal,
            Error::License(_) => ErrorSeverity::Fatal,

            // Non-fatal errors (warnings)
            Error::Binary(BinaryError::CacheError(_)) => ErrorSeverity::NonFatal,
            Error::Binary(BinaryError::VersionResolution(_)) => ErrorSeverity::NonFatal,
            Error::GitHub(GitHubError::DiffTooLarge) => ErrorSeverity::NonFatal,
            Error::GitHub(GitHubError::NotFound(_)) => ErrorSeverity::NonFatal,
            Error::GitHub(GitHubError::RateLimitExceeded) => ErrorSeverity::NonFatal,

            // Default to fatal for safety
            _ => ErrorSeverity::Fatal,
        }
    }

    /// Masks sensitive information in error messages
    pub fn sanitized(&self) -> String {
        let message = self.to_string();
        // Replace any potential tokens or secrets with ***
        message
            .replace(
                |c: char| c.is_ascii_alphanumeric() && message.contains("token"),
                "***",
            )
            .replace(
                |c: char| c.is_ascii_alphanumeric() && message.contains("key"),
                "***",
            )
    }
}

impl ConfigError {
    /// Create a missing environment variable error
    pub fn missing_env(key: impl Into<String>) -> Self {
        ConfigError::MissingEnvVar(key.into())
    }

    /// Create an invalid environment variable error
    pub fn invalid_env(key: impl Into<String>, value: impl Into<String>) -> Self {
        ConfigError::InvalidEnvVar {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl EventError {
    /// Create an unsupported event error
    pub fn unsupported(event: impl Into<String>) -> Self {
        EventError::UnsupportedEvent(event.into())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err.to_string())
    }
}

#[cfg(feature = "native")]
impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err.to_string())
    }
}

#[cfg(feature = "native")]
impl From<octocrab::Error> for Error {
    fn from(err: octocrab::Error) -> Self {
        Error::GitHub(GitHubError::NetworkError(err.to_string()))
    }
}

/// Result type alias for SecretScout operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity() {
        let err = Error::Config(ConfigError::MissingEnvVar("TEST".into()));
        assert_eq!(err.severity(), ErrorSeverity::Fatal);

        let err = Error::Event(EventError::NoCommits);
        assert_eq!(err.severity(), ErrorSeverity::Expected);

        let err = Error::Binary(BinaryError::CacheError("test".into()));
        assert_eq!(err.severity(), ErrorSeverity::NonFatal);
    }

    #[test]
    fn test_config_error_constructors() {
        let err = ConfigError::missing_env("TEST_VAR");
        assert!(matches!(err, ConfigError::MissingEnvVar(_)));

        let err = ConfigError::invalid_env("KEY", "value");
        assert!(matches!(err, ConfigError::InvalidEnvVar { .. }));
    }

    #[test]
    fn test_error_display() {
        let err = Error::Config(ConfigError::MissingEnvVar("GITHUB_TOKEN".into()));
        let msg = err.to_string();
        assert!(msg.contains("GITHUB_TOKEN"));
        assert!(msg.contains("Configuration error"));
    }
}
