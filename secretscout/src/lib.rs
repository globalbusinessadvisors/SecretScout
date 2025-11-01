//! # SecretScout
//!
//! A blazingly fast, memory-safe CLI tool for detecting secrets, passwords, API keys,
//! and tokens in git repositories. Built with Rust for maximum performance and safety.
//!
//! ## Features
//!
//! - **10x Faster** - Rust-powered performance with intelligent caching
//! - **Memory Safe** - Zero buffer overflows, crashes, or memory leaks
//! - **Dual Mode** - Use as standalone CLI or GitHub Action
//! - **Pre-commit Hooks** - Protect staged changes before commit
//! - **Multiple Formats** - SARIF, JSON, CSV, text output
//! - **Zero Config** - Works out of the box with sensible defaults
//!
//! ## Installation
//!
//! ### Via cargo
//!
//! ```bash
//! cargo install secretscout
//! ```
//!
//! ### Via npm
//!
//! ```bash
//! npm install -g secretscout
//! ```
//!
//! ## CLI Usage
//!
//! ```bash
//! # Scan a repository
//! secretscout detect
//!
//! # Scan with custom config
//! secretscout detect --config .gitleaks.toml
//!
//! # Protect staged changes (pre-commit hook)
//! secretscout protect --staged
//! ```
//!
//! ## Library Usage
//!
//! This crate can also be used as a library for integrating secret scanning
//! into your own Rust applications:
//!
//! ```no_run
//! use secretscout::{Config, commands};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Scan a repository for secrets
//!     commands::detect(
//!         ".",                    // source path
//!         "results.sarif",        // report path
//!         "sarif",                // format
//!         false,                  // redact
//!         2,                      // exit code on findings
//!         None,                   // log options
//!         None,                   // config path
//!         false,                  // verbose
//!     ).await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! This library exposes modules for both native and WASM targets:
//!
//! - **Native mode** (`feature = "native"`): Full CLI and GitHub Actions functionality
//! - **WASM mode** (`feature = "wasm"`): Browser-based secret scanning
//!
//! ## Safety
//!
//! All code in this crate is memory-safe Rust with no unsafe blocks in the main logic.
//! We leverage Rust's ownership system and type safety to prevent common security vulnerabilities.

#![allow(missing_docs)]
#![warn(clippy::all)]

// Core modules (available in all modes)
pub mod error;

// Native-only modules
#[cfg(feature = "native")]
pub mod binary;

#[cfg(feature = "native")]
pub mod config;

#[cfg(feature = "native")]
pub mod events;

#[cfg(feature = "native")]
pub mod sarif;

#[cfg(feature = "native")]
pub mod outputs;

#[cfg(feature = "native")]
pub mod github;

// CLI-specific modules
#[cfg(feature = "native")]
pub mod cli;

#[cfg(feature = "native")]
pub mod commands;

// GitHub Actions-specific modules
#[cfg(feature = "native")]
pub mod github_actions;

// WASM-specific exports
#[cfg(feature = "wasm")]
pub mod wasm;

// Re-exports for convenience
#[cfg(feature = "native")]
pub use config::Config;

pub use error::{Error, Result};

#[cfg(feature = "native")]
pub use events::{EventContext, EventType};

#[cfg(feature = "native")]
pub use sarif::types::DetectedSecret;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if running in native mode
pub fn is_native() -> bool {
    cfg!(feature = "native")
}

/// Check if running in WASM mode
pub fn is_wasm() -> bool {
    cfg!(feature = "wasm")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_feature_flags() {
        // At least one should be true
        assert!(is_native() || is_wasm());
    }
}
