//! Embedded terminal integration using `egui_term` and `alacritty_terminal`.
//!
//! Provides ultra-lightweight, zero-lag interactive PTY sessions with:
//! - Multi-session management with a right sidebar for switching
//! - Universal dynamic shell detection (Git Bash, Scoop, Chocolatey, Winget,
//!   MSYS2, WSL, PowerShell, CMD) with automatic `~/.bashrc` sourcing
//! - State persistence across dock toggling
//! - Hacker-grade cyber styling with theme-aware color palettes
//! - Full keyboard input forwarding (Ctrl+C, Ctrl+V, arrows, etc.)
//!
//! The terminal runs as a real PTY process — not an emulator — ensuring
//! full compatibility with CLI tools, TUI apps, and shell scripts.

use crate::ui::theme::Theme;
use eframe::egui::{self, pos2, vec2, Align2, Color32, FontId, Key, Modifiers, Rect, Stroke};
use egui_term::{
    BackendCommand, BackendSettings, ColorPalette, PtyEvent, TerminalBackend, TerminalTheme,
    TerminalView,
};
use std::sync::mpsc;

/// Actions that can be triggered from the terminal pane UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalAction {
    None,
    Close,
    RequestFocus,
}

/// A shell candidate with executable path, startup arguments, and user-facing name.
#[derive(Clone, Debug)]
pub struct ShellCandidate {
    pub path: String,
    pub args: Vec<String>,
    pub name: String,
}

/// An individual terminal session with its own PTY process, title, and event channel.
pub struct TerminalSession {
    pub id: u64,
    pub title: String,
    pub shell_type: String,
    pub backend: TerminalBackend,
    pub event_rx: mpsc::Receiver<(u64, PtyEvent)>,
}

pub struct TerminalPane {
    pub sessions: Vec<TerminalSession>,
    pub active_session_idx: usize,
    pub next_session_id: u64,
    pub sessions_sidebar_w: f32,
    pub is_dragging_sessions_splitter: bool,
    pub sessions_scroll_y: f32,
    last_theme_kind: Option<crate::ui::theme::ThemeKind>,
    cached_theme: Option<TerminalTheme>,
}

fn to_hex(c: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
}

