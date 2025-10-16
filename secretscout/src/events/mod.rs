//! Event routing and processing module
//!
//! This module handles GitHub event parsing and routing for all supported
//! event types: push, pull_request, workflow_dispatch, and schedule.

use crate::config::Config;
use crate::error::{EventError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported GitHub event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    Push,
    PullRequest,
    WorkflowDispatch,
    Schedule,
}

/// Complete event context with all necessary information for scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventContext {
    pub event_type: EventType,
    pub repository: Repository,
    pub base_ref: String,
    pub head_ref: String,
    pub commits: Vec<Commit>,
    pub pull_request: Option<PullRequest>,
}

/// Repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub html_url: String,
}

/// Commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub sha: String,
    pub author: Author,
    pub message: String,
}

/// Author information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    pub email: String,
}

/// Pull request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub number: i64,
    pub base: GitReference,
    pub head: GitReference,
}

/// Git reference (branch or tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitReference {
    pub sha: String,
    pub ref_name: String,
}

impl EventType {
    /// Parse event type from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "push" => Ok(EventType::Push),
            "pull_request" => Ok(EventType::PullRequest),
            "workflow_dispatch" => Ok(EventType::WorkflowDispatch),
            "schedule" => Ok(EventType::Schedule),
            _ => Err(EventError::UnsupportedEvent(s.to_string()).into()),
        }
    }
}

/// Parse event context from configuration
#[cfg(feature = "native")]
pub async fn parse_event_context(config: &Config) -> Result<EventContext> {
    // Parse event type
    let event_type = EventType::from_str(&config.event_name)?;

    // Read event JSON file
    let event_json = read_event_file(&config.event_path)?;

    // Parse repository information
    let repository = parse_repository(&event_json, config)?;

    // Route to event-specific parser
    match event_type {
        EventType::Push => parse_push_event(&event_json, repository, config).await,
        EventType::PullRequest => parse_pull_request_event(&event_json, repository, config).await,
        EventType::WorkflowDispatch => parse_workflow_dispatch_event(repository),
        EventType::Schedule => parse_schedule_event(repository),
    }
}

/// Read event JSON file
fn read_event_file(path: &Path) -> Result<serde_json::Value> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| EventError::InvalidEventJson(format!("Failed to read event file: {}", e)))?;

    serde_json::from_str(&contents)
        .map_err(|e| EventError::InvalidEventJson(format!("Failed to parse JSON: {}", e)).into())
}

/// Parse repository from event JSON
fn parse_repository(event_json: &serde_json::Value, config: &Config) -> Result<Repository> {
    // Try to extract repository from event
    if let Some(repo_obj) = event_json.get("repository") {
        let owner = repo_obj["owner"]["login"]
            .as_str()
            .ok_or_else(|| EventError::MissingField("repository.owner.login".to_string()))?
            .to_string();

        let name = repo_obj["name"]
            .as_str()
            .ok_or_else(|| EventError::MissingField("repository.name".to_string()))?
            .to_string();

        let full_name = repo_obj["full_name"]
            .as_str()
            .ok_or_else(|| EventError::MissingField("repository.full_name".to_string()))?
            .to_string();

        let html_url = repo_obj["html_url"]
            .as_str()
            .ok_or_else(|| EventError::MissingField("repository.html_url".to_string()))?
            .to_string();

        Ok(Repository {
            owner,
            name,
            full_name,
            html_url,
        })
    } else {
        // Fallback for schedule events where repository may be undefined
        let full_name = config.repository.clone();
        let owner = config.repository_owner.clone();
        let name = full_name.trim_start_matches(&format!("{}/", owner)).to_string();
        let html_url = format!("https://github.com/{}", full_name);

        Ok(Repository {
            owner,
            name,
            full_name,
            html_url,
        })
    }
}

/// Parse push event
#[cfg(feature = "native")]
async fn parse_push_event(
    event_json: &serde_json::Value,
    repository: Repository,
    config: &Config,
) -> Result<EventContext> {
    let commits_array = event_json["commits"]
        .as_array()
        .ok_or_else(|| EventError::MissingField("commits".to_string()))?;

    if commits_array.is_empty() {
        return Err(EventError::NoCommits.into());
    }

    let commits: Vec<Commit> = commits_array
        .iter()
        .filter_map(|c| parse_commit(c))
        .collect();

    if commits.is_empty() {
        return Err(EventError::NoCommits.into());
    }

    // Determine base and head refs
    let base_ref = config.base_ref.clone().unwrap_or_else(|| commits[0].sha.clone());
    let head_ref = commits.last().unwrap().sha.clone();

    Ok(EventContext {
        event_type: EventType::Push,
        repository,
        base_ref,
        head_ref,
        commits,
        pull_request: None,
    })
}

