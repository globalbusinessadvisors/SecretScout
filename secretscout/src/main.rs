//! SecretScout - Native binary entry point
//!
//! This is the main entry point for the native binary (non-WASM) version
//! of SecretScout. It orchestrates the entire scanning workflow.

use secretscout::{binary, config, error, events, outputs, sarif};
use std::process;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    // Run the main workflow
    let exit_code = match run().await {
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

/// Main workflow execution
async fn run() -> error::Result<i32> {
    log::info!("SecretScout v{} starting...", env!("CARGO_PKG_VERSION"));

    // Step 1: Load configuration
    log::info!("Loading configuration from environment...");
    let config = config::Config::from_env()?;
    log::debug!("Configuration loaded: event={}, repo={}", config.event_name, config.repository);

    // Step 2: Parse event context
    log::info!("Parsing event context...");
    let event_context = events::parse_event_context(&config).await?;
    log::info!("Event type: {:?}", event_context.event_type);
    log::info!("Base ref: {}", event_context.base_ref);
    log::info!("Head ref: {}", event_context.head_ref);

    // Step 3: Obtain gitleaks binary
    log::info!("Obtaining gitleaks binary...");
    let binary_path = binary::obtain_binary(&config).await?;
    log::info!("Using binary: {}", binary_path.display());

    // Step 4: Build gitleaks arguments
    let log_opts = events::build_log_opts(&event_context);
    let args = binary::build_arguments(&config, &log_opts);
    log::debug!("Gitleaks arguments: {:?}", args);

    // Step 5: Execute gitleaks
    log::info!("Executing gitleaks scan...");
    let execution_result = binary::execute_gitleaks(
        &binary_path,
        &args,
        &config.workspace_path,
    )
    .await?;

    // Step 6: Process results based on exit code
    match execution_result.exit_code {
        0 => {
            // No secrets found
            log::info!("✅ No secrets detected");

            if config.enable_summary {
                let summary = outputs::generate_success_summary();
                outputs::write_summary(&summary)?;
            }

            Ok(0)
        }
        2 => {
            // Secrets detected
            log::warn!("🛑 Secrets detected!");

            // Parse SARIF report
            log::info!("Parsing SARIF report...");
            let findings = sarif::parse_and_extract(&config.sarif_path())?;
            log::warn!("Found {} secret(s)", findings.len());

            // Generate outputs (must complete before exiting)
            if config.enable_comments && matches!(event_context.event_type, events::EventType::PullRequest) {
                log::info!("Posting PR comments...");
                match outputs::post_pr_comments(&config, &event_context, &findings).await {
                    Ok(count) => log::info!("Posted {} comments", count),
                    Err(e) => log::warn!("Failed to post some comments: {}", e),
                }
            }

            if config.enable_summary {
                log::info!("Generating job summary...");
                let summary = outputs::generate_findings_summary(&event_context.repository, &findings);
                outputs::write_summary(&summary)?;
            }

            if config.enable_upload_artifact {
                log::info!("SARIF report ready for artifact upload: {}", config.sarif_path().display());
            }

            // Return 1 to fail the workflow when secrets are found
            Ok(1)
        }
        1 => {
            // Gitleaks error
            log::error!("❌ Gitleaks exited with error code 1");
            log::error!("Stderr: {}", execution_result.stderr);

            if config.enable_summary {
                let summary = outputs::generate_error_summary(1);
                outputs::write_summary(&summary)?;
            }

            Ok(1)
        }
        code => {
            // Unexpected exit code
            log::error!("Unexpected gitleaks exit code: {}", code);
            log::error!("Stdout: {}", execution_result.stdout);
            log::error!("Stderr: {}", execution_result.stderr);

            if config.enable_summary {
                let summary = outputs::generate_error_summary(code);
                outputs::write_summary(&summary)?;
            }

            Ok(code)
        }
    }
}
