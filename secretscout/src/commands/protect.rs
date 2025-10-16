//! Protect command - scan staged changes

use crate::{binary, error::Result};
use std::path::Path;

pub async fn protect(
    source: &Path,
    staged: bool,
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
        "protect".to_string(),
        "--source".to_string(),
        source.display().to_string(),
    ];

    if staged {
        args.push("--staged".to_string());
    }

    if verbose {
        args.push("-v".to_string());
        args.push("--log-level=debug".to_string());
    }

    if let Some(config) = config_path {
        args.push("--config".to_string());
        args.push(config.display().to_string());
    }

    // Execute gitleaks
    let result = binary::execute_gitleaks(&gitleaks_path, &args, source).await?;

    match result.exit_code {
        0 => {
            println!("No secrets in staged changes");
            Ok(())
        }
        1 => {
            eprintln!("Secrets found in staged changes");
            eprintln!("{}", result.stdout);
            std::process::exit(1);
        }
        code => {
            eprintln!("Error: gitleaks exited with code {}", code);
            eprintln!("{}", result.stderr);
            std::process::exit(code);
        }
    }
}
