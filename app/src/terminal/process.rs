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
        // 1. Check PATH for bash.exe directly
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let bash_exe = dir.join("bash.exe");
                let dir_str = dir.to_string_lossy().to_lowercase();
                if bash_exe.exists() && !dir_str.contains("system32") {
                    candidates.push(ShellCandidate {
                        path: bash_exe.to_string_lossy().to_string(),
                        args: vec!["--login".into()],
                        name: "bash".into(),
                    });
                }
            }
        }

        // 2. Check PATH for git.exe -> find sister bash.exe in sibling dirs
        // If git is in PATH (e.g. D:\Custom\Git\cmd\git.exe or E:\Tools\Git\bin\git.exe),
        // we can automatically discover its bin/bash.exe or usr/bin/bash.exe!
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in std::env::split_paths(&path_var) {
                let git_exe = dir.join("git.exe");
                if git_exe.exists() {
                    if let Some(parent) = dir.parent() {
                        let bash1 = parent.join("bin").join("bash.exe");
                        if bash1.exists() {
                            candidates.push(ShellCandidate {
                                path: bash1.to_string_lossy().to_string(),
                                args: vec!["--login".into()],
                                name: "bash".into(),
                            });
                        }
                        let bash2 = parent.join("usr").join("bin").join("bash.exe");
                        if bash2.exists() {
                            candidates.push(ShellCandidate {
                                path: bash2.to_string_lossy().to_string(),
                                args: vec!["--login".into()],
                                name: "bash".into(),
                            });
                        }
                    }
                }
            }
        }

        // 3. Scan common drives (C:, D:, E:, F:) for custom installations
        for drive in ['C', 'D', 'E', 'F'] {
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
                if std::path::Path::new(&p).exists() {
                    candidates.push(ShellCandidate {
                        path: p,
                        args: vec!["--login".into()],
                        name: "bash".into(),
                    });
                }
            }
        }

        // 4. User profile & package managers (AppData, Scoop, Chocolatey)
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            let paths = [
                format!("{}\\Programs\\Git\\bin\\bash.exe", local),
                format!("{}\\Programs\\Git\\usr\\bin\\bash.exe", local),
            ];
            for p in paths {
                if std::path::Path::new(&p).exists() {
                    candidates.push(ShellCandidate {
                        path: p,
                        args: vec!["--login".into()],
                        name: "bash".into(),
                    });
                }
            }
        }

        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let scoop_bash = format!("{}\\scoop\\apps\\git\\current\\bin\\bash.exe", userprofile);
            if std::path::Path::new(&scoop_bash).exists() {
                candidates.push(ShellCandidate {
                    path: scoop_bash,
                    args: vec!["--login".into()],
                    name: "bash".into(),
                });
            }
            let scoop_shim = format!("{}\\scoop\\shims\\bash.exe", userprofile);
            if std::path::Path::new(&scoop_shim).exists() {
                candidates.push(ShellCandidate {
                    path: scoop_shim,
                    args: vec!["--login".into()],
                    name: "bash".into(),
                });
            }
        }

        let choco_bash = "C:\\tools\\git\\bin\\bash.exe";
        if std::path::Path::new(choco_bash).exists() {
            candidates.push(ShellCandidate {
                path: choco_bash.to_string(),
                args: vec!["--login".into()],
                name: "bash".into(),
            });
        }

        // 5. WSL (Windows Subsystem for Linux)
        let wsl_path = "C:\\Windows\\System32\\wsl.exe";
        if std::path::Path::new(wsl_path).exists() {
            candidates.push(ShellCandidate {
                path: wsl_path.to_string(),
                args: vec!["-e".into(), "bash".into()],
                name: "wsl".into(),
            });
        }

        // 6. PowerShell Core (pwsh.exe) and Windows PowerShell
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

        // 7. Command Prompt (Ultimate fallback, always present on Windows)
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
