//! Integration tests for SecretScout
//!
//! These tests verify the end-to-end functionality with realistic scenarios.

use secretscout::{config::Config, events, sarif};
use std::env;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a test environment
fn setup_test_env() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let event_file = temp_dir.path().join("event.json");
    (temp_dir, event_file)
}

/// Helper to set required environment variables
fn set_env_vars(workspace: &str, event_path: &str) {
    env::set_var("GITHUB_WORKSPACE", workspace);
    env::set_var("GITHUB_EVENT_PATH", event_path);
    env::set_var("GITHUB_EVENT_NAME", "push");
    env::set_var("GITHUB_REPOSITORY", "test/repo");
    env::set_var("GITHUB_REPOSITORY_OWNER", "test");
    env::set_var("GITHUB_TOKEN", "test-token-123");
}

/// Helper to create a mock push event JSON
fn create_push_event() -> serde_json::Value {
    serde_json::json!({
        "ref": "refs/heads/main",
        "before": "0000000000000000000000000000000000000000",
        "after": "abc123def456",
        "repository": {
            "name": "repo",
            "owner": {
                "name": "test",
                "login": "test"
            },
            "full_name": "test/repo",
            "html_url": "https://github.com/test/repo"
        },
        "commits": [
            {
                "id": "abc123def456",
                "message": "Add new feature",
                "author": {
                    "name": "Test Author",
                    "email": "test@example.com"
                }
            }
        ]
    })
}

/// Helper to create a mock pull request event JSON
fn create_pull_request_event() -> serde_json::Value {
    serde_json::json!({
        "action": "opened",
        "number": 42,
        "pull_request": {
            "number": 42,
            "base": {
                "ref": "main",
                "sha": "base123"
            },
            "head": {
                "ref": "feature-branch",
                "sha": "head456"
            }
        },
        "repository": {
            "name": "repo",
            "owner": {
                "name": "test",
                "login": "test"
            },
            "full_name": "test/repo",
            "html_url": "https://github.com/test/repo"
        }
    })
}

/// Helper to create a mock workflow_dispatch event JSON
fn create_workflow_dispatch_event() -> serde_json::Value {
    serde_json::json!({
        "ref": "refs/heads/main",
        "repository": {
            "name": "repo",
            "owner": {
                "name": "test",
                "login": "test"
            },
            "full_name": "test/repo",
            "html_url": "https://github.com/test/repo"
        }
    })
}

/// Helper to create a mock schedule event JSON
fn create_schedule_event() -> serde_json::Value {
    serde_json::json!({
        "repository": {
            "name": "repo",
            "owner": {
                "name": "test",
                "login": "test"
            },
            "full_name": "test/repo",
            "html_url": "https://github.com/test/repo",
            "default_branch": "main"
        }
    })
}

/// Helper to create a mock SARIF report with findings
fn create_sarif_with_findings() -> serde_json::Value {
    serde_json::json!({
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0-rtm.5.json",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "gitleaks",
                        "version": "8.24.3",
                        "informationUri": "https://gitleaks.io"
                    }
                },
                "results": [
                    {
                        "ruleId": "aws-access-token",
                        "message": {
                            "text": "AWS Access Key ID detected"
                        },
                        "locations": [
                            {
                                "physicalLocation": {
                                    "artifactLocation": {
                                        "uri": "src/config.rs"
                                    },
                                    "region": {
                                        "startLine": 42,
                                        "snippet": {
                                            "text": "AWS_KEY = AKIAIOSFODNN7EXAMPLE"
                                        }
                                    }
                                }
                            }
                        ],
                        "partialFingerprints": {
                            "commitSha": "abc123",
                            "author": "Test Author",
                            "email": "test@example.com",
                            "date": "2025-10-16T12:00:00Z"
                        }
                    },
                    {
                        "ruleId": "generic-api-key",
                        "message": {
                            "text": "Generic API Key detected"
                        },
                        "locations": [
                            {
                                "physicalLocation": {
                                    "artifactLocation": {
                                        "uri": "config/secrets.toml"
                                    },
                                    "region": {
                                        "startLine": 10,
                                        "snippet": {
                                            "text": "api_key = sk_test_1234567890"
                                        }
                                    }
                                }
                            }
                        ],
                        "partialFingerprints": {
                            "commitSha": "def456",
                            "author": "Another Author",
                            "email": "another@example.com",
                            "date": "2025-10-16T11:00:00Z"
                        }
                    }
                ]
            }
        ]
    })
}