fn build_terminal_palette(theme: &Theme) -> ColorPalette {
    let mut palette = ColorPalette::default();
    let bg_hex = to_hex(theme.bg);
    let fg_hex = to_hex(theme.text);
    let accent_hex = to_hex(theme.accent);
    let muted_hex = to_hex(theme.muted);
    let highlight_hex = to_hex(theme.highlight);

    palette.background = bg_hex.clone();
    palette.foreground = fg_hex.clone();
    palette.black = bg_hex.clone();
    palette.dim_black = bg_hex;
    palette.dim_foreground = muted_hex;
    palette.bright_white = highlight_hex.clone();
    palette.white = fg_hex;
    palette.bright_cyan = accent_hex.clone();
    palette.cyan = accent_hex.clone();
    palette.bright_blue = accent_hex.clone();
    palette.blue = accent_hex;
    palette.bright_foreground = Some(highlight_hex);

    palette
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
fn spawn_backend_session(
    id: u64,
    ctx: &egui::Context,
    session_num: usize,
) -> anyhow::Result<TerminalSession> {
    let candidates = get_shell_candidates();
    let (tx, rx) = mpsc::channel();

    for candidate in candidates {
        let mut settings = BackendSettings::default();
        settings.shell = candidate.path.clone();
        settings.args = candidate.args.clone();
        settings.working_directory = std::env::current_dir().ok();

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

impl TerminalPane {
    pub fn spawn(ctx: &egui::Context, theme: &Theme) -> anyhow::Result<Self> {
        let first_session = spawn_backend_session(1, ctx, 1)?;
        let palette = build_terminal_palette(theme);
        let term_theme = TerminalTheme::new(Box::new(palette));

        Ok(Self {
            sessions: vec![first_session],
            active_session_idx: 0,
            next_session_id: 2,
            sessions_sidebar_w: 160.0,
            is_dragging_sessions_splitter: false,
            sessions_scroll_y: 0.0,
            last_theme_kind: Some(theme.kind),
            cached_theme: Some(term_theme),
        })
    }

    /// Spawns a new session and sets it as the active session.
    pub fn new_session(&mut self, ctx: &egui::Context) -> bool {
        let session_num = self.sessions.len() + 1;
        let id = self.next_session_id;
        self.next_session_id += 1;

        match spawn_backend_session(id, ctx, session_num) {
            Ok(session) => {
                self.sessions.push(session);
                self.active_session_idx = self.sessions.len() - 1;
                true
            }
            Err(_) => false,
        }
    }

    /// Closes the session at the specified index.
    /// Returns true if all sessions were closed and the terminal dock should collapse.
    pub fn close_session(&mut self, idx: usize) -> bool {
        if idx < self.sessions.len() {
            self.sessions.remove(idx);
            if self.sessions.is_empty() {
                return true;
            }
            if self.active_session_idx >= self.sessions.len() {
                self.active_session_idx = self.sessions.len() - 1;
            }
        }
        false
    }

    /// Process pending PTY events across all sessions.
    /// If an individual session exits (e.g. user typed 'exit'), remove only that session.
    /// Returns true only if all sessions have exited.
    pub fn pump(&mut self) -> bool {
        let mut exited_ids = Vec::new();
        for session in &mut self.sessions {
            while let Ok((_id, event)) = session.event_rx.try_recv() {
                if let PtyEvent::Exit = event {
                    exited_ids.push(session.id);
                }
            }
        }
        for id in exited_ids {
            if let Some(pos) = self.sessions.iter().position(|s| s.id == id) {
                self.sessions.remove(pos);
            }
        }
        if self.sessions.is_empty() {
            return true;
        }
        if self.active_session_idx >= self.sessions.len() {
            self.active_session_idx = self.sessions.len() - 1;
        }
        false
    }

    /// Feed an egui event directly to the active session's PTY backend with zero latency.
    pub fn feed_event(&mut self, event: &egui::Event, modifiers: Modifiers) {
        if let Some(active) = self.sessions.get_mut(self.active_session_idx) {
            match event {
                egui::Event::Text(text) => {
                    if !modifiers.ctrl && !modifiers.command && !modifiers.alt {
                        active
                            .backend
                            .process_command(BackendCommand::Write(text.as_bytes().to_vec()));
                    }
                }
                egui::Event::Paste(text) => {
                    active
                        .backend
                        .process_command(BackendCommand::Write(text.as_bytes().to_vec()));
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    if let Some(bytes) = map_key_to_terminal_bytes(*key, *modifiers) {
                        active.backend.process_command(BackendCommand::Write(bytes));
                    }
                }
                _ => {}
            }
        }
    }

    /// Renders the docked terminal pane with:
    /// 1. Cyber-styled header showing status pill, active session badge, and close button.
    /// 2. Active `TerminalView` on the left.
    /// 3. Dedicated sessions sidebar on the right with a `+` button to open new concurrent sessions.
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        theme: &Theme,
        font_size: f32,
        is_focused: bool,
        opacity: f32,
    ) -> TerminalAction {
        let mut action = TerminalAction::None;
        let painter = ui.painter().with_clip_rect(rect);

        // Detect mouse click inside dock to request focus
        if ui.rect_contains_pointer(rect) && ui.input(|i| i.pointer.primary_clicked()) {
            action = TerminalAction::RequestFocus;
        }

        // Sessions right sidebar layout with resizable width (collapsed when 1 or fewer sessions)
        let show_sessions_sidebar = self.sessions.len() > 1;
        let min_sidebar_w = 110.0f32;
        let max_sidebar_w = (rect.width() * 0.50).max(min_sidebar_w);
        let sidebar_w = self.sessions_sidebar_w.clamp(min_sidebar_w, max_sidebar_w);
        let splitter_w = 6.0f32;

        let (term_rect, splitter_rect_opt, sessions_rect_opt) = if show_sessions_sidebar {
            let term_rect = Rect::from_min_max(
                rect.min,
                pos2(rect.max.x - sidebar_w - splitter_w, rect.max.y),
            );
            let splitter_rect = Rect::from_min_max(
                pos2(rect.max.x - sidebar_w - splitter_w, rect.min.y),
                pos2(rect.max.x - sidebar_w, rect.max.y),
            );
            let sessions_rect =
                Rect::from_min_max(pos2(rect.max.x - sidebar_w, rect.min.y), rect.max);
            (term_rect, Some(splitter_rect), Some(sessions_rect))
        } else {
            (rect, None, None)
        };

        let header_h = 28.0;

        // --- 1. LEFT TERMINAL HEADER & BODY ---
        let term_header_rect = Rect::from_min_max(
            term_rect.min,
            pos2(term_rect.max.x, term_rect.min.y + header_h),
        );
        let term_body_rect = Rect::from_min_max(
            pos2(term_rect.min.x, term_rect.min.y + header_h),
            term_rect.max,
        );

        // Term header background & bottom divider line
        let term_h_bg = Color32::from_rgba_unmultiplied(
            theme.surface().r(),
            theme.surface().g(),
            theme.surface().b(),
            (opacity * 255.0) as u8,
        );
        painter.rect_filled(term_header_rect, 0.0, term_h_bg);

        // Title on left: Cyberpunk/Hacker aesthetic
        let title_color = if is_focused {
            theme.accent
        } else {
            theme.muted
        };
        painter.text(
            pos2(term_header_rect.min.x + 12.0, term_header_rect.center().y),
            Align2::LEFT_CENTER,
            "TERMINAL",
            FontId::monospace(font_size * 0.75),
            title_color,
        );

        // Current active session badge
        if let Some(active) = self.sessions.get(self.active_session_idx) {
            let tag_text = format!("[PTY-0{}: {}]", self.active_session_idx + 1, active.title);
            painter.text(
                pos2(term_header_rect.min.x + 115.0, term_header_rect.center().y),
                Align2::LEFT_CENTER,
                tag_text,
                FontId::monospace(font_size * 0.70),
                theme.muted,
            );
        }

        // Focus status pill
        let status_text = if is_focused {
            "● ACTIVE (Ctrl+J toggle • Esc editor)"
        } else {
            "○ READY (Click to focus • Ctrl+J)"
        };
        painter.text(
            pos2(term_header_rect.max.x - 62.0, term_header_rect.center().y),
            Align2::RIGHT_CENTER,
            status_text,
            FontId::proportional(font_size * 0.70),
            if is_focused {
                theme.highlight
            } else {
                theme.muted
            },
        );

        // New session (+) button in header (easy access when collapsed)
        let add_btn_center = pos2(term_header_rect.max.x - 40.0, term_header_rect.center().y);
        let add_btn_rect = Rect::from_center_size(add_btn_center, vec2(18.0, 18.0));
        let is_add_hover = ui.rect_contains_pointer(add_btn_rect);
        if is_add_hover {
            painter.rect_filled(
                add_btn_rect,
                3.0,
                theme.surface().lerp_to_gamma(theme.accent, 0.15),
            );
            if ui.input(|i| i.pointer.primary_clicked()) {
                self.new_session(ui.ctx());
                action = TerminalAction::RequestFocus;
            }
        }
        painter.text(
            add_btn_center,
            Align2::CENTER_CENTER,
            "+",
            FontId::proportional(14.0),
            if is_add_hover {
                theme.accent
            } else {
                theme.muted
            },
        );

        // Unified sleek close button on far right of terminal header
        let btn_center = pos2(term_header_rect.max.x - 16.0, term_header_rect.center().y);
        if crate::ui_components::render_close_button(
            ui,
            &painter,
            btn_center,
            20.0,
            theme,
            "term_header_close",
        ) {
            action = TerminalAction::Close;
        }

        // Ensure theme matches current Mindforge theme
        if self.last_theme_kind != Some(theme.kind) || self.cached_theme.is_none() {
            let palette = build_terminal_palette(theme);
            self.cached_theme = Some(TerminalTheme::new(Box::new(palette)));
            self.last_theme_kind = Some(theme.kind);
        }

        // Render active session embedded terminal widget with breathing room
        if let Some(ref term_theme) = self.cached_theme {
            if let Some(active_session) = self.sessions.get_mut(self.active_session_idx) {
                // Uniform padding inside terminal body: 8px horizontal, 6px vertical
                let inner_rect = term_body_rect.shrink2(vec2(8.0, 6.0));
                let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(inner_rect));
                // set_focus(false) guarantees egui_term does not double-process keyboard events;
                // feed_event in input/mod.rs remains the sole, crisp event writer to the backend.
                let term_view = TerminalView::new(&mut child_ui, &mut active_session.backend)
                    .set_size(inner_rect.size())
                    .set_focus(false)
                    .set_theme(term_theme.clone());
                child_ui.add(term_view);
            }
        }

        if let (Some(splitter_rect), Some(sessions_rect)) = (splitter_rect_opt, sessions_rect_opt) {
            // --- 2. SESSIONS RESIZE SPLITTER (Between Terminal & Sessions Sidebar) ---
            let mid_x = splitter_rect.center().x;
            let knob_mid = pos2(mid_x, splitter_rect.center().y);
            let knob_h = 32.0;
            let is_dragging = self.is_dragging_sessions_splitter;
            let knob_w = if is_dragging { 6.0 } else { 4.0 };
            let knob_rect = Rect::from_center_size(knob_mid, vec2(knob_w, knob_h));
            let knob_hit_rect = Rect::from_center_size(knob_mid, vec2(16.0, 40.0));

            let is_splitter_hovered = ui.rect_contains_pointer(knob_hit_rect);
            let primary_down = ui.input(|i| i.pointer.primary_down());
            let primary_pressed = ui.input(|i| {
                i.pointer.primary_clicked()
                    || i.pointer.button_pressed(egui::PointerButton::Primary)
            });

            if is_splitter_hovered && primary_pressed {
                self.is_dragging_sessions_splitter = true;
            }

            if self.is_dragging_sessions_splitter {
                if primary_down {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
                    if let Some(pos) =
                        ui.input(|i| i.pointer.interact_pos().or_else(|| i.pointer.hover_pos()))
                    {
                        let desired_w = rect.max.x - pos.x;
                        self.sessions_sidebar_w = desired_w.clamp(min_sidebar_w, max_sidebar_w);
                        ui.ctx().request_repaint();
                    }
                } else {
                    self.is_dragging_sessions_splitter = false;
                }
            } else if is_splitter_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeColumn);
            }

            let is_splitter_active = is_splitter_hovered || self.is_dragging_sessions_splitter;

            // Tactile knob on the sessions splitter — no harsh full-height line
            let active_knob_rect = if is_splitter_active {
                Rect::from_center_size(knob_mid, vec2(6.0, 34.0))
            } else {
                knob_rect
            };
            painter.rect_filled(
                active_knob_rect,
                2.0,
                if is_splitter_active {
                    theme.accent
                } else {
                    Color32::from_rgba_unmultiplied(
                        theme.muted.r(),
                        theme.muted.g(),
                        theme.muted.b(),
                        110,
                    )
                },
            );
            for dy in [-5.0, 0.0, 5.0] {
                painter.line_segment(
                    [
                        pos2(knob_mid.x - 1.0, knob_mid.y + dy),
                        pos2(knob_mid.x + 1.0, knob_mid.y + dy),
                    ],
                    Stroke::new(1.0_f32, theme.bg),
                );
            }

            // --- 3. RIGHT SIDEBAR: SESSIONS LIST ---
            let sessions_bg = Color32::from_rgba_unmultiplied(
                theme.surface().r(),
                theme.surface().g(),
                theme.surface().b(),
                (opacity * 255.0) as u8,
            );
            painter.rect_filled(sessions_rect, 0.0, sessions_bg);

            // Sidebar Header
            let sessions_header_rect = Rect::from_min_max(
                sessions_rect.min,
                pos2(sessions_rect.max.x, sessions_rect.min.y + header_h),
            );
            painter.rect_filled(sessions_header_rect, 0.0, sessions_bg);
            painter.line_segment(
                [
                    pos2(sessions_header_rect.min.x, sessions_header_rect.max.y),
                    pos2(sessions_header_rect.max.x, sessions_header_rect.max.y),
                ],
                Stroke::new(1.0_f32, theme.border()),
            );

            // Sessions Header Label
            let count_str = format!("SESSIONS ({})", self.sessions.len());
            painter.text(
                pos2(
                    sessions_header_rect.min.x + 10.0,
                    sessions_header_rect.center().y,
                ),
                Align2::LEFT_CENTER,
                count_str,
                FontId::monospace(font_size * 0.68),
                theme.muted,
            );

            // [+] Sleek New Session Button (matching sidebar [+] button: transparent inactive, borderless soft hover)
            let btn_center_x = sessions_rect.max.x - 18.0;
            let new_btn_rect = Rect::from_center_size(
                pos2(btn_center_x, sessions_header_rect.center().y),
                vec2(20.0, 20.0),
            );
            let is_new_btn_hovered = ui.rect_contains_pointer(new_btn_rect);
            if is_new_btn_hovered {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                painter.rect_filled(
                    new_btn_rect,
                    4.0,
                    theme.surface().lerp_to_gamma(theme.accent, 0.15),
                );
                if ui.input(|i| i.pointer.primary_clicked()) {
                    self.new_session(ui.ctx());
                    action = TerminalAction::RequestFocus;
                }
            }
            let plus_color = if is_new_btn_hovered {
                theme.accent
            } else {
                theme.muted
            };
            painter.text(
                new_btn_rect.center(),
                Align2::CENTER_CENTER,
                "+",
                FontId::proportional(14.0),
                plus_color,
            );

            // Scrollable Sessions List Items
            let list_rect = Rect::from_min_max(
                pos2(sessions_rect.min.x, sessions_header_rect.max.y),
                sessions_rect.max,
            );
            let list_painter = painter.with_clip_rect(list_rect);

            let row_height = 28.0f32;
            let row_gap = 4.0f32;
            let total_content_h = (self.sessions.len() as f32) * (row_height + row_gap) + 8.0;
            let visible_list_h = list_rect.height().max(1.0);
            let max_scroll = (total_content_h - visible_list_h).max(0.0);

            if ui.rect_contains_pointer(list_rect) {
                let scroll_delta = ui.input(|i| {
                    if i.smooth_scroll_delta.y.abs() > 0.001 {
                        i.smooth_scroll_delta.y
                    } else {
                        i.raw_scroll_delta.y * 0.5
                    }
                });
                if scroll_delta.abs() > 0.001 {
                    self.sessions_scroll_y =
                        (self.sessions_scroll_y - scroll_delta).clamp(0.0, max_scroll);
                }
            }
            self.sessions_scroll_y = self.sessions_scroll_y.clamp(0.0, max_scroll);

            let list_y_start = list_rect.min.y + 5.0 - self.sessions_scroll_y;
            let mut session_to_close: Option<usize> = None;
            let mut session_to_select: Option<usize> = None;

            for (idx, session) in self.sessions.iter().enumerate() {
                let row_y = list_y_start + (idx as f32) * (row_height + row_gap);
                if row_y + row_height < list_rect.min.y || row_y > list_rect.max.y {
                    continue;
                }

                let row_rect = Rect::from_min_max(
                    pos2(list_rect.min.x + 6.0, row_y),
                    pos2(list_rect.max.x - 6.0, row_y + row_height),
                );

                let is_active = idx == self.active_session_idx;
                let is_hovered = ui.rect_contains_pointer(row_rect);

                // Pill surface: match sidebar notes exactly (no stroke, transparent when inactive)
                if is_active || is_hovered {
                    let bg_color = if is_active {
                        Color32::from_rgba_unmultiplied(
                            theme.accent.r(),
                            theme.accent.g(),
                            theme.accent.b(),
                            24,
                        )
                    } else if theme.is_light() {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 13)
                    } else {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 14)
                    };
                    list_painter.rect_filled(row_rect, 6.0, bg_color);

                    // Left vertical accent bar on active session (matching sidebar notes)
                    if is_active {
                        let bar = Rect::from_min_size(
                            pos2(row_rect.min.x + 1.0, row_rect.min.y + 5.0),
                            vec2(3.0, row_rect.height() - 10.0),
                        );
                        list_painter.rect_filled(bar, 1.5, theme.accent);
                    }
                }

                // Session title (matching sidebar notes typography, color, and clipping)
                let title_x = row_rect.min.x + 12.0;
                let text_color = if is_active {
                    theme.accent
                } else if is_hovered {
                    theme.text
                } else {
                    theme.text.lerp_to_gamma(theme.muted, 0.35)
                };

                let title_display = format!("{}. {}", idx + 1, session.title);
                let text_right_limit = btn_center_x - 12.0;
                let row_clip = Rect::from_min_max(
                    pos2(row_rect.min.x, row_rect.min.y),
                    pos2(text_right_limit, row_rect.max.y),
                )
                .intersect(list_rect);
                let row_painter = list_painter.with_clip_rect(row_clip);

                row_painter.text(
                    pos2(title_x, row_rect.center().y),
                    Align2::LEFT_CENTER,
                    title_display,
                    FontId::proportional(12.5),
                    text_color,
                );

                // Close session button on the right of the row - vertically aligned with header [+] button
                let close_center = pos2(btn_center_x, row_rect.center().y);
                let is_close_hover = ui
                    .rect_contains_pointer(Rect::from_center_size(close_center, vec2(18.0, 18.0)));

                if (is_hovered || is_active)
                    && crate::ui_components::render_close_button(
                        ui,
                        &list_painter,
                        close_center,
                        18.0,
                        theme,
                        ("term_session_close", idx),
                    )
                {
                    session_to_close = Some(idx);
                }

                // Handle clicking the row to select session
                if is_hovered && !is_close_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    session_to_select = Some(idx);
                }
            }

            // Mini vertical scrollbar indicator if overflowing
            if max_scroll > 0.0 && visible_list_h > 30.0 {
                let thumb_h = ((visible_list_h / total_content_h) * visible_list_h)
                    .clamp(18.0, visible_list_h);
                let scroll_ratio = (self.sessions_scroll_y / max_scroll).clamp(0.0, 1.0);
                let thumb_y = list_rect.min.y + scroll_ratio * (visible_list_h - thumb_h);
                let thumb_rect =
                    Rect::from_min_size(pos2(list_rect.max.x - 4.0, thumb_y), vec2(2.5, thumb_h));
                list_painter.rect_filled(
                    thumb_rect,
                    1.25,
                    Color32::from_rgba_unmultiplied(
                        theme.muted.r(),
                        theme.muted.g(),
                        theme.muted.b(),
                        100,
                    ),
                );
            }

            if let Some(idx) = session_to_select {
                self.active_session_idx = idx;
                action = TerminalAction::RequestFocus;
            }

            if let Some(idx) = session_to_close {
                if self.close_session(idx) {
                    action = TerminalAction::Close;
                }
            }
        }

        action
    }
}

