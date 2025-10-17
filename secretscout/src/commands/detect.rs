//! Detect command - scan repository for secrets

use crate::{binary, error::Result};
use std::path::Path;

#[allow(clippy::too_many_arguments)]
pub async fn detect(
    source: &Path,
    report_path: &Path,
    report_format: &str,
    redact: bool,
    exit_code: i32,
    log_opts: Option<&str>,
    config_path: Option<&Path>,
    verbose: bool,
) -> Result<()> {
    // Ensure gitleaks binary is available
    let platform = binary::Platform::detect()?;
    let arch = binary::Architecture::detect()?;
    let version = binary::resolve_version("8.24.3").await?;

    // Check cache or download
    let gitleaks_path = if let Some(cached) = binary::check_cache(&version, platform, arch) {
        cached
    } else {
        binary::download_binary(&version, platform, arch).await?
    };

    // Build command arguments
    let mut args = vec![
        "detect".to_string(),
        "--source".to_string(),
        source.display().to_string(),
        "--report-path".to_string(),
        report_path.display().to_string(),
        "--report-format".to_string(),
        report_format.to_string(),
        format!("--exit-code={}", exit_code),
    ];

    if redact {
        args.push("--redact".to_string());
    }

    if verbose {
        args.push("-v".to_string());
        args.push("--log-level=debug".to_string());
    }

    if let Some(config) = config_path {
        args.push("--config".to_string());
        args.push(config.display().to_string());
    }

    if let Some(opts) = log_opts {
        args.push(format!("--log-opts={}", opts));
    }

    // Execute gitleaks
    let result = binary::execute_gitleaks(&gitleaks_path, &args, source).await?;

    match result.exit_code {
        0 => {
            println!("No secrets detected");
            Ok(())
        }
        2 => {
            eprintln!("Secrets detected - see {}", report_path.display());
            std::process::exit(1);
        }
        code => {
            eprintln!("Error: gitleaks exited with code {}", code);
            eprintln!("{}", result.stderr);
            std::process::exit(code);
        }
    }
}