#[test]
fn test_config_from_env_push_event() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_push_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );

    // Ensure feature toggle env vars are not set (clean slate for defaults)
    env::remove_var("GITLEAKS_ENABLE_SUMMARY");
    env::remove_var("GITLEAKS_ENABLE_UPLOAD_ARTIFACT");
    env::remove_var("GITLEAKS_ENABLE_COMMENTS");

    let config = Config::from_env().unwrap();

    assert_eq!(config.event_name, "push");
    assert_eq!(config.repository, "test/repo");
    assert_eq!(config.repository_owner, "test");
    assert_eq!(config.github_token, "test-token-123");
    assert_eq!(config.gitleaks_version, "8.24.3");
    assert!(config.enable_summary);
    assert!(config.enable_upload_artifact);
    assert!(config.enable_comments);
}

#[test]
fn test_config_feature_toggles() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_push_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );

    // Test disabling features
    env::set_var("GITLEAKS_ENABLE_SUMMARY", "false");
    env::set_var("GITLEAKS_ENABLE_UPLOAD_ARTIFACT", "0");
    env::set_var("GITLEAKS_ENABLE_COMMENTS", "false");

    let config = Config::from_env().unwrap();

    assert!(!config.enable_summary);
    assert!(!config.enable_upload_artifact);
    assert!(!config.enable_comments);

    // Clean up
    env::remove_var("GITLEAKS_ENABLE_SUMMARY");
    env::remove_var("GITLEAKS_ENABLE_UPLOAD_ARTIFACT");
    env::remove_var("GITLEAKS_ENABLE_COMMENTS");
}

#[test]
fn test_config_user_notification_list() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_push_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );
    env::set_var("GITLEAKS_NOTIFY_USER_LIST", "@user1, @user2, @user3");

    let config = Config::from_env().unwrap();

    assert_eq!(config.notify_user_list, vec!["@user1", "@user2", "@user3"]);

    env::remove_var("GITLEAKS_NOTIFY_USER_LIST");
}

#[tokio::test]
async fn test_parse_push_event() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_push_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );

    let config = Config::from_env().unwrap();
    let context = events::parse_event_context(&config).await.unwrap();

    assert!(matches!(context.event_type, events::EventType::Push));
    assert_eq!(context.repository.name, "repo");
    assert_eq!(context.repository.owner, "test");
    // For push events, base_ref comes from "after" and head_ref from "after"
    assert_eq!(context.base_ref, "abc123def456");
    assert_eq!(context.head_ref, "abc123def456");
    assert_eq!(context.commits.len(), 1);
    assert_eq!(context.commits[0].sha, "abc123def456");
}

#[tokio::test]
async fn test_parse_pull_request_event() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_pull_request_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );
    env::set_var("GITHUB_EVENT_NAME", "pull_request");

    let config = Config::from_env().unwrap();

    // We can't test the full parse because it would try to fetch commits from GitHub
    // But we can verify the event is recognized
    assert_eq!(config.event_name, "pull_request");
}

#[tokio::test]
async fn test_parse_workflow_dispatch_event() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_workflow_dispatch_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );
    env::set_var("GITHUB_EVENT_NAME", "workflow_dispatch");

    let config = Config::from_env().unwrap();
    let context = events::parse_event_context(&config).await.unwrap();

    assert!(matches!(
        context.event_type,
        events::EventType::WorkflowDispatch
    ));
    // For workflow_dispatch, it does a full scan
    assert_eq!(context.base_ref, "");
    assert_eq!(context.head_ref, "");
}

#[tokio::test]
async fn test_parse_schedule_event() {
    let (temp_dir, event_file) = setup_test_env();
    let event_json = create_schedule_event();
    fs::write(
        &event_file,
        serde_json::to_string_pretty(&event_json).unwrap(),
    )
    .unwrap();

    set_env_vars(
        temp_dir.path().to_str().unwrap(),
        event_file.to_str().unwrap(),
    );
    env::set_var("GITHUB_EVENT_NAME", "schedule");

    let config = Config::from_env().unwrap();
    let context = events::parse_event_context(&config).await.unwrap();

    assert!(matches!(context.event_type, events::EventType::Schedule));
    assert_eq!(context.base_ref, ""); // Full scan
    assert_eq!(context.head_ref, ""); // Full scan
}

