//! Configuration module for SecretScout
//!
//! This module handles parsing and validation of all configuration from
//! GitHub Actions environment variables and configuration files.

use crate::error::{ConfigError, Result};
use std::env;
use std::path::{Path, PathBuf};

#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};

/// Main configuration structure for SecretScout
#[derive(Debug, Clone)]
#[cfg_attr(feature = "wasm", derive(Serialize, Deserialize))]
pub struct Config {
    /// GitHub token for API access
    pub github_token: String,

    /// Optional gitleaks license key
    pub gitleaks_license: Option<String>,

    /// Gitleaks version to use (default: 8.24.3)
    pub gitleaks_version: String,

    /// Optional path to gitleaks configuration file
    pub gitleaks_config: Option<PathBuf>,

    /// Enable job summary generation (default: true)
    pub enable_summary: bool,

    /// Enable SARIF artifact upload (default: true)
    pub enable_upload_artifact: bool,

    /// Enable PR comments (default: true)
    pub enable_comments: bool,

    /// List of users to notify in PR comments
    pub notify_user_list: Vec<String>,

    /// Optional base ref override
    pub base_ref: Option<String>,

    /// GitHub workspace path
    pub workspace_path: PathBuf,

    /// Path to GitHub event JSON
    pub event_path: PathBuf,

    /// GitHub event name (push, pull_request, etc.)
    pub event_name: String,

    /// Repository in owner/repo format
    pub repository: String,

    /// Repository owner
    pub repository_owner: String,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// This function reads all required and optional environment variables
    /// and validates them according to the specification.
    pub fn from_env() -> Result<Self> {
        // Required environment variables
        let workspace_path = Self::get_required_env("GITHUB_WORKSPACE")?;
        let event_path = Self::get_required_env("GITHUB_EVENT_PATH")?;
        let event_name = Self::get_required_env("GITHUB_EVENT_NAME")?;
        let repository = Self::get_required_env("GITHUB_REPOSITORY")?;
        let repository_owner = Self::get_required_env("GITHUB_REPOSITORY_OWNER")?;

        // GitHub token (required for PR events, optional for others)
        let github_token = env::var("GITHUB_TOKEN").unwrap_or_default();

        // Check if token is required based on event type
        if event_name == "pull_request" && github_token.is_empty() {
            return Err(ConfigError::MissingEnvVar(
                "GITHUB_TOKEN is required for pull_request events".into(),
            )
            .into());
        }

        // Optional gitleaks configuration
        let gitleaks_license = env::var("GITLEAKS_LICENSE").ok();
        let gitleaks_version =
            env::var("GITLEAKS_VERSION").unwrap_or_else(|_| "8.24.3".to_string());

        // Feature toggles with backward-compatible boolean parsing
        let enable_summary = Self::parse_boolean_env("GITLEAKS_ENABLE_SUMMARY", true)?;
        let enable_upload_artifact =
            Self::parse_boolean_env("GITLEAKS_ENABLE_UPLOAD_ARTIFACT", true)?;
        let enable_comments = Self::parse_boolean_env("GITLEAKS_ENABLE_COMMENTS", true)?;

        // User notification list
        let notify_user_list =
            Self::parse_user_list(&env::var("GITLEAKS_NOTIFY_USER_LIST").unwrap_or_default());

        // Base ref override
        let base_ref = env::var("BASE_REF").ok();

        // Auto-detect or use explicit gitleaks config
        let gitleaks_config = if let Ok(explicit_config) = env::var("GITLEAKS_CONFIG") {
            let path = PathBuf::from(&explicit_config);
            Self::validate_path(&path, &PathBuf::from(&workspace_path))?;
            Some(path)
        } else {
            // Auto-detect gitleaks.toml in workspace
            let default_config = PathBuf::from(&workspace_path).join("gitleaks.toml");
            if default_config.exists() {
                Some(default_config)
            } else {
                None
            }
        };

        // Validate paths
        let workspace_path = Self::validate_workspace_path(&workspace_path)?;
        let event_path = Self::validate_path_buf(&event_path, &workspace_path)?;

        // Validate repository format
        if !repository.contains('/') {
            return Err(ConfigError::InvalidRepository(repository).into());
        }

        Ok(Config {
            github_token,
            gitleaks_license,
            gitleaks_version,
            gitleaks_config,
            enable_summary,
            enable_upload_artifact,
            enable_comments,
            notify_user_list,
            base_ref,
            workspace_path,
            event_path,
            event_name,
            repository,
            repository_owner,
        })
    }

