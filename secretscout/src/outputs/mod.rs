//! Output generation module
//!
//! This module handles all output generation including job summaries,
//! PR comments, and artifact handling.

pub mod comments;
pub mod summary;

pub use comments::post_pr_comments;
pub use summary::{
    generate_error_summary, generate_findings_summary, generate_success_summary, write_summary,
};
