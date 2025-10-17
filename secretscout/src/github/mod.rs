//! GitHub API client module
//!
//! This module provides GitHub API integration using octocrab with retry logic,
//! rate limit handling, and all necessary API operations.

use crate::config::Config;
use crate::error::{GitHubError, Result};
use crate::events::{Author, Commit, Repository};

#[cfg(feature = "native")]
use octocrab::Octocrab;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// PR review comment for posting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PRComment {
    pub body: String,
    pub commit_id: String,
    pub path: String,
    pub line: u32,
    pub side: String, // "RIGHT" or "LEFT"
}

/// Account type (User vs Organization)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountType {
    User,
    Organization,
}

/// Account information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub account_type: AccountType,
    pub login: String,
}

/// Create an octocrab instance with authentication
#[cfg(feature = "native")]
fn create_client(token: &str) -> Result<Octocrab> {
    Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .map_err(|e| GitHubError::NetworkError(e.to_string()).into())
}

/// Fetch PR commits with retry
#[cfg(feature = "native")]
pub async fn fetch_pr_commits(
    config: &Config,
    repository: &Repository,
    pr_number: i64,
) -> Result<Vec<Commit>> {
    log::info!("Fetching commits for PR #{}", pr_number);

    let octocrab = create_client(&config.github_token)?;

    // Use octocrab HTTP client to fetch commits directly
    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/commits",
        repository.owner, repository.name, pr_number
    );

    let response = retry_with_backoff(|| async {
        octocrab
            .get::<Vec<serde_json::Value>, _, ()>(&url, None::<&()>)
            .await
            .map_err(|e| GitHubError::RequestFailed {
                status: 0,
                message: e.to_string(),
            })
    })
    .await?;

    let result: Vec<Commit> = response
        .into_iter()
        .filter_map(|c| {
            Some(Commit {
                sha: c["sha"].as_str()?.to_string(),
                author: Author {
                    name: c["commit"]["author"]["name"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                    email: c["commit"]["author"]["email"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string(),
                },
                message: c["commit"]["message"].as_str()?.to_string(),
            })
        })
        .collect();

    log::info!("Fetched {} commits", result.len());

    Ok(result)
}

/// Fetch existing PR review comments
#[cfg(feature = "native")]
pub async fn fetch_pr_comments(
    config: &Config,
    repository: &Repository,
    pr_number: i64,
) -> Result<Vec<serde_json::Value>> {
    log::debug!("Fetching existing PR comments for deduplication");

    let octocrab = create_client(&config.github_token)?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/comments",
        repository.owner, repository.name, pr_number
    );

    let result: Vec<serde_json::Value> = retry_with_backoff(|| async {
        octocrab
            .get::<Vec<serde_json::Value>, _, ()>(&url, None::<&()>)
            .await
            .map_err(|e| GitHubError::RequestFailed {
                status: 0,
                message: e.to_string(),
            })
    })
    .await?;

    log::debug!("Fetched {} existing comments", result.len());

    Ok(result)
}

/// Post a PR review comment
#[cfg(feature = "native")]
pub async fn post_pr_comment(
    config: &Config,
    repository: &Repository,
    pr_number: i64,
    comment: &PRComment,
) -> Result<()> {
    log::debug!("Posting comment on {}:{}", comment.path, comment.line);

    let octocrab = create_client(&config.github_token)?;

    let url = format!(
        "https://api.github.com/repos/{}/{}/pulls/{}/comments",
        repository.owner, repository.name, pr_number
    );

    let body = serde_json::json!({
        "body": comment.body,
        "commit_id": comment.commit_id,
        "path": comment.path,
        "line": comment.line,
        "side": comment.side,
    });

    retry_with_backoff::<_, _, (), _>(|| async {
        octocrab
            .post::<serde_json::Value, _>(&url, Some(&body))
            .await
            .map_err(|e| {
                // Check for specific error types
                let error_msg = e.to_string();
                if error_msg.contains("422") || error_msg.contains("Unprocessable") {
                    GitHubError::DiffTooLarge
                } else if error_msg.contains("401") || error_msg.contains("403") {
                    GitHubError::AuthenticationFailed(error_msg)
                } else if error_msg.contains("404") {
                    GitHubError::NotFound(error_msg)
                } else {
                    GitHubError::RequestFailed {
                        status: 0,
                        message: error_msg,
                    }
                }
            })
    })
    .await?;

    Ok(())
}

