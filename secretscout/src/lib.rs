//! SecretScout - Fast, memory-safe secret detection
//!
//! This is the library entry point that exposes modules for both native
//! and WASM targets, as well as CLI and GitHub Actions functionality.

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