    /// Get a required environment variable
    fn get_required_env(key: &str) -> Result<String> {
        env::var(key).map_err(|_| ConfigError::MissingEnvVar(key.to_string()).into())
    }

    /// Parse boolean environment variable with v2 compatibility
    ///
    /// Per FR-8 specification:
    /// - "false" or "0" -> false
    /// - Everything else (including empty) -> true
    fn parse_boolean_env(key: &str, default: bool) -> Result<bool> {
        match env::var(key) {
            Ok(value) => match value.as_str() {
                "false" | "0" => Ok(false),
                _ => Ok(true),
            },
            Err(_) => Ok(default),
        }
    }

    /// Parse comma-separated user list with @ prefixes
    fn parse_user_list(input: &str) -> Vec<String> {
        if input.is_empty() {
            return Vec::new();
        }

        input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Validate workspace path
    fn validate_workspace_path(path: &str) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        if !path_buf.exists() {
            return Err(ConfigError::FileNotFound(path.to_string()).into());
        }

        if !path_buf.is_dir() {
            return Err(ConfigError::InvalidPath(format!("{} is not a directory", path)).into());
        }

        // Canonicalize to get absolute path
        path_buf.canonicalize().map_err(|e| {
            ConfigError::InvalidPath(format!("Failed to canonicalize {}: {}", path, e)).into()
        })
    }

    /// Validate path buffer against workspace
    fn validate_path_buf(path: &str, workspace: &Path) -> Result<PathBuf> {
        let path_buf = PathBuf::from(path);

        // Check for path traversal
        if path.contains("..") {
            return Err(ConfigError::PathTraversal(path.to_string()).into());
        }

        // Canonicalize if exists, otherwise just validate format
        let canonical = if path_buf.exists() {
            path_buf.canonicalize().map_err(|e| {
                ConfigError::InvalidPath(format!("Failed to canonicalize {}: {}", path, e))
            })?
        } else {
            // If file doesn't exist yet (e.g., results.sarif), construct expected path
            if path_buf.is_absolute() {
                path_buf
            } else {
                workspace.join(&path_buf)
            }
        };

        // Ensure path is within workspace
        if !canonical.starts_with(workspace) {
            return Err(ConfigError::OutsideWorkspace(path.to_string()).into());
        }

        Ok(canonical)
    }

    /// Validate a path against workspace
    fn validate_path(path: &Path, workspace: &Path) -> Result<()> {
        // Check for path traversal
        if path.to_string_lossy().contains("..") {
            return Err(ConfigError::PathTraversal(path.display().to_string()).into());
        }

        // Canonicalize and check if within workspace
        if path.exists() {
            let canonical = path.canonicalize().map_err(|e| {
                ConfigError::InvalidPath(format!(
                    "Failed to canonicalize {}: {}",
                    path.display(),
                    e
                ))
            })?;

            if !canonical.starts_with(workspace) {
                return Err(ConfigError::OutsideWorkspace(path.display().to_string()).into());
            }
        }

        Ok(())
    }

