//! Shell process discovery and PTY process lifecycle.
//!
//! Handles universal dynamic shell detection (Git Bash, Scoop, Chocolatey,
//! Winget, MSYS2, WSL, PowerShell, CMD) with automatic `~/.bashrc` sourcing,
//! and spawns a backend session by attempting candidates in cascade order.

use eframe::egui;
use egui_term::{BackendSettings, TerminalBackend};
use std::sync::mpsc;

use super::session::TerminalSession;

/// A shell candidate with executable path, startup arguments, and user-facing name.
#[derive(Clone, Debug)]
pub struct ShellCandidate {
    pub path: String,
    pub args: Vec<String>,
    pub name: String,
}

/// Universal Dynamic Shell Discovery:
/// Finds Bash wherever the user installed it (standard paths, custom directories,
/// PATH, Scoop, Chocolatey, Winget, MSYS2, WSL) and cascades to PowerShell or CMD.
/// Never panics, never fails to spawn.
pub fn get_shell_candidates() -> Vec<ShellCandidate> {
    let mut candidates = Vec::new();

    // Ensure HOME is mapped to USERPROFILE on Windows so Git Bash sources ~/.bashrc and ~/.bash_profile
    if cfg!(windows) && std::env::var("HOME").is_err() {
        if let Ok(user_profile) = std::env::var("USERPROFILE") {
            std::env::set_var("HOME", &user_profile);
        }
    }

    if cfg!(windows) {
        let push_bash = |candidates: &mut Vec<ShellCandidate>, path: String| {
            if std::path::Path::new(&path).exists() {
                candidates.push(ShellCandidate {
                    path,
                    args: vec!["--login".into()],
                    name: "bash".into(),
                });
            }
        };

        // 1 + 2. Single pass over PATH: find bash.exe directly, and discover the
        // bash sibling of any git.exe on PATH (e.g. D:\Custom\Git\cmd\git.exe →
        // D:\Custom\Git\bin\bash.exe). One pass keeps terminal startup fast.
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let bash_exe = dir.join("bash.exe");
                let dir_str = dir.to_string_lossy().to_lowercase();
                if !dir_str.contains("system32") && bash_exe.exists() {
                    push_bash(
                        &mut candidates,
                        bash_exe.to_string_lossy().to_string(),
                    );
                }
                if dir.join("git.exe").exists() {
                    if let Some(parent) = dir.parent() {
                        push_bash(
                            &mut candidates,
                            parent.join("bin").join("bash.exe").to_string_lossy().to_string(),
                        );
                        push_bash(
                            &mut candidates,
                            parent
                                .join("usr")
                                .join("bin")
                                .join("bash.exe")
                                .to_string_lossy()
                                .to_string(),
                        );
                    }
                }
            }
        }

        // 3 + 4. Only probe fixed install locations, package managers, and common
        // drives when PATH did not already yield a bash. Scanning non-existent
        // drives/prefixes is the slowest part, so we skip it in the common case
        // and fall straight through to the guaranteed PowerShell / cmd fallbacks.
        if candidates.is_empty() {
            // Common drives (C:–F:), skipped entirely when the drive is absent.
            for drive in ['C', 'D', 'E', 'F'] {
                if !std::path::Path::new(&format!("{}:\\", drive)).exists() {
                    continue;
                }
                let prefixes = [
                    format!("{}:\\Program Files\\Git\\bin\\bash.exe", drive),
                    format!("{}:\\Program Files\\Git\\usr\\bin\\bash.exe", drive),
                    format!("{}:\\Program Files (x86)\\Git\\bin\\bash.exe", drive),
                    format!("{}:\\Program Files (x86)\\Git\\usr\\bin\\bash.exe", drive),
                    format!("{}:\\Git\\bin\\bash.exe", drive),
                    format!("{}:\\Git\\usr\\bin\\bash.exe", drive),
                    format!("{}:\\Tools\\Git\\bin\\bash.exe", drive),
                    format!("{}:\\msys64\\usr\\bin\\bash.exe", drive),
                    format!("{}:\\cygwin64\\bin\\bash.exe", drive),
                ];
                for p in prefixes {
                    push_bash(&mut candidates, p);
                }
            }

            // User profile & package managers (AppData, Scoop, Chocolatey)
            if let Ok(local) = std::env::var("LOCALAPPDATA") {
                push_bash(
                    &mut candidates,
                    format!("{}\\Programs\\Git\\bin\\bash.exe", local),
                );
                push_bash(
                    &mut candidates,
                    format!("{}\\Programs\\Git\\usr\\bin\\bash.exe", local),
                );
            }
            if let Ok(userprofile) = std::env::var("USERPROFILE") {
                push_bash(
                    &mut candidates,
                    format!("{}\\scoop\\apps\\git\\current\\bin\\bash.exe", userprofile),
                );
                push_bash(
                    &mut candidates,
                    format!("{}\\scoop\\shims\\bash.exe", userprofile),
                );
            }
            push_bash(
                &mut candidates,
                "C:\\tools\\git\\bin\\bash.exe".to_string(),
            );
        }

        // WSL (Windows Subsystem for Linux) — a cheap existence check.
        let wsl_path = "C:\\Windows\\System32\\wsl.exe";
        if std::path::Path::new(wsl_path).exists() {
            candidates.push(ShellCandidate {
                path: wsl_path.to_string(),
                args: vec!["-e".into(), "bash".into()],
                name: "wsl".into(),
            });
        }

        // PowerShell Core (pwsh.exe) and Windows PowerShell
        candidates.push(ShellCandidate {
            path: "pwsh.exe".into(),
            args: vec!["-NoLogo".into()],
            name: "pwsh".into(),
        });
        candidates.push(ShellCandidate {
            path: "powershell.exe".into(),
            args: vec!["-NoLogo".into()],
            name: "powershell".into(),
        });

        // Command Prompt (Ultimate fallback, always present on Windows)
        let comspec =
            std::env::var("COMSPEC").unwrap_or_else(|_| "C:\\Windows\\System32\\cmd.exe".into());
        candidates.push(ShellCandidate {
            path: comspec,
            args: vec![],
            name: "cmd".into(),
        });
    } else {
        // Unix (Linux, macOS)
        if let Ok(shell) = std::env::var("SHELL") {
            let is_bash = shell.ends_with("bash");
            candidates.push(ShellCandidate {
                path: shell,
                args: if is_bash {
                    vec!["--login".into()]
                } else {
                    vec![]
                },
                name: "bash".into(),
            });
        }
        candidates.push(ShellCandidate {
            path: "/bin/bash".into(),
            args: vec!["--login".into()],
            name: "bash".into(),
        });
        candidates.push(ShellCandidate {
            path: "/bin/zsh".into(),
            args: vec!["-l".into(), "-i".into()],
            name: "zsh".into(),
        });
        candidates.push(ShellCandidate {
            path: "/bin/sh".into(),
            args: vec![],
            name: "sh".into(),
        });
    }

    // Deduplicate candidates
    let mut seen = std::collections::HashSet::new();
    candidates.retain(|c| seen.insert(c.path.clone()));

    candidates
}

/// Spawns a backend session by attempting candidates in cascade order until one succeeds.
pub(crate) fn spawn_backend_session(
    id: u64,
    ctx: &egui::Context,
    session_num: usize,
    working_dir: Option<std::path::PathBuf>,
) -> anyhow::Result<TerminalSession> {
    let candidates = get_shell_candidates();
    let (tx, rx) = mpsc::channel();

    // Prefer the active workspace root so shells open in the project the user
    // is working in; fall back to the process CWD when no workspace is open.
    let working_directory = working_dir.or_else(|| std::env::current_dir().ok());

    for candidate in candidates {
        let mut settings = BackendSettings::default();
        settings.shell = candidate.path.clone();
        settings.args = candidate.args.clone();
        settings.working_directory = working_directory.clone();

        match TerminalBackend::new(id, ctx.clone(), tx.clone(), settings) {
            Ok(backend) => {
                let title = format!("{} {}", candidate.name, session_num);
                return Ok(TerminalSession {
                    id,
                    title,
                    shell_type: candidate.name,
                    backend,
                    event_rx: rx,
                });
            }
            Err(_) => continue, // Try next candidate in the cascade
        }
    }

    anyhow::bail!("Failed to spawn any valid shell candidate.")
}