/// Fetch account information to determine type
#[cfg(feature = "native")]
pub async fn fetch_account_info(config: &Config, username: &str) -> Result<AccountInfo> {
    log::debug!("Fetching account info for: {}", username);

    let octocrab = create_client(&config.github_token)?;

    let url = format!("https://api.github.com/users/{}", username);

    let user: serde_json::Value = retry_with_backoff(|| async {
        octocrab
            .get::<serde_json::Value, _, ()>(&url, None::<&()>)
            .await
            .map_err(|e| GitHubError::RequestFailed {
                status: 0,
                message: e.to_string(),
            })
    })
    .await?;

    let account_type = match user["type"].as_str() {
        Some("Organization") => AccountType::Organization,
        _ => AccountType::User,
    };

    let login = user["login"].as_str().unwrap_or(username).to_string();

    Ok(AccountInfo {
        account_type,
        login,
    })
}

/// Retry with exponential backoff
#[cfg(feature = "native")]
async fn retry_with_backoff<F, Fut, T, E>(mut f: F) -> std::result::Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    let max_retries = 3;
    let base_delay = Duration::from_secs(1);

    for attempt in 0..max_retries {
        match f().await {
            Ok(value) => return Ok(value),
            Err(e) => {
                if attempt < max_retries - 1 {
                    let delay = base_delay * 2_u32.pow(attempt);
                    log::warn!(
                        "Request failed (attempt {}/{}): {}. Retrying in {:?}...",
                        attempt + 1,
                        max_retries,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                } else {
                    log::error!("Request failed after {} attempts: {}", max_retries, e);
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

/// Check if comment is duplicate
pub fn is_duplicate_comment(
    existing_comments: &[serde_json::Value],
    new_body: &str,
    new_path: &str,
    new_line: u32,
) -> bool {
    for comment in existing_comments {
        let body = comment["body"].as_str().unwrap_or("");
        let path = comment["path"].as_str().unwrap_or("");
        let line = comment["line"].as_u64().unwrap_or(0) as u32;

        if body == new_body && path == new_path && line == new_line {
            return true;
        }
    }

    false
}

/// Build comment body for a detected secret
pub fn build_comment_body(
    rule_id: &str,
    commit_sha: &str,
    fingerprint: &str,
    notify_users: &[String],
) -> String {
    let mut body = format!(
        "🛑 **Gitleaks Secret Detected**\n\n\
         **Rule:** `{}`\n\
         **Commit:** `{}`\n\
         **Fingerprint:** `{}`\n\n\
         To ignore this finding, add the fingerprint to `.gitleaksignore`.\n",
        rule_id, commit_sha, fingerprint
    );

    if !notify_users.is_empty() {
        body.push_str(&format!("\n**CC:** {}\n", notify_users.join(" ")));
    }

    body
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_comment_body() {
        let body = build_comment_body(
            "aws-access-token",
            "abc123",
            "abc123:src/main.rs:aws-access-token:42",
            &[],
        );

        assert!(body.contains("aws-access-token"));
        assert!(body.contains("abc123"));
        assert!(body.contains("abc123:src/main.rs:aws-access-token:42"));
        assert!(body.contains(".gitleaksignore"));
    }

    #[test]
    fn test_build_comment_body_with_mentions() {
        let body = build_comment_body(
            "generic-api-key",
            "def456",
            "def456:config.yml:generic-api-key:10",
            &["@user1".to_string(), "@user2".to_string()],
        );

        assert!(body.contains("@user1"));
        assert!(body.contains("@user2"));
        assert!(body.contains("CC:"));
    }

    #[test]
    fn test_is_duplicate_comment() {
        let existing = vec![serde_json::json!({
            "body": "test body",
            "path": "src/main.rs",
            "line": 42
        })];

        assert!(is_duplicate_comment(
            &existing,
            "test body",
            "src/main.rs",
            42
        ));
        assert!(!is_duplicate_comment(
            &existing,
            "different body",
            "src/main.rs",
            42
        ));
        assert!(!is_duplicate_comment(
            &existing,
            "test body",
            "src/other.rs",
            42
        ));
        assert!(!is_duplicate_comment(
            &existing,
            "test body",
            "src/main.rs",
            43
        ));
    }
}
