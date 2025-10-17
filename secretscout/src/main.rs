//! SecretScout - Fast, memory-safe secret detection
//!
//! Can run in two modes:
//! 1. CLI mode: General-purpose secret scanner like gitleaks
//! 2. GitHub Actions mode: Automated scanning in CI/CD

use secretscout::{config, error};
use std::{env, process};

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Detect mode: CLI vs GitHub Actions
    let mode = detect_mode();

    let exit_code = match mode {
        Mode::Cli => run_cli_mode().await,
        Mode::GitHubActions => run_github_actions_mode().await,
    };

    let exit_code = match exit_code {
        Ok(code) => code,
        Err(e) => {
            log::error!("Fatal error: {}", e);
            match e.severity() {
                error::ErrorSeverity::Expected => 0,
                _ => 1,
            }
        }
    };

    log::info!("Exiting with code: {}", exit_code);
    process::exit(exit_code);
}

enum Mode {
    Cli,
    GitHubActions,
}

fn detect_mode() -> Mode {
    // If specific GitHub env vars are present, use GitHub Actions mode
    // Otherwise, use CLI mode
    if env::var("GITHUB_ACTIONS").is_ok()
        && env::var("GITHUB_WORKSPACE").is_ok()
        && env::var("GITHUB_EVENT_PATH").is_ok()
    {
        Mode::GitHubActions
    } else {
        Mode::Cli
    }
}

async fn run_cli_mode() -> error::Result<i32> {
    use secretscout::cli::{Cli, Commands};

    let cli = Cli::parse_args();

    // Set log level based on verbose flag
    if cli.verbose {
        env::set_var("RUST_LOG", "debug");
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
    }

    match cli.command {
        Commands::Detect {
            source,
            report_path,
            report_format,
            redact,
            exit_code,
            log_opts,
            verbose,
        } => {
            // Run gitleaks detect
            secretscout::commands::detect(
                &source,
                &report_path,
                &report_format,
                redact,
                exit_code,
                log_opts.as_deref(),
                cli.config.as_deref(),
                verbose,
            )
            .await?;
            Ok(0)
        }

        Commands::Protect {
            source,
            staged,
            verbose,
        } => {
            // Run gitleaks protect
            secretscout::commands::protect(&source, staged, cli.config.as_deref(), verbose).await?;
            Ok(0)
        }

        Commands::Version => {
            println!("secretscout {}", env!("CARGO_PKG_VERSION"));
            Ok(0)
        }
    }
}

async fn run_github_actions_mode() -> error::Result<i32> {
    // Original GitHub Actions logic
    let config = config::Config::from_env()?;
    secretscout::github_actions::run(&config).await
}