/// Parse pull request event
#[cfg(feature = "native")]
async fn parse_pull_request_event(
    event_json: &serde_json::Value,
    repository: Repository,
    config: &Config,
) -> Result<EventContext> {
    let pr_obj = event_json["pull_request"]
        .as_object()
        .ok_or_else(|| EventError::MissingField("pull_request".to_string()))?;

    let pr_number = pr_obj["number"]
        .as_i64()
        .ok_or_else(|| EventError::MissingField("pull_request.number".to_string()))?;

    let base_sha = pr_obj["base"]["sha"]
        .as_str()
        .ok_or_else(|| EventError::MissingField("pull_request.base.sha".to_string()))?
        .to_string();

    let base_ref_name = pr_obj["base"]["ref"]
        .as_str()
        .ok_or_else(|| EventError::MissingField("pull_request.base.ref".to_string()))?
        .to_string();

    let head_sha = pr_obj["head"]["sha"]
        .as_str()
        .ok_or_else(|| EventError::MissingField("pull_request.head.sha".to_string()))?
        .to_string();

    let head_ref_name = pr_obj["head"]["ref"]
        .as_str()
        .ok_or_else(|| EventError::MissingField("pull_request.head.ref".to_string()))?
        .to_string();

    let pull_request = PullRequest {
        number: pr_number,
        base: GitReference {
            sha: base_sha.clone(),
            ref_name: base_ref_name,
        },
        head: GitReference {
            sha: head_sha.clone(),
            ref_name: head_ref_name,
        },
    };

    // Fetch PR commits to determine exact scan range
    let pr_commits = crate::github::fetch_pr_commits(config, &repository, pr_number).await?;

    if pr_commits.is_empty() {
        return Err(EventError::NoCommits.into());
    }

    let base_ref = config
        .base_ref
        .clone()
        .unwrap_or_else(|| pr_commits[0].sha.clone());
    let head_ref = pr_commits.last().unwrap().sha.clone();

    Ok(EventContext {
        event_type: EventType::PullRequest,
        repository,
        base_ref,
        head_ref,
        commits: pr_commits,
        pull_request: Some(pull_request),
    })
}

/// Parse workflow dispatch event
fn parse_workflow_dispatch_event(repository: Repository) -> Result<EventContext> {
    Ok(EventContext {
        event_type: EventType::WorkflowDispatch,
        repository,
        base_ref: String::new(),
        head_ref: String::new(),
        commits: Vec::new(),
        pull_request: None,
    })
}

/// Parse schedule event
fn parse_schedule_event(repository: Repository) -> Result<EventContext> {
    Ok(EventContext {
        event_type: EventType::Schedule,
        repository,
        base_ref: String::new(),
        head_ref: String::new(),
        commits: Vec::new(),
        pull_request: None,
    })
}

/// Parse a single commit from JSON
fn parse_commit(commit_json: &serde_json::Value) -> Option<Commit> {
    Some(Commit {
        sha: commit_json["id"].as_str()?.to_string(),
        author: Author {
            name: commit_json["author"]["name"].as_str()?.to_string(),
            email: commit_json["author"]["email"].as_str()?.to_string(),
        },
        message: commit_json["message"].as_str()?.to_string(),
    })
}

/// Build log-opts for gitleaks based on event context
pub fn build_log_opts(context: &EventContext) -> String {
    match context.event_type {
        EventType::Push => {
            if context.base_ref == context.head_ref {
                // Single commit
                "-1".to_string()
            } else {
                // Range scan
                format!(
                    "--no-merges --first-parent {}^..{}",
                    context.base_ref, context.head_ref
                )
            }
        }
        EventType::PullRequest => {
            // Always range scan for PRs
            format!(
                "--no-merges --first-parent {}^..{}",
                context.base_ref, context.head_ref
            )
        }
        EventType::WorkflowDispatch | EventType::Schedule => {
            // Full repository scan - no log-opts
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_from_str() {
        assert_eq!(EventType::from_str("push").unwrap(), EventType::Push);
        assert_eq!(
            EventType::from_str("pull_request").unwrap(),
            EventType::PullRequest
        );
        assert_eq!(
            EventType::from_str("workflow_dispatch").unwrap(),
            EventType::WorkflowDispatch
        );
        assert_eq!(EventType::from_str("schedule").unwrap(), EventType::Schedule);
        assert!(EventType::from_str("invalid").is_err());
    }

    #[test]
    fn test_build_log_opts() {
        let context = EventContext {
            event_type: EventType::Push,
            repository: Repository {
                owner: "owner".to_string(),
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                html_url: "https://github.com/owner/repo".to_string(),
            },
            base_ref: "abc123".to_string(),
            head_ref: "def456".to_string(),
            commits: Vec::new(),
            pull_request: None,
        };

        assert_eq!(
            build_log_opts(&context),
            "--no-merges --first-parent abc123^..def456"
        );

        // Single commit
        let context = EventContext {
            event_type: EventType::Push,
            repository: Repository {
                owner: "owner".to_string(),
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                html_url: "https://github.com/owner/repo".to_string(),
            },
            base_ref: "abc123".to_string(),
            head_ref: "abc123".to_string(),
            commits: Vec::new(),
            pull_request: None,
        };

        assert_eq!(build_log_opts(&context), "-1");

        // Full scan
        let context = EventContext {
            event_type: EventType::WorkflowDispatch,
            repository: Repository {
                owner: "owner".to_string(),
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                html_url: "https://github.com/owner/repo".to_string(),
            },
            base_ref: String::new(),
            head_ref: String::new(),
            commits: Vec::new(),
            pull_request: None,
        };

        assert_eq!(build_log_opts(&context), "");
    }
}
