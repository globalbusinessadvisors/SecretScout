//! Job summary generation module
//!
//! This module generates GitHub Actions job summaries in HTML/Markdown format.

use crate::events::Repository;
use crate::sarif::types::DetectedSecret;

/// Generate success summary (no secrets found)
pub fn generate_success_summary() -> String {
    "## No leaks detected ✅\n".to_string()
}

/// Generate error summary (gitleaks failed)
pub fn generate_error_summary(exit_code: i32) -> String {
    format!(
        "## ❌ Gitleaks exited with error. Exit code [{}]\n",
        exit_code
    )
}

/// Generate findings summary with HTML table
pub fn generate_findings_summary(repository: &Repository, findings: &[DetectedSecret]) -> String {
    let mut summary = String::from("## 🛑 Gitleaks detected secrets 🛑\n\n");

    summary.push_str("<table>\n");
    summary.push_str("<tr>\n");
    summary.push_str("  <th>Rule ID</th>\n");
    summary.push_str("  <th>Commit</th>\n");
    summary.push_str("  <th>Secret URL</th>\n");
    summary.push_str("  <th>Start Line</th>\n");
    summary.push_str("  <th>Author</th>\n");
    summary.push_str("  <th>Date</th>\n");
    summary.push_str("  <th>Email</th>\n");
    summary.push_str("  <th>File</th>\n");
    summary.push_str("</tr>\n");

    for finding in findings {
        let commit_url = finding.commit_url(&repository.html_url);
        let secret_url = finding.secret_url(&repository.html_url);
        let file_url = finding.file_url(&repository.html_url);
        let short_sha = finding.short_sha();

        summary.push_str("<tr>\n");
        summary.push_str(&format!("  <td>{}</td>\n", escape_html(&finding.rule_id)));
        summary.push_str(&format!(
            "  <td><a href=\"{}\">{}</a></td>\n",
            commit_url, short_sha
        ));
        summary.push_str(&format!(
            "  <td><a href=\"{}\">View Secret</a></td>\n",
            secret_url
        ));
        summary.push_str(&format!("  <td>{}</td>\n", finding.line_number));
        summary.push_str(&format!("  <td>{}</td>\n", escape_html(&finding.author)));
        summary.push_str(&format!("  <td>{}</td>\n", escape_html(&finding.date)));
        summary.push_str(&format!("  <td>{}</td>\n", escape_html(&finding.email)));
        summary.push_str(&format!(
            "  <td><a href=\"{}\">{}</a></td>\n",
            file_url,
            escape_html(&finding.file_path)
        ));
        summary.push_str("</tr>\n");
    }

    summary.push_str("</table>\n");

    summary
}

/// Escape HTML special characters
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Write summary to GITHUB_STEP_SUMMARY file
#[cfg(feature = "native")]
pub fn write_summary(content: &str) -> std::io::Result<()> {
    use std::env;
    use std::fs::OpenOptions;
    use std::io::Write;

    if let Ok(summary_path) = env::var("GITHUB_STEP_SUMMARY") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(summary_path)?;

        file.write_all(content.as_bytes())?;
        file.write_all(b"\n")?;

        log::debug!("Wrote summary to GITHUB_STEP_SUMMARY");
        Ok(())
    } else {
        log::warn!("GITHUB_STEP_SUMMARY not set, cannot write summary");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_success_summary() {
        let summary = generate_success_summary();
        assert!(summary.contains("No leaks detected"));
        assert!(summary.contains("✅"));
    }

    #[test]
    fn test_generate_error_summary() {
        let summary = generate_error_summary(1);
        assert!(summary.contains("Exit code [1]"));
        assert!(summary.contains("❌"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(escape_html("'single'"), "&#39;single&#39;");
    }

    #[test]
    fn test_generate_findings_summary() {
        let repository = Repository {
            owner: "owner".to_string(),
            name: "repo".to_string(),
            full_name: "owner/repo".to_string(),
            html_url: "https://github.com/owner/repo".to_string(),
        };

        let findings = vec![DetectedSecret {
            rule_id: "aws-access-token".to_string(),
            file_path: "src/config.rs".to_string(),
            line_number: 42,
            commit_sha: "abc123def456".to_string(),
            author: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            date: "2025-10-16".to_string(),
            fingerprint: "abc123def456:src/config.rs:aws-access-token:42".to_string(),
        }];

        let summary = generate_findings_summary(&repository, &findings);

        assert!(summary.contains("🛑"));
        assert!(summary.contains("<table>"));
        assert!(summary.contains("aws-access-token"));
        assert!(summary.contains("abc123d")); // short SHA
        assert!(summary.contains("John Doe"));
        assert!(summary.contains("src/config.rs"));
        assert!(summary.contains("https://github.com/owner/repo/commit/abc123def456"));
    }
}
