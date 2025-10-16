//! Binary management module for downloading and executing gitleaks
//!
//! This module handles platform detection, binary downloading, caching,
//! and execution of the gitleaks CLI tool.

use crate::config::Config;
use crate::error::{BinaryError, Result};
use std::path::{Path, PathBuf};
use std::process::Stdio;

#[cfg(feature = "native")]
use tokio::process::Command;

/// Platform information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Darwin,
    Windows,
}

/// CPU Architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X64,
    Arm64,
    Arm,
}

/// Gitleaks execution result
#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Platform {
    /// Detect current platform
    pub fn detect() -> Result<Self> {
        match std::env::consts::OS {
            "linux" => Ok(Platform::Linux),
            "macos" => Ok(Platform::Darwin),
            "windows" => Ok(Platform::Windows),
            other => Err(BinaryError::UnsupportedPlatform(other.to_string()).into()),
        }
    }

    /// Get platform string for gitleaks releases
    pub fn as_str(&self) -> &'static str {
        match self {
            Platform::Linux => "linux",
            Platform::Darwin => "darwin",
            Platform::Windows => "windows",
        }
    }

    /// Get file extension for archives
    pub fn archive_ext(&self) -> &'static str {
        match self {
            Platform::Windows => ".zip",
            _ => ".tar.gz",
        }
    }
}

impl Architecture {
    /// Detect current CPU architecture
    pub fn detect() -> Result<Self> {
        match std::env::consts::ARCH {
            "x86_64" | "amd64" => Ok(Architecture::X64),
            "aarch64" | "arm64" => Ok(Architecture::Arm64),
            "arm" => Ok(Architecture::Arm),
            other => Err(BinaryError::UnsupportedArchitecture(other.to_string()).into()),
        }
    }

    /// Get architecture string for gitleaks releases
    pub fn as_str(&self) -> &'static str {
        match self {
            Architecture::X64 => "x64",
            Architecture::Arm64 => "arm64",
            Architecture::Arm => "arm",
        }
    }
}

/// Build download URL for gitleaks binary
pub fn build_download_url(version: &str, platform: Platform, arch: Architecture) -> String {
    let base_url = "https://github.com/zricethezav/gitleaks/releases/download";
    let filename = format!("gitleaks_{}_{}_{}",  version, platform.as_str(), arch.as_str());
    let ext = platform.archive_ext();

    format!("{}/v{}/{}{}", base_url, version, filename, ext)
}

/// Get cache directory for gitleaks binaries
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_root = dirs::cache_dir()
        .ok_or_else(|| BinaryError::CacheError("Cannot determine cache directory".to_string()))?;

    let cache_dir = cache_root.join("secretscout").join("gitleaks");

    // Create cache directory if it doesn't exist
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| BinaryError::CacheError(format!("Failed to create cache dir: {}", e)))?;
    }

    Ok(cache_dir)
}

/// Get cache key for a specific gitleaks version
pub fn get_cache_key(version: &str, platform: Platform, arch: Architecture) -> String {
    format!("gitleaks-{}-{}-{}", version, platform.as_str(), arch.as_str())
}

/// Check if binary exists in cache
pub fn check_cache(version: &str, platform: Platform, arch: Architecture) -> Option<PathBuf> {
    let cache_dir = get_cache_dir().ok()?;
    let cache_key = get_cache_key(version, platform, arch);
    let binary_name = if platform == Platform::Windows {
        "gitleaks.exe"
    } else {
        "gitleaks"
    };

    let cached_path = cache_dir.join(&cache_key).join(binary_name);

    if cached_path.exists() {
        log::info!("Found cached gitleaks binary: {}", cached_path.display());
        Some(cached_path)
    } else {
        None
    }
}

/// Resolve gitleaks version (handles "latest")
#[cfg(feature = "native")]
pub async fn resolve_version(version_input: &str) -> Result<String> {
    if version_input == "latest" {
        log::info!("Resolving 'latest' gitleaks version...");
        fetch_latest_version().await
    } else {
        Ok(version_input.to_string())
    }
}

