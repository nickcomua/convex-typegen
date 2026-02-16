//! Automatic bun binary downloading and caching.
//!
//! This module handles downloading the appropriate bun binary for the current
//! platform and caching it in the project's target directory. This eliminates
//! the need for users to manually install bun.
//!
//! ## Cache Location
//!
//! Bun binaries are cached in `target/.convex-typegen-cache/bun/{version}/`.
//! This is a project-local cache that:
//! - Persists across incremental builds
//! - Gets cleaned with `cargo clean`
//! - Respects `CARGO_TARGET_DIR` environment variable
//! - Can be added to `.gitignore` if desired

use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::errors::ConvexTypeGeneratorError;

const BUN_VERSION: &str = "1.2.6";

/// Get the path to the cached bun binary, downloading it if necessary.
pub(crate) fn get_bun_path() -> Result<PathBuf, ConvexTypeGeneratorError>
{
    // First, check if bun is available in PATH
    if let Ok(output) = std::process::Command::new("bun").arg("--version").output() {
        if output.status.success() {
            // Use system bun if available
            return Ok(PathBuf::from("bun"));
        }
    }

    // Fall back to downloading bun
    let cache_dir = get_cache_dir()?;
    let bun_path = cache_dir.join(get_bun_executable_name());

    if bun_path.exists() && verify_bun_binary(&bun_path)? {
        return Ok(bun_path);
    }

    // Download and install bun
    download_and_install_bun(&cache_dir, &bun_path)?;

    Ok(bun_path)
}

/// Get the cache directory for bun binaries.
/// Uses project-local target directory: target/.convex-typegen-cache/bun/{version}/
fn get_cache_dir() -> Result<PathBuf, ConvexTypeGeneratorError>
{
    // Use CARGO_TARGET_DIR if set (for workspaces), otherwise default to ./target
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target"));

    // Store bun in target/.convex-typegen-cache/bun/{version}/
    let cache_dir = target_dir.join(".convex-typegen-cache").join("bun").join(BUN_VERSION);

    fs::create_dir_all(&cache_dir).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Failed to create cache directory {}: {e}", cache_dir.display()),
    })?;

    Ok(cache_dir)
}

/// Get the platform-specific executable name for bun.
fn get_bun_executable_name() -> &'static str
{
    if cfg!(windows) {
        "bun.exe"
    } else {
        "bun"
    }
}

/// Verify that the bun binary exists and is executable.
fn verify_bun_binary(path: &Path) -> Result<bool, ConvexTypeGeneratorError>
{
    if !path.exists() {
        return Ok(false);
    }

    // Try running `bun --version` to verify it works
    let output = std::process::Command::new(path).arg("--version").output().map_err(|e| {
        ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to verify bun binary: {e}"),
        }
    })?;

    Ok(output.status.success())
}

/// Download and install bun to the cache directory.
fn download_and_install_bun(_cache_dir: &Path, target_path: &Path) -> Result<(), ConvexTypeGeneratorError>
{
    let download_url = get_download_url()?;

    eprintln!("Downloading bun {BUN_VERSION}...");

    // Create a client with timeout to prevent hanging
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to create HTTP client: {e}"),
        })?;

    let response = client
        .get(&download_url)
        .send()
        .map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to download bun from {download_url}: {e}"),
        })?;

    if !response.status().is_success() {
        return Err(ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to download bun: HTTP {} from {download_url}", response.status()),
        });
    }

    let bytes = response.bytes().map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Failed to read download response: {e}"),
    })?;

    // Extract the archive and find the bun binary
    extract_bun_from_archive(&bytes, target_path)?;

    Ok(())
}

/// Get the download URL for the current platform.
fn get_download_url() -> Result<String, ConvexTypeGeneratorError>
{
    let (os, arch) = get_platform_info()?;

    // Bun release URLs follow this pattern:
    // https://github.com/oven-sh/bun/releases/download/bun-v{version}/bun-{os}-{arch}.zip
    let url = format!("https://github.com/oven-sh/bun/releases/download/bun-v{BUN_VERSION}/bun-{os}-{arch}.zip");

    Ok(url)
}

/// Get the OS and architecture for downloading the correct binary.
fn get_platform_info() -> Result<(&'static str, &'static str), ConvexTypeGeneratorError>
{
    let os = if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        return Err(ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Unsupported OS: {}", std::env::consts::OS),
        });
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        return Err(ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Unsupported architecture: {}", std::env::consts::ARCH),
        });
    };

    Ok((os, arch))
}

/// Extract the bun binary from the downloaded archive.
/// The archive structure is: bun-{os}-{arch}/bun (or bun.exe on Windows)
fn extract_bun_from_archive(bytes: &[u8], target_path: &Path) -> Result<(), ConvexTypeGeneratorError>
{
    let cursor = io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Failed to read zip archive: {e}"),
    })?;

    let exe_name = get_bun_executable_name();

    // Find the bun binary in the archive
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to read zip entry: {e}"),
        })?;

        let name = file.name();

        // Look for the bun executable (usually in a subdirectory like bun-darwin-aarch64/bun)
        if name.ends_with(exe_name) && !name.contains("..") {
            let mut outfile = fs::File::create(target_path).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                details: format!("Failed to create file {}: {e}", target_path.display()),
            })?;

            io::copy(&mut file, &mut outfile).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                details: format!("Failed to extract bun binary: {e}"),
            })?;

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(target_path)
                    .map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                        details: format!("Failed to read file metadata: {e}"),
                    })?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(target_path, perms).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to set executable permissions: {e}"),
                })?;
            }

            eprintln!("Bun downloaded successfully to {}", target_path.display());
            return Ok(());
        }
    }

    Err(ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Bun binary '{}' not found in archive", exe_name),
    })
}
