//! CLI argument parsing and command execution

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "secretscout")]
#[command(version, about = "Fast, memory-safe secret detection", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Path to gitleaks config file
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Detect secrets in repository
    Detect {
        /// Path to git repository to scan
        #[arg(short, long, default_value = ".")]
        source: PathBuf,

        /// Path to write SARIF report
        #[arg(short, long, default_value = "results.sarif")]
        report_path: PathBuf,

        /// Report format (sarif, json, csv, text)
        #[arg(short = 'f', long, default_value = "sarif")]
        report_format: String,

        /// Redact secrets in output
        #[arg(long, default_value_t = true)]
        redact: bool,

        /// Exit code when leaks detected
        #[arg(long, default_value_t = 2)]
        exit_code: i32,

        /// Git log options (e.g., "--all", "main..dev")
        #[arg(long)]
        log_opts: Option<String>,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },

    /// Protect staged changes
    Protect {
        /// Path to git repository
        #[arg(short, long, default_value = ".")]
        source: PathBuf,

        /// Scan staged changes only
        #[arg(long, default_value_t = true)]
        staged: bool,

        /// Enable verbose logging
        #[arg(short, long)]
        verbose: bool,
    },

    /// Print version information
    Version,
}

impl Cli {
    pub fn parse_args() -> Self {
        Cli::parse()
    }
}