/// Fetch latest gitleaks version from GitHub API
#[cfg(feature = "native")]
async fn fetch_latest_version() -> Result<String> {
    let url = "https://api.github.com/repos/zricethezav/gitleaks/releases/latest";

    let client = reqwest::Client::builder()
        .user_agent("SecretScout/3.0.0")
        .build()
        .map_err(|e| BinaryError::VersionResolution(e.to_string()))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| BinaryError::VersionResolution(format!("Failed to fetch: {}", e)))?;

    if !response.status().is_success() {
        return Err(BinaryError::VersionResolution(format!(
            "API returned status {}",
            response.status()
        ))
        .into());
    }

    let text = response
        .text()
        .await
        .map_err(|e| BinaryError::VersionResolution(format!("Failed to read response: {}", e)))?;

    let json: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| BinaryError::VersionResolution(format!("Failed to parse JSON: {}", e)))?;

    let tag_name = json["tag_name"]
        .as_str()
        .ok_or_else(|| BinaryError::VersionResolution("No tag_name in response".to_string()))?;

    // Remove 'v' prefix if present
    let version = tag_name.trim_start_matches('v').to_string();
    log::info!("Resolved latest version: {}", version);

    Ok(version)
}

/// Download gitleaks binary
#[cfg(feature = "native")]
pub async fn download_binary(
    version: &str,
    platform: Platform,
    arch: Architecture,
) -> Result<PathBuf> {
    log::info!(
        "Downloading gitleaks v{} for {}/{}",
        version,
        platform.as_str(),
        arch.as_str()
    );

    let url = build_download_url(version, platform, arch);
    log::debug!("Download URL: {}", url);

    let client = reqwest::Client::builder()
        .user_agent("SecretScout/3.0.0")
        .build()
        .map_err(|e| BinaryError::DownloadFailed(e.to_string()))?;

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| BinaryError::DownloadFailed(format!("HTTP request failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(BinaryError::DownloadFailed(format!(
            "HTTP status {}",
            response.status()
        ))
        .into());
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| BinaryError::DownloadFailed(format!("Failed to read response: {}", e)))?;

    // Extract archive
    let cache_dir = get_cache_dir()?;
    let cache_key = get_cache_key(version, platform, arch);
    let extract_dir = cache_dir.join(&cache_key);

    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)
            .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to remove old cache: {}", e)))?;
    }

    std::fs::create_dir_all(&extract_dir)
        .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to create extract dir: {}", e)))?;

    if platform == Platform::Windows {
        extract_zip(&bytes, &extract_dir)?;
    } else {
        extract_tar_gz(&bytes, &extract_dir)?;
    }

    // Find binary in extracted directory
    let binary_name = if platform == Platform::Windows {
        "gitleaks.exe"
    } else {
        "gitleaks"
    };

    let binary_path = extract_dir.join(binary_name);

    if !binary_path.exists() {
        return Err(BinaryError::BinaryNotFound.into());
    }

    // Make executable on Unix-like systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&binary_path)
            .map_err(|e| BinaryError::ChmodFailed(e.to_string()))?
            .permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&binary_path, perms)
            .map_err(|e| BinaryError::ChmodFailed(e.to_string()))?;
    }

    log::info!("Extracted gitleaks binary to: {}", binary_path.display());

    Ok(binary_path)
}

/// Extract tar.gz archive
#[cfg(feature = "native")]
fn extract_tar_gz(bytes: &[u8], dest: &Path) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let decoder = GzDecoder::new(bytes);
    let mut archive = Archive::new(decoder);

    archive
        .unpack(dest)
        .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to unpack tar.gz: {}", e)))?;

    Ok(())
}

/// Extract zip archive
#[cfg(feature = "native")]
fn extract_zip(bytes: &[u8], dest: &Path) -> Result<()> {
    use std::io::Cursor;
    use zip::ZipArchive;

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to open zip: {}", e)))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to read zip entry: {}", e)))?;

        let outpath = dest.join(file.name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to create dir: {}", e)))?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to create parent dir: {}", e)))?;
            }

            let mut outfile = std::fs::File::create(&outpath)
                .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to create file: {}", e)))?;

            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| BinaryError::ExtractionFailed(format!("Failed to write file: {}", e)))?;
        }
    }

    Ok(())
}

