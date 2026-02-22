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
use std::time::Duration;
use std::{fs, io, thread};

use crate::errors::ConvexTypeGeneratorError;

const BUN_VERSION: &str = "1.2.6";

/// RAII guard that removes the lock file when dropped.
struct FileLockGuard
{
    path: PathBuf,
    _file: fs::File,
}

impl Drop for FileLockGuard
{
    fn drop(&mut self)
    {
        let _ = fs::remove_file(&self.path);
    }
}

/// Get the path to the cached bun binary, downloading it if necessary.
///
/// Uses a file lock to prevent concurrent downloads when multiple processes
/// or test threads try to get bun at the same time (avoids "Text file busy" errors).
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

    // Use a lock file to synchronize concurrent access.
    // The lock is held until _lock is dropped (end of this function).
    let lock_path = cache_dir.join(".lock");
    let _lock = acquire_file_lock(&lock_path)?;

    if bun_path.exists() && verify_bun_binary(&bun_path)? {
        return Ok(bun_path);
    }

    // Download and install bun (writes to temp file, then atomically renames)
    download_and_install_bun(&cache_dir, &bun_path)?;

    Ok(bun_path)
}

/// Acquire an exclusive file lock, retrying with backoff.
/// Returns a guard that removes the lock file when dropped.
fn acquire_file_lock(lock_path: &Path) -> Result<FileLockGuard, ConvexTypeGeneratorError>
{
    use std::io::Write;

    let mut attempts = 0;
    let max_attempts = 60; // Up to ~60 seconds total wait

    loop {
        match fs::OpenOptions::new().write(true).create_new(true).open(lock_path) {
            Ok(mut file) => {
                let _ = write!(file, "{}", std::process::id());
                return Ok(FileLockGuard {
                    path: lock_path.to_path_buf(),
                    _file: file,
                });
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                attempts += 1;
                if attempts >= max_attempts {
                    // Stale lock — remove and retry once
                    let _ = fs::remove_file(lock_path);
                    let file = fs::OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(lock_path)
                        .map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                            details: format!("Failed to acquire lock after timeout: {e}"),
                        })?;
                    return Ok(FileLockGuard {
                        path: lock_path.to_path_buf(),
                        _file: file,
                    });
                }
                thread::sleep(Duration::from_secs(1));
            }
            Err(e) => {
                return Err(ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to create lock file: {e}"),
                });
            }
        }
    }
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
/// Retries on "Text file busy" (ETXTBSY) which can occur if another process
/// just finished writing the binary.
fn verify_bun_binary(path: &Path) -> Result<bool, ConvexTypeGeneratorError>
{
    if !path.exists() {
        return Ok(false);
    }

    for attempt in 0..5 {
        match std::process::Command::new(path).arg("--version").output() {
            Ok(output) => return Ok(output.status.success()),
            Err(e) => {
                let is_text_busy = e.raw_os_error() == Some(26); // ETXTBSY
                if is_text_busy && attempt < 4 {
                    thread::sleep(Duration::from_millis(200 * (attempt + 1) as u64));
                    continue;
                }
                return Err(ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to verify bun binary: {e}"),
                });
            }
        }
    }

    Ok(false)
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
/// Writes to a temporary file first, then atomically renames to the target path.
/// This prevents "Text file busy" (ETXTBSY) errors when another process tries to
/// execute the binary while it's still being written.
fn extract_bun_from_archive(bytes: &[u8], target_path: &Path) -> Result<(), ConvexTypeGeneratorError>
{
    let cursor = io::Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Failed to read zip archive: {e}"),
    })?;

    let exe_name = get_bun_executable_name();
    let temp_path = target_path.with_extension(format!("tmp.{}", std::process::id()));

    // Find the bun binary in the archive
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
            details: format!("Failed to read zip entry: {e}"),
        })?;

        let name = file.name();

        // Look for the bun executable (usually in a subdirectory like bun-darwin-aarch64/bun)
        if name.ends_with(exe_name) && !name.contains("..") {
            // Write to a temp file first to avoid ETXTBSY
            let mut outfile = fs::File::create(&temp_path).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                details: format!("Failed to create temp file {}: {e}", temp_path.display()),
            })?;

            io::copy(&mut file, &mut outfile).map_err(|e| {
                let _ = fs::remove_file(&temp_path);
                ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to extract bun binary: {e}"),
                }
            })?;

            // Ensure all data is flushed to disk before setting permissions
            drop(outfile);

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&temp_path)
                    .map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                        details: format!("Failed to read file metadata: {e}"),
                    })?
                    .permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&temp_path, perms).map_err(|e| ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to set executable permissions: {e}"),
                })?;
            }

            // Atomically move the temp file to the target path.
            // This ensures other processes never see a partially-written binary.
            fs::rename(&temp_path, target_path).map_err(|e| {
                let _ = fs::remove_file(&temp_path);
                ConvexTypeGeneratorError::ExtractionFailed {
                    details: format!("Failed to rename temp file to {}: {e}", target_path.display()),
                }
            })?;

            eprintln!("Bun downloaded successfully to {}", target_path.display());
            return Ok(());
        }
    }

    Err(ConvexTypeGeneratorError::ExtractionFailed {
        details: format!("Bun binary '{}' not found in archive", exe_name),
    })
}
