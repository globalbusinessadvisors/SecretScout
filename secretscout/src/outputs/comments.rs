//! PR comment generation and posting module

use crate::config::Config;
use crate::error::Result;
use crate::events::EventContext;
use crate::github::{self, PRComment};
use crate::sarif::types::DetectedSecret;

/// Post PR comments for detected secrets
#[cfg(feature = "native")]
pub async fn post_pr_comments(
    config: &Config,
    context: &EventContext,
    findings: &[DetectedSecret],
) -> Result<usize> {
    // Verify this is a PR event
    let pr = match &context.pull_request {
        Some(pr) => pr,
        None => {
            log::error!("Cannot post PR comments: not a pull request event");
            return Ok(0);
        }
    };

    log::info!("Posting comments for {} findings on PR #{}", findings.len(), pr.number);

    // Fetch existing comments for deduplication
    let existing_comments = match github::fetch_pr_comments(config, &context.repository, pr.number).await {
        Ok(comments) => comments,
        Err(e) => {
            log::warn!("Failed to fetch existing comments: {}. Continuing without deduplication.", e);
            Vec::new()
        }
    };

    let mut posted = 0;
    let mut skipped = 0;

    for finding in findings {
        let comment_body = github::build_comment_body(
            &finding.rule_id,
            &finding.commit_sha,
            &finding.fingerprint,
            &config.notify_user_list,
        );

        // Check for duplicates
        if github::is_duplicate_comment(
            &existing_comments,
            &comment_body,
            &finding.file_path,
            finding.line_number,
        ) {
            log::debug!(
                "Skipping duplicate comment for {}:{}",
                finding.file_path,
                finding.line_number
            );
            skipped += 1;
            continue;
        }

        let comment = PRComment {
            body: comment_body,
            commit_id: finding.commit_sha.clone(),
            path: finding.file_path.clone(),
            line: finding.line_number,
            side: "RIGHT".to_string(),
        };

        // Post comment (non-fatal errors)
        match github::post_pr_comment(config, &context.repository, pr.number, &comment).await {
            Ok(_) => {
                log::debug!("Posted comment on {}:{}", finding.file_path, finding.line_number);
                posted += 1;
            }
            Err(e) => {
                log::warn!(
                    "Failed to post comment on {}:{}: {}",
                    finding.file_path,
                    finding.line_number,
                    e
                );
                // Continue with other comments
            }
        }
    }

    log::info!("Posted {} PR comments, skipped {} duplicates", posted, skipped);

    Ok(posted)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock tests would go here - they require complex setup with API mocking
    // For now, the integration will be tested at a higher level
}