/// Obtain gitleaks binary (download or use cached)
#[cfg(feature = "native")]
pub async fn obtain_binary(config: &Config) -> Result<PathBuf> {
    let platform = Platform::detect()?;
    let arch = Architecture::detect()?;
    let version = resolve_version(&config.gitleaks_version).await?;

    // Check cache first
    if let Some(cached_path) = check_cache(&version, platform, arch) {
        return Ok(cached_path);
    }

    // Download and cache
    download_binary(&version, platform, arch).await
}

/// Build gitleaks command-line arguments
pub fn build_arguments(config: &Config, log_opts: &str) -> Vec<String> {
    let mut args = vec![
        "detect".to_string(),
        "--redact".to_string(),
        "-v".to_string(),
        "--exit-code=2".to_string(),
        "--report-format=sarif".to_string(),
        format!("--report-path={}", config.sarif_path().display()),
        "--log-level=debug".to_string(),
    ];

    // Add config file if specified
    if let Some(ref config_path) = config.gitleaks_config {
        args.push(format!("--config={}", config_path.display()));
    }

    // Add log-opts if specified
    if !log_opts.is_empty() {
        args.push(format!("--log-opts={}", log_opts));
    }

    args
}

/// Execute gitleaks binary
#[cfg(feature = "native")]
pub async fn execute_gitleaks(binary_path: &Path, args: &[String], workspace: &Path) -> Result<ExecutionResult> {
    log::info!("Executing gitleaks: {} {}", binary_path.display(), args.join(" "));

    let output = Command::new(binary_path)
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| BinaryError::ExecutionFailed(format!("Failed to spawn process: {}", e)))?;

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    log::debug!("Gitleaks exit code: {}", exit_code);
    log::debug!("Gitleaks stdout:\n{}", stdout);
    log::debug!("Gitleaks stderr:\n{}", stderr);

    // Exit code 1 is an error, 0 = no secrets, 2 = secrets found
    if exit_code == 1 {
        return Err(BinaryError::GitleaksError {
            code: exit_code,
            stderr: stderr.clone(),
        }
        .into());
    }

    Ok(ExecutionResult {
        exit_code,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::detect().unwrap();
        assert!(matches!(
            platform,
            Platform::Linux | Platform::Darwin | Platform::Windows
        ));
    }

    #[test]
    fn test_architecture_detection() {
        let arch = Architecture::detect().unwrap();
        assert!(matches!(
            arch,
            Architecture::X64 | Architecture::Arm64 | Architecture::Arm
        ));
    }

    #[test]
    fn test_build_download_url() {
        let url = build_download_url("8.24.3", Platform::Linux, Architecture::X64);
        assert_eq!(
            url,
            "https://github.com/zricethezav/gitleaks/releases/download/v8.24.3/gitleaks_8.24.3_linux_x64.tar.gz"
        );

        let url = build_download_url("8.24.3", Platform::Windows, Architecture::X64);
        assert_eq!(
            url,
            "https://github.com/zricethezav/gitleaks/releases/download/v8.24.3/gitleaks_8.24.3_windows_x64.zip"
        );
    }

    #[test]
    fn test_cache_key() {
        let key = get_cache_key("8.24.3", Platform::Linux, Architecture::X64);
        assert_eq!(key, "gitleaks-8.24.3-linux-x64");
    }

    #[test]
    fn test_build_arguments() {
        let config = Config {
            github_token: String::new(),
            gitleaks_license: None,
            gitleaks_version: "8.24.3".to_string(),
            gitleaks_config: None,
            enable_summary: true,
            enable_upload_artifact: true,
            enable_comments: true,
            notify_user_list: Vec::new(),
            base_ref: None,
            workspace_path: PathBuf::from("/workspace"),
            event_path: PathBuf::from("/workspace/event.json"),
            event_name: "push".to_string(),
            repository: "owner/repo".to_string(),
            repository_owner: "owner".to_string(),
        };

        let args = build_arguments(&config, "--no-merges");
        assert!(args.contains(&"detect".to_string()));
        assert!(args.contains(&"--redact".to_string()));
        assert!(args.contains(&"--log-opts=--no-merges".to_string()));
    }
}