    /// Validate a git reference for security
    pub fn validate_git_ref(git_ref: &str) -> Result<()> {
        if git_ref.is_empty() {
            return Ok(()); // Empty refs are allowed for full scans
        }

        // Check for shell metacharacters
        let dangerous_chars = [';', '&', '|', '$', '`', '\n', '\r', '<', '>'];
        for ch in dangerous_chars {
            if git_ref.contains(ch) {
                return Err(ConfigError::InvalidGitRef(format!(
                    "Contains dangerous character '{}'",
                    ch
                ))
                .into());
            }
        }

        // Check for path traversal
        if git_ref.contains("..") {
            return Err(ConfigError::InvalidGitRef("Contains path traversal".to_string()).into());
        }

        Ok(())
    }

    /// Get the SARIF report path (workspace/results.sarif)
    pub fn sarif_path(&self) -> PathBuf {
        self.workspace_path.join("results.sarif")
    }

    /// Get repository owner and name as tuple
    pub fn repo_parts(&self) -> (&str, &str) {
        let parts: Vec<&str> = self.repository.split('/').collect();
        if parts.len() == 2 {
            (parts[0], parts[1])
        } else {
            (self.repository_owner.as_str(), "")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_parse_boolean_env() {
        assert_eq!(
            Config::parse_boolean_env("NONEXISTENT", true).unwrap(),
            true
        );
        assert_eq!(
            Config::parse_boolean_env("NONEXISTENT", false).unwrap(),
            false
        );

        env::set_var("TEST_BOOL_FALSE", "false");
        assert_eq!(
            Config::parse_boolean_env("TEST_BOOL_FALSE", true).unwrap(),
            false
        );

        env::set_var("TEST_BOOL_ZERO", "0");
        assert_eq!(
            Config::parse_boolean_env("TEST_BOOL_ZERO", true).unwrap(),
            false
        );

        env::set_var("TEST_BOOL_TRUE", "true");
        assert_eq!(
            Config::parse_boolean_env("TEST_BOOL_TRUE", false).unwrap(),
            true
        );

        env::set_var("TEST_BOOL_ONE", "1");
        assert_eq!(
            Config::parse_boolean_env("TEST_BOOL_ONE", false).unwrap(),
            true
        );

        env::set_var("TEST_BOOL_ANY", "anything");
        assert_eq!(
            Config::parse_boolean_env("TEST_BOOL_ANY", false).unwrap(),
            true
        );
    }

    #[test]
    fn test_parse_user_list() {
        assert_eq!(Config::parse_user_list(""), Vec::<String>::new());
        assert_eq!(Config::parse_user_list("@user1"), vec!["@user1"]);
        assert_eq!(
            Config::parse_user_list("@user1, @user2, @user3"),
            vec!["@user1", "@user2", "@user3"]
        );
        assert_eq!(
            Config::parse_user_list("@user1,@user2"),
            vec!["@user1", "@user2"]
        );
    }

    #[test]
    fn test_validate_git_ref() {
        assert!(Config::validate_git_ref("").is_ok());
        assert!(Config::validate_git_ref("main").is_ok());
        assert!(Config::validate_git_ref("abc123def456").is_ok());
        assert!(Config::validate_git_ref("refs/heads/feature-branch").is_ok());

        assert!(Config::validate_git_ref("main;echo hello").is_err());
        assert!(Config::validate_git_ref("main&& rm -rf /").is_err());
        assert!(Config::validate_git_ref("../../../etc/passwd").is_err());
        assert!(Config::validate_git_ref("main`whoami`").is_err());
    }

    #[test]
    fn test_repo_parts() {
        let config = Config {
            github_token: String::new(),
            gitleaks_license: None,
            gitleaks_version: "8.24.3".to_string(),
            gitleaks_config: None,
            enable_summary: true,
            enable_upload_artifact: true,
            enable_comments: true,
            notify_user_list: Vec::new(),
            base_ref: None,
            workspace_path: PathBuf::from("/tmp"),
            event_path: PathBuf::from("/tmp/event.json"),
            event_name: "push".to_string(),
            repository: "owner/repo".to_string(),
            repository_owner: "owner".to_string(),
        };

        assert_eq!(config.repo_parts(), ("owner", "repo"));
    }
}