#[test]
fn test_sarif_parsing_with_findings() {
    let (temp_dir, _) = setup_test_env();
    let sarif_file = temp_dir.path().join("results.sarif");
    let sarif_json = create_sarif_with_findings();
    fs::write(
        &sarif_file,
        serde_json::to_string_pretty(&sarif_json).unwrap(),
    )
    .unwrap();

    let findings = sarif::parse_and_extract(&sarif_file).unwrap();

    assert_eq!(findings.len(), 2);

    // Check first finding
    assert_eq!(findings[0].rule_id, "aws-access-token");
    assert_eq!(findings[0].file_path, "src/config.rs");
    assert_eq!(findings[0].line_number, 42);
    assert_eq!(findings[0].commit_sha, "abc123");
    assert_eq!(findings[0].author, "Test Author");
    assert_eq!(findings[0].email, "test@example.com");
    assert!(findings[0].fingerprint.contains("abc123"));
    assert!(findings[0].fingerprint.contains("aws-access-token"));

    // Check second finding
    assert_eq!(findings[1].rule_id, "generic-api-key");
    assert_eq!(findings[1].file_path, "config/secrets.toml");
    assert_eq!(findings[1].line_number, 10);
    assert_eq!(findings[1].commit_sha, "def456");
}

#[test]
fn test_sarif_parsing_empty_results() {
    let (temp_dir, _) = setup_test_env();
    let sarif_file = temp_dir.path().join("results.sarif");
    let sarif_json = serde_json::json!({
        "version": "2.1.0",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "gitleaks"
                    }
                },
                "results": []
            }
        ]
    });
    fs::write(
        &sarif_file,
        serde_json::to_string_pretty(&sarif_json).unwrap(),
    )
    .unwrap();

    let result = sarif::parse_and_extract(&sarif_file);

    // Empty results returns empty vector, not an error
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[test]
fn test_build_log_opts_for_different_events() {
    // Test push event
    let push_context = events::EventContext {
        event_type: events::EventType::Push,
        repository: events::Repository {
            owner: "test".to_string(),
            name: "repo".to_string(),
            full_name: "test/repo".to_string(),
            html_url: "https://github.com/test/repo".to_string(),
        },
        base_ref: "abc123".to_string(),
        head_ref: "def456".to_string(),
        commits: vec![],
        pull_request: None,
    };

    let log_opts = events::build_log_opts(&push_context);
    // build_log_opts adds git log flags
    assert!(log_opts.contains("abc123"));
    assert!(log_opts.contains("def456"));

    // Test workflow_dispatch (full scan)
    let dispatch_context = events::EventContext {
        event_type: events::EventType::WorkflowDispatch,
        repository: push_context.repository.clone(),
        base_ref: "".to_string(),
        head_ref: "refs/heads/main".to_string(),
        commits: vec![],
        pull_request: None,
    };

    let log_opts = events::build_log_opts(&dispatch_context);
    assert_eq!(log_opts, "");
}

#[test]
fn test_security_git_ref_validation() {
    use secretscout::config::Config;

    // Valid refs
    assert!(Config::validate_git_ref("main").is_ok());
    assert!(Config::validate_git_ref("feature/new-feature").is_ok());
    assert!(Config::validate_git_ref("abc123def456").is_ok());
    assert!(Config::validate_git_ref("").is_ok());

    // Invalid refs with shell metacharacters
    assert!(Config::validate_git_ref("main; rm -rf /").is_err());
    assert!(Config::validate_git_ref("main && whoami").is_err());
    assert!(Config::validate_git_ref("main | ls").is_err());
    assert!(Config::validate_git_ref("main`whoami`").is_err());
    assert!(Config::validate_git_ref("main$USER").is_err());

    // Path traversal attempts
    assert!(Config::validate_git_ref("../../../etc/passwd").is_err());
    assert!(Config::validate_git_ref("../../.git/config").is_err());
}
