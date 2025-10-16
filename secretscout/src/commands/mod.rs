//! Command implementations for CLI

use crate::{binary, error::Result};
use std::path::Path;

pub mod detect;
pub mod protect;

pub use detect::detect;
pub use protect::protect;