/// Translates keys and Ctrl combinations into standard Linux/Bash ANSI sequences.
fn map_key_to_terminal_bytes(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    // Word jumping with Ctrl+Left / Ctrl+Right
    if (modifiers.ctrl || modifiers.command) && key == Key::ArrowLeft {
        return Some(b"\x1b[1;5D".to_vec());
    }
    if (modifiers.ctrl || modifiers.command) && key == Key::ArrowRight {
        return Some(b"\x1b[1;5C".to_vec());
    }

    // Standard Ctrl combinations in Bash/Linux
    if modifiers.ctrl || modifiers.command {
        return match key {
            Key::A => Some(vec![0x01]), // Line start
            Key::B => Some(vec![0x02]), // Back char
            Key::C => Some(vec![0x03]), // SIGINT / Break
            Key::D => Some(vec![0x04]), // EOF / Exit
            Key::E => Some(vec![0x05]), // Line end
            Key::F => Some(vec![0x06]), // Forward char
            Key::G => Some(vec![0x07]), // Bell
            Key::H => Some(vec![0x08]), // Backspace
            Key::I => Some(vec![0x09]), // Tab
            Key::J => Some(vec![0x0a]), // Linefeed
            Key::K => Some(vec![0x0b]), // Kill to end
            Key::L => Some(vec![0x0c]), // Clear screen
            Key::N => Some(vec![0x0e]), // Next line
            Key::P => Some(vec![0x10]), // Prev line
            Key::R => Some(vec![0x12]), // Reverse search
            Key::T => Some(vec![0x14]), // Transpose chars
            Key::U => Some(vec![0x15]), // Delete to start
            Key::V => Some(vec![0x16]), // Literal next / paste
            Key::W => Some(vec![0x17]), // Delete word
            Key::Z => Some(vec![0x1a]), // SIGTSTP / Suspend
            Key::ArrowUp => Some(b"\x1b[1;5A".to_vec()),
            Key::ArrowDown => Some(b"\x1b[1;5B".to_vec()),
            _ => None,
        };
    }

    // Standard navigation and control keys
    match key {
        Key::Enter => Some(vec![b'\r']),
        Key::Backspace => Some(vec![0x7f]),
        Key::Tab => Some(vec![b'\t']),
        Key::Escape => Some(vec![0x1b]),
        Key::ArrowUp => Some(b"\x1b[A".to_vec()),
        Key::ArrowDown => Some(b"\x1b[B".to_vec()),
        Key::ArrowRight => Some(b"\x1b[C".to_vec()),
        Key::ArrowLeft => Some(b"\x1b[D".to_vec()),
        Key::Home => Some(b"\x1b[H".to_vec()),
        Key::End => Some(b"\x1b[F".to_vec()),
        Key::Delete => Some(b"\x1b[3~".to_vec()),
        Key::Insert => Some(b"\x1b[2~".to_vec()),
        Key::PageUp => Some(b"\x1b[5~".to_vec()),
        Key::PageDown => Some(b"\x1b[6~".to_vec()),
        Key::F1 => Some(b"\x1bOP".to_vec()),
        Key::F2 => Some(b"\x1bOQ".to_vec()),
        Key::F3 => Some(b"\x1bOR".to_vec()),
        Key::F4 => Some(b"\x1bOS".to_vec()),
        Key::F5 => Some(b"\x1b[15~".to_vec()),
        Key::F6 => Some(b"\x1b[17~".to_vec()),
        Key::F7 => Some(b"\x1b[18~".to_vec()),
        Key::F8 => Some(b"\x1b[19~".to_vec()),
        Key::F9 => Some(b"\x1b[20~".to_vec()),
        Key::F10 => Some(b"\x1b[21~".to_vec()),
        Key::F11 => Some(b"\x1b[23~".to_vec()),
        Key::F12 => Some(b"\x1b[24~".to_vec()),
        _ => None,
    }
}
