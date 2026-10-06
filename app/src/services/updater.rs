//! In-app GitHub updater for MindForge.
//!
//! Provides a complete self-update pipeline:
//! 1. **Check** — Queries GitHub Releases API for the latest version
//! 2. **Download** — Streams the platform-specific binary with progress tracking
//! 3. **Apply** — Replaces the current executable and restarts the app
//!
//! All network operations run on background threads. The UI polls
//! `UpdateManager::status()` to display current progress.
//!
//! Platform-specific asset detection supports Windows (.exe/.zip),
//! macOS (.dmg/.tar.gz), and Linux (.deb/.tar.gz/.AppImage).

use serde::Deserialize;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Current status of the updater state machine.
///
/// Transitions: Idle → Checking → UpToDate | UpdateAvailable → Downloading → ReadyToRestart
/// Any state can transition to `Error(String)` on failure.
#[derive(Debug, Clone, PartialEq)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate {
        version: String,
    },
    UpdateAvailable {
        new_version: String,
        current_version: String,
        release_notes: String,
        asset_url: String,
        asset_name: String,
        asset_size: u64,
    },
    Downloading {
        new_version: String,
        progress: f32,
        downloaded_bytes: u64,
        total_bytes: u64,
    },
    ReadyToRestart {
        new_version: String,
        downloaded_path: PathBuf,
    },
    Error(String),
}

#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    size: u64,
    browser_download_url: String,
}

/// Thread-safe handle for managing the update lifecycle.
///
/// Cloneable — the UI holds one clone, background threads hold others.
/// All state is protected by a `Mutex` and shared via `Arc`.
#[derive(Clone)]
pub struct UpdateManager {
    status: Arc<Mutex<UpdateStatus>>,
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UpdateManager {
    pub fn new() -> Self {
        Self {
            status: Arc::new(Mutex::new(UpdateStatus::Idle)),
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status
            .lock()
            .map(|s| s.clone())
            .unwrap_or(UpdateStatus::Idle)
    }

    #[allow(dead_code)]
    pub fn set_error(&self, msg: impl Into<String>) {
        if let Ok(mut s) = self.status.lock() {
            *s = UpdateStatus::Error(msg.into());
        }
    }

    /// Triggers an asynchronous check for updates against the GitHub repo.
    ///
    /// Spawns a background thread that queries the GitHub Releases API.
    /// On success, transitions to `UpToDate` or `UpdateAvailable`.
    /// On failure, transitions to `Error` with a descriptive message.
    ///
    /// # Arguments
    /// * `current_version` — The currently running version string (e.g. "0.1.14")
    pub fn check_for_updates(&self, current_version: &str) {
        let status = self.status.clone();
        let cur_ver = current_version.to_string();

        if let Ok(mut s) = status.lock() {
            *s = UpdateStatus::Checking;
        }

        std::thread::spawn(move || {
            let repo = "Saboor-Hamedi/mindforge";
            let url = format!("https://api.github.com/repos/{repo}/releases/latest");

            let agent = ureq::builder()
                .timeout(std::time::Duration::from_secs(12))
                .user_agent("mindforge-updater/1.0")
                .build();

            let response = match agent.get(&url).call() {
                Ok(resp) => resp,
                Err(ureq::Error::Status(404, _)) => {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(
                            "GitHub repository or release not found (404)".into(),
                        );
                    }
                    return;
                }
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(format!("Failed to query GitHub: {e}"));
                    }
                    return;
                }
            };

            let release: GithubRelease = match response.into_json() {
                Ok(rel) => rel,
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(format!("Failed to parse release: {e}"));
                    }
                    return;
                }
            };

            let remote_version = release.tag_name.trim_start_matches('v').to_string();
            let is_newer = is_version_newer(&cur_ver, &remote_version);

