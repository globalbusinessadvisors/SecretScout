//! SecretScout - Rust/WASM library
//!
//! This is the library entry point that exposes modules for both native
//! and WASM targets.

#![warn(missing_docs)]
#![warn(clippy::all)]

// Core modules (available in both native and WASM)
pub mod config;
pub mod error;
pub mod events;
pub mod sarif;
pub mod outputs;

// Native-only modules
#[cfg(feature = "native")]
pub mod binary;

#[cfg(feature = "native")]
pub mod github;

// WASM-specific exports
#[cfg(feature = "wasm")]
pub mod wasm;

// Re-exports for convenience
pub use config::Config;
pub use error::{Error, Result};
pub use events::{EventContext, EventType};
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