            if is_newer {
                let os_asset = find_platform_asset(&release.assets);
                if let Some(asset) = os_asset {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::UpdateAvailable {
                            new_version: remote_version,
                            current_version: cur_ver,
                            release_notes: release.body.unwrap_or_default(),
                            asset_url: asset.browser_download_url.clone(),
                            asset_name: asset.name.clone(),
                            asset_size: asset.size,
                        };
                    }
                } else {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(format!(
                            "Update v{} is available, but no matching asset found for this OS.",
                            remote_version
                        ));
                    }
                }
            } else {
                if let Ok(mut s) = status.lock() {
                    *s = UpdateStatus::UpToDate { version: cur_ver };
                }
            }
        });
    }

    /// Downloads the available update with streaming progress updates.
    ///
    /// Spawns a background thread that streams the binary in 64KB chunks,
    /// updating `UpdateStatus::Downloading` with byte counts and progress
    /// percentage after each chunk. On completion, transitions to
    /// `ReadyToRestart` with the downloaded file path.
    pub fn start_download(&self) {
        let (asset_url, new_version, asset_name, total_size) = {
            let s = self.status();
            match s {
                UpdateStatus::UpdateAvailable {
                    asset_url,
                    new_version,
                    asset_name,
                    asset_size,
                    ..
                } => (asset_url, new_version, asset_name, asset_size),
                _ => return,
            }
        };

        let status = self.status.clone();
        if let Ok(mut s) = status.lock() {
            *s = UpdateStatus::Downloading {
                new_version: new_version.clone(),
                progress: 0.0,
                downloaded_bytes: 0,
                total_bytes: total_size,
            };
        }

        std::thread::spawn(move || {
            let agent = ureq::builder()
                .timeout(std::time::Duration::from_secs(300))
                .user_agent("mindforge-updater/1.0")
                .build();

            let response = match agent.get(&asset_url).call() {
                Ok(r) => r,
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(format!("Download failed to connect: {e}"));
                    }
                    return;
                }
            };

            let content_len: u64 = response
                .header("Content-Length")
                .and_then(|h| h.parse().ok())
                .unwrap_or(total_size);

            let temp_dir = std::env::temp_dir();
            let dest_path = temp_dir.join(format!("mindforge_update_{new_version}_{asset_name}"));

            let mut reader = response.into_reader();
            let mut file = match std::fs::File::create(&dest_path) {
                Ok(f) => f,
                Err(e) => {
                    if let Ok(mut s) = status.lock() {
                        *s = UpdateStatus::Error(format!("Failed to create destination file: {e}"));
                    }
                    return;
                }
            };

            let mut buffer = [0u8; 64 * 1024];
            let mut downloaded: u64 = 0;

            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        use std::io::Write;
                        if let Err(e) = file.write_all(&buffer[..n]) {
                            if let Ok(mut s) = status.lock() {
                                *s = UpdateStatus::Error(format!("Write error: {e}"));
                            }
                            return;
                        }
                        downloaded += n as u64;
                        let progress = if content_len > 0 {
                            (downloaded as f32 / content_len as f32).clamp(0.0, 1.0)
                        } else {
                            0.5
                        };

                        if let Ok(mut s) = status.lock() {
                            *s = UpdateStatus::Downloading {
                                new_version: new_version.clone(),
                                progress,
                                downloaded_bytes: downloaded,
                                total_bytes: content_len,
                            };
                        }
                    }
                    Err(e) => {
                        if let Ok(mut s) = status.lock() {
                            *s = UpdateStatus::Error(format!("Streaming read error: {e}"));
                        }
                        return;
                    }
                }
            }

            // Flush file
            use std::io::Write;
            let _ = file.flush();
            drop(file);

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&dest_path)
                    .map(|m| m.permissions())
                    .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o755));
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&dest_path, perms);
            }

            if let Ok(mut s) = status.lock() {
                *s = UpdateStatus::ReadyToRestart {
                    new_version,
                    downloaded_path: dest_path,
                };
            }
        });
    }

    /// Relaunches the application with the downloaded updated executable.
    ///
    /// Platform-specific behavior:
    /// - **Windows**: Writes a batch script that waits for the current process
    ///   to exit, copies the new binary over the old one, and relaunches.
    ///   Handles both NSIS installers and portable executables.
    /// - **Unix**: Copies the binary and uses `exec` to replace the process.
    ///
    /// # Returns
    /// `Ok(())` if the relaunch was initiated, `Err(String)` with a
    /// descriptive error message on failure.
    pub fn restart_and_apply(&self) -> Result<(), String> {
        let downloaded_path = match self.status() {
            UpdateStatus::ReadyToRestart {
                downloaded_path, ..
            } => downloaded_path,
            _ => return Err("No downloaded update is ready to restart.".to_string()),
        };

        let current_exe = std::env::current_exe()
            .map_err(|e| format!("Could not get current executable path: {e}"))?;

        #[cfg(target_os = "windows")]
        {
            let is_installer = downloaded_path
                .file_name()
                .map(|f| {
                    let s = f.to_string_lossy().to_lowercase();
                    s.contains("setup") || s.contains("installer")
                })
                .unwrap_or(false);

            if is_installer {
                // The downloaded asset is the NSIS setup wizard.
                // Using cmd.exe /C start "" properly prompts Windows UAC elevation
                let downloaded_str = downloaded_path.to_string_lossy();
                let res = std::process::Command::new("cmd")
                    .args(["/C", "start", "", &downloaded_str])
                    .spawn();
                match res {
                    Ok(_) => std::process::exit(0),
                    Err(e) => {
                        std::process::Command::new(&downloaded_path)
                            .spawn()
                            .map_err(|e2| format!("Failed to launch installer ({e}, {e2})"))?;
                        std::process::exit(0);
                    }
                }
            } else {
                // Standalone / portable executable update.
                let current_str = current_exe.to_string_lossy();
                let downloaded_str = downloaded_path.to_string_lossy();
                let pid = std::process::id();
                let updater_bat = std::env::temp_dir().join("mindforge_updater.bat");
                let bat_content = format!(
                    "@echo off\r\n\
                     :wait\r\n\
                     timeout /t 1 /nobreak >nul\r\n\
                     tasklist /fi \"PID eq {pid}\" | find \"{pid}\" >nul\r\n\
                     if not errorlevel 1 goto wait\r\n\
                     copy /y \"{downloaded_str}\" \"{current_str}\" >nul\r\n\
                     start \"\" \"{current_str}\"\r\n\
                     del \"%~f0\"\r\n"
                );
                std::fs::write(&updater_bat, bat_content)
                    .map_err(|e| format!("Failed to write update script: {e}"))?;

                std::process::Command::new("cmd")
                    .args([
                        "/C",
                        updater_bat.to_str().unwrap_or("mindforge_updater.bat"),
                    ])
                    .spawn()
                    .map_err(|e| format!("Failed to trigger update script: {e}"))?;

                std::process::exit(0);
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On Unix, replace binary and exec
            std::fs::copy(&downloaded_path, &current_exe)
                .map_err(|e| format!("Failed to replace executable: {e}"))?;

            std::process::Command::new(&current_exe)
                .spawn()
                .map_err(|e| format!("Failed to spawn updated application: {e}"))?;

            std::process::exit(0);
        }
    }
}

/// Simple semantic version comparison: returns true if remote > current.
///
/// Parses dot-separated version strings (ignoring pre-release suffixes
/// like `-beta`). Compares component by component; if all compared
/// components are equal, the version with more components is newer.
fn is_version_newer(current: &str, remote: &str) -> bool {
    let parse_parts = |v: &str| -> Vec<u64> {
        v.trim_start_matches('v')
            .split('.')
            .filter_map(|s| s.split('-').next()) // ignore pre-release tags like -beta
            .filter_map(|s| s.parse::<u64>().ok())
            .collect()
    };

    let cur = parse_parts(current);
    let rem = parse_parts(remote);

    for (c, r) in cur.iter().zip(rem.iter()) {
        if r > c {
            return true;
        } else if r < c {
            return false;
        }
    }
    rem.len() > cur.len()
}

/// Finds the most suitable release asset for the current OS and architecture.
///
/// Uses `cfg!` macros to select the appropriate asset naming convention
/// for the target platform. Falls back to the first matching extension
/// if no platform-specific name is found.
fn find_platform_asset<'a>(assets: &'a [GithubAsset]) -> Option<&'a GithubAsset> {
    #[cfg(target_os = "windows")]
    {
        assets
            .iter()
            .find(|a| {
                let n = a.name.to_lowercase();
                (n.ends_with(".exe") || n.ends_with(".zip"))
                    && (n.contains("win") || n.contains("x86_64") || n.contains("x64"))
            })
            .or_else(|| {
                assets
                    .iter()
                    .find(|a| a.name.to_lowercase().ends_with(".exe"))
            })
    }

    #[cfg(target_os = "macos")]
    {
        assets
            .iter()
            .find(|a| {
                let n = a.name.to_lowercase();
                (n.ends_with(".dmg") || n.ends_with(".tar.gz") || n.ends_with(".zip"))
                    && (n.contains("mac") || n.contains("darwin") || n.contains("apple"))
            })
            .or_else(|| {
                assets
                    .iter()
                    .find(|a| a.name.to_lowercase().ends_with(".dmg"))
            })
    }

    #[cfg(target_os = "linux")]
    {
        assets
            .iter()
            .find(|a| {
                let n = a.name.to_lowercase();
                (n.ends_with(".deb") || n.ends_with(".tar.gz") || n.ends_with(".appimage"))
                    && (n.contains("linux") || n.contains("x86_64"))
            })
            .or_else(|| {
                assets
                    .iter()
                    .find(|a| a.name.to_lowercase().ends_with(".tar.gz"))
            })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        assets.first()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(is_version_newer("0.1.0", "0.2.0"));
        assert!(is_version_newer("0.1.0", "0.1.1"));
        assert!(is_version_newer("0.1.0", "1.0.0"));
        assert!(!is_version_newer("0.2.0", "0.1.0"));
        assert!(!is_version_newer("0.1.0", "0.1.0"));
        assert!(is_version_newer("0.1.0", "0.1.0.1"));
    }
}
