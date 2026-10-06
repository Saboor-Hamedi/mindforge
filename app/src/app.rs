//! Application controller, state management, and egui frame loop.

pub mod init;
pub mod modals;
pub mod notes;
pub mod panes;
pub mod right_pane;
pub mod scan_view;
pub mod shell;
pub mod stats_view;
pub mod terminal_drawer;

#[allow(unused_imports)]
pub use init::default_backup_dir;

use crate::editor::Editor;
use crate::input::{handle_input, window_shortcuts};
use crate::mode::Mode;
use eframe::egui::{Pos2, Rect};

use eframe::egui::{self, Color32, FontId};
use std::time::Duration;

pub use crate::editor::types::EditorMode as EditorInputMode;

#[derive(Clone)]
pub struct OpenNote {
    pub id: i64,
    pub title: String,
    pub editor: Editor,
    pub scroll_y: f32,
    pub is_dirty: bool,
    pub file_path: Option<std::path::PathBuf>,
    pub language_override: Option<crate::language::FileLanguage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightPaneTab {
    Preview,
    AiAgent,
    Backlinks,
    Outline,
}

pub struct App {
    pub workspace: crate::workspace::WorkspaceState,
    pub file_versions: std::collections::HashMap<std::path::PathBuf, u64>,
    pub editor: crate::state::editor::EditorState,
    pub command_bar: crate::state::command_bar::CommandBarState,
    pub misc: crate::state::misc::MiscState,

    // Sleek floating sidebar (Ctrl+B)
    pub sidebar: crate::state::sidebar::SidebarState,

    // All modal dialogs (settings, search, rename, delete, help)
    pub modal: crate::state::modal::ModalState,

    // Active document & sidebar limits
    pub notes: crate::state::notes::NotesState,

    // Multi-note & Documentation Tabs
    pub open_notes: Vec<OpenNote>,
    pub tabs: crate::state::tabs::TabsState,

    // Daily Activity & Writing Story tracking
    pub activity: crate::state::activity::ActivityState,

    // Scrolling & auto-save
    pub scroll_y: f32,
    pub doc_scroll_y: f32,
    pub preview_scroll_y: f32,
    pub preview_open: bool,
    pub inline_mode: bool,
    pub split_ratio: f32,
    pub is_dragging_splitter: bool,
    pub show_line_numbers: bool,
    pub is_dirty: bool,
    pub font_dirty: bool,
    pub last_saved_time: f64,
    pub last_editor_rect: Option<Rect>,
    pub last_ed_origin: Option<Pos2>,
    pub last_ed_font_size: Option<f32>,

    // External services and background engines
    pub services: crate::state::services::ServicesState,

    // Active editing mode (Hybrid vs Vim)
    pub showcmd: crate::ui::showcmd::ShowCmdState,
    pub active_doc_idx: usize,
    pub doc_sidebar_focused: bool,
    pub doc_selected_idx: usize,

    // Webscan (:scan & :scans) state
    pub scan: crate::state::scan::ScanState,

    // Embedded Terminal (:term) docked session state
    pub terminal: crate::state::terminal::TerminalState,

    // Right side pane (Preview and AI Agent tabs)
    pub right_pane: crate::state::right_pane::RightPaneState,
}

impl App {
    pub fn display_file_path(&self, path: &std::path::Path) -> String {
        if let Some(root) = self.workspace.root.as_deref() {
            if let Some(relative) = crate::workspace::relative_display(root, path) {
                return relative;
            }
        }
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into())
    }

    fn apply_external_workspace_renames(&mut self) {
        let renames = self.workspace.take_external_renames();
        if renames.is_empty() { return; }
        for (old, new) in renames {
            self.remap_workspace_prefix(&old, &new);
            let versions: Vec<_> = self.file_versions.iter().map(|(p,v)|(p.clone(),*v)).collect();
            for (path,version) in versions {
                if path.starts_with(&old) { if let Ok(suffix)=path.strip_prefix(&old) {
                    self.file_versions.remove(&path);
                    let moved=new.join(suffix);
                    if let Ok(actual)=crate::workspace::file_fingerprint(&moved) { self.file_versions.insert(moved,actual); }
                    else { self.file_versions.insert(moved,version); }
                } }
            }
            for tab in &mut self.open_notes {
                if let Some(path)=tab.file_path.as_mut() { if path.starts_with(&old) {
                    if let Ok(suffix)=path.strip_prefix(&old) { *path=new.join(suffix); }
                    if *path==new { tab.title=new.file_name().unwrap_or_default().to_string_lossy().into_owned(); }
                } }
            }
        }
        let _=self.workspace.refresh();
        self.persist_workspace_expansion();
        self.save_open_tabs();
    }

    fn remap_workspace_prefix(&mut self,old:&std::path::Path,new:&std::path::Path){
        let expanded:Vec<_>=self.workspace.expanded.iter().cloned().collect();
        for path in expanded {if path.starts_with(old){if let Ok(suffix)=path.strip_prefix(old){self.workspace.expanded.remove(&path);self.workspace.expanded.insert(new.join(suffix));}}}
        let selected:Vec<_>=self.workspace.selected_items.iter().cloned().collect();
        for path in selected {if path.starts_with(old){if let Ok(suffix)=path.strip_prefix(old){self.workspace.selected_items.remove(&path);self.workspace.selected_items.insert(new.join(suffix));}}}
        if let Some(anchor)=self.workspace.selection_anchor.as_mut(){if anchor.starts_with(old){if let Ok(suffix)=anchor.strip_prefix(old){*anchor=new.join(suffix);}}}
    }

    fn persist_workspace_expansion(&self) {
        if let Some(root) = self.workspace.root.as_ref() {
            let key = format!("workspace_expanded:{}", root.to_string_lossy());
            let paths: Vec<String> = self
                .workspace
                .expanded
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect();
            if let Ok(value) = serde_json::to_string(&paths) {
                let _ = self
                    .services
                    .db_tx
                    .send(crate::services::db_worker::DbMsg::SaveSetting {
                        key,
                        val: value,
                    });
            }
        }
    }
    pub fn open_workspace(&mut self, path: std::path::PathBuf, now: f64) {
        match self.workspace.open(path) {
            Ok(()) => {
                if let Some(root) = self.workspace.root.clone() {
                    let expanded_key = format!("workspace_expanded:{}", root.to_string_lossy());
                    if let Some(saved) = crate::services::db_worker::get_stored_setting(&expanded_key) {
                        if let Ok(paths) = serde_json::from_str::<Vec<String>>(&saved) {
                            for path in paths.into_iter().map(std::path::PathBuf::from) {
                                if path.is_dir() && path.starts_with(&root) {
                                    self.workspace.expanded.insert(path);
                                }
                            }
                            let _ = self.workspace.refresh();
                        }
                    }
                    let root_str = root.to_string_lossy().into_owned();
                    let _ =
                        self.services
                            .db_tx
                            .send(crate::services::db_worker::DbMsg::SaveSetting {
                                key: "workspace_root".into(),
                                val: root_str.clone(),
                            });

                    // Track recent workspaces (MRU order, max 10, without duplicating)
                    let mut recents: Vec<String> = crate::services::db_worker::get_stored_setting("recent_workspaces")
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();
                    recents.retain(|p| p != &root_str);
                    recents.insert(0, root_str);
                    recents.truncate(10);
                    if let Ok(json) = serde_json::to_string(&recents) {
                        let _ = self.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                            key: "recent_workspaces".into(),
                            val: json,
                        });
                    }
                }
                if let (Some(backend), Some(root)) = (
                    self.services.vim_runtime.backend.as_mut(),
                    self.workspace.root.as_deref(),
                ) {
                    let _ = backend.set_workspace_root(root);
                }
                self.set_status("Workspace opened", now);
            }
            Err(e) => self.set_status(format!("Could not open workspace: {e}"), now),
        }
    }

    /// Closes the active workspace and returns to the welcome page when no tabs are open.
    pub fn close_workspace(&mut self, now: f64) {
        self.workspace.root = None;
        self.workspace.entries.clear();
        self.workspace.expanded.clear();
        self.workspace.selected_items.clear();
        self.workspace.selection_anchor = None;
        self.workspace.dialog = None;
        self.workspace.dialog_name.clear();
        self.workspace.dialog_error = None;
        let _ = self
            .services
            .db_tx
            .send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "workspace_root".into(),
                val: String::new(),
            });
        if let Some(backend) = self.services.vim_runtime.backend.as_mut() {
            let _ = backend.set_workspace_root(std::path::Path::new(""));
        }
        if self.open_notes.is_empty() {
            self.misc.show_welcome = true;
        }
        self.set_status("Workspace closed", now);
    }

    /// Removes a workspace path from the recent list WITHOUT deleting any files on disk.
    pub fn remove_recent_workspace(&mut self, path: &std::path::Path) {
        let path_str = path.to_string_lossy();
        let mut recents: Vec<String> = crate::services::db_worker::get_stored_setting("recent_workspaces")
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        recents.retain(|p| p.as_str() != path_str);
        if let Ok(json) = serde_json::to_string(&recents) {
            let _ = self.services.db_tx.send(crate::services::db_worker::DbMsg::SaveSetting {
                key: "recent_workspaces".into(),
                val: json,
            });
        }
    }

    /// Returns the stored list of recent workspace paths.
    pub fn get_recent_workspaces(&self) -> Vec<std::path::PathBuf> {
        crate::services::db_worker::get_stored_setting("recent_workspaces")
            .and_then(|s| serde_json::from_str::<Vec<String>>(&s).ok())
            .map(|list| list.into_iter().map(std::path::PathBuf::from).collect())
            .unwrap_or_default()
    }

    /// Navigates to or creates a note/document target referenced by a Wikilink `[[target]]`.
    /// Resolves against workspace files on disk first, then database notes.
    pub fn follow_wikilink(&mut self, target: &str, now: f64) {
        let clean = target.trim();
        if clean.is_empty() {
            return;
        }

        // 1. Check workspace entries if a project workspace is active
        if let Some(root) = self.workspace.root.as_ref() {
            let direct_file = root.join(clean);
            if direct_file.is_file() {
                self.open_file_path(direct_file, now);
                return;
            }
            let md_file = root.join(format!("{}.md", clean));
            if md_file.is_file() {
                self.open_file_path(md_file, now);
                return;
            }

            // Search by stem or filename in workspace entries
            let clean_lower = clean.to_ascii_lowercase();
            if let Some(entry) = self.workspace.entries.iter().find(|e| {
                !e.directory && (
                    e.path.file_stem().is_some_and(|s| s.to_string_lossy().to_ascii_lowercase() == clean_lower)
                    || e.path.file_name().is_some_and(|n| n.to_string_lossy().to_ascii_lowercase() == clean_lower)
                )
            }) {
                let path = entry.path.clone();
                self.open_file_path(path, now);
                return;
            }

            // If not found in workspace and not in notes, create file in workspace root
            let in_notes = crate::wikilink::resolve_wikilink(clean, &self.notes.notes_list);
            if in_notes.is_none() {
                let new_path = root.join(format!("{}.md", clean));
                let initial = format!("# {}\n\n", clean);
                if std::fs::write(&new_path, initial).is_ok() {
                    let _ = self.workspace.refresh();
                    self.open_file_path(new_path, now);
                    return;
                }
            }
        }

        // 2. Fall back to existing database notes
        if let Some(note) = crate::wikilink::resolve_wikilink(clean, &self.notes.notes_list) {
            self.open_note_by_id(note.id, now);
        } else {
            self.create_new_note(now);
            crate::notes::rename_active_note(self, clean, now);
        }
    }

    pub fn toggle_workspace_folder(&mut self,path:&std::path::Path){
        self.workspace.toggle(path);
        self.persist_workspace_expansion();
    }

    pub fn persist_scratch_tab(&mut self, index: usize, now: f64) -> std::io::Result<()> {
        let tab = self
            .open_notes
            .get(index)
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No scratch tab"))?;
        if tab.file_path.is_some() || tab.id > 0 {
            return Ok(());
        }
        let is_active = index == self.tabs.active_tab;
        let content = if is_active {
            self.editor.ed.text()
        } else {
            tab.editor.text()
        };
        let title = if tab.title.trim().is_empty() {
            "Untitled Note"
        } else {
            tab.title.as_str()
        };
        let dir = if let Some(root) = &self.workspace.root {
            root.clone()
        } else {
            let d = crate::workspace::default_workspace_dir().join("Scratch");
            std::fs::create_dir_all(&d)?;
            d
        };
        let safe: String = title
            .chars()
            .map(|c| {
                if "<>:\"/\\|?*".contains(c) || c.is_control() {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let stem = if safe.trim().is_empty() {
            "Untitled Note"
        } else {
            safe.trim()
        };
        let has_ext = std::path::Path::new(stem).extension().is_some();
        let mut path = if has_ext {
            dir.join(stem)
        } else {
            dir.join(format!("{stem}.md"))
        };
        let mut suffix = 2;
        while path.exists() {
            path = if has_ext {
                let file_stem = std::path::Path::new(stem)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(stem);
                let ext = std::path::Path::new(stem)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                dir.join(format!("{file_stem}_{suffix}.{ext}"))
            } else {
                dir.join(format!("{stem} {suffix}.md"))
            };
            suffix += 1;
        }
        {
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)?;
            file.write_all(content.as_bytes())?;
        }
        let path = crate::workspace::normalized_path(&path);
        let display_title = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if let Some(tab) = self.open_notes.get_mut(index) {
            tab.file_path = Some(path.clone());
            tab.title = display_title.clone();
            tab.is_dirty = false;
        }
        if is_active {
            self.notes.active_note_title = display_title;
            self.editor.is_dirty = false;
            self.editor.last_saved_time = now;
        }
        self.file_versions.insert(
            path.clone(),
            crate::workspace::file_fingerprint(&path).unwrap_or(0),
        );
        if self.workspace.root.is_none() {
            self.open_workspace(crate::workspace::default_workspace_dir(), now);
        } else {
            let _ = self.workspace.refresh();
        }
        self.sync_active_tab();
        self.save_open_tabs();
        Ok(())
    }
    /// Computes the exact monospace character advance width and line height.
    /// Measuring 100 characters eliminates single-glyph bounding box ink discrepancies,
    /// guaranteeing that caret placement at `col * cw` aligns with rendered text at any line length.
    pub fn cell_size(&mut self, ctx: &egui::Context) -> (f32, f32) {
        *self.editor.cell.get_or_insert_with(|| {
            let font = FontId::monospace(self.misc.font_size);
            let sample_100 = "MMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMMM";
            let g100 = ctx.fonts(|f| f.layout_no_wrap(sample_100.to_owned(), font.clone(), Color32::WHITE));
            let g1 = ctx.fonts(|f| f.layout_no_wrap("M".to_owned(), font, Color32::WHITE));
            let cw = (g100.size().x - g1.size().x) / 99.0;
            let lh = (g1.size().y * 1.30).round();
            (cw, lh)
        })
    }

    pub fn active_language(&self) -> crate::language::FileLanguage {
        self.open_notes
            .get(self.tabs.active_tab)
            .map(|tab| {
                tab.language_override
                    .unwrap_or_else(|| match tab.file_path.as_deref() {
                        Some(path) => crate::language::FileLanguage::from_path(path),
                        None => {
                            crate::language::FileLanguage::from_title(&self.notes.active_note_title)
                                .unwrap_or(crate::language::FileLanguage::Markdown)
                        }
                    })
            })
            .unwrap_or(crate::language::FileLanguage::Markdown)
    }

    /// Buffer name handed to Neovim so its filetype detection matches the document.
    pub fn active_buffer_name(&self) -> String {
        self.open_notes
            .get(self.tabs.active_tab)
            .and_then(|tab| tab.file_path.as_deref())
            .map(|path| {
                crate::workspace::normalized_path(path)
                    .to_string_lossy()
                    .into_owned()
            })
            .unwrap_or_else(|| {
                let title = self.notes.active_note_title.trim();
                if crate::language::FileLanguage::from_title(title).is_some() {
                    title.to_owned()
                } else {
                    "note.md".to_owned()
                }
            })
    }

    pub fn set_language_override(
        &mut self,
        override_language: Option<crate::language::FileLanguage>,
        now: f64,
    ) {
        if let Some(tab) = self.open_notes.get_mut(self.tabs.active_tab) {
            tab.language_override = override_language;
            let language = self.active_language();
            self.set_status(&format!("Language: {}", language.label()), now);
        }
    }

    pub fn open_file_path(&mut self, path: std::path::PathBuf, now: f64) {
        let path = crate::workspace::normalized_path(&path);
        if self.workspace.reveal(&path) {
            self.persist_workspace_expansion();
        }
        if let Some(index) = self.open_notes.iter().position(|tab| {
            tab.file_path
                .as_ref()
                .is_some_and(|p| crate::workspace::same_path(p, &path))
        }) {
            self.switch_tab(index, now);
            self.sidebar.focused=false;
            return;
        }
        if let Ok(meta) = std::fs::metadata(&path) {
            if meta.len() > 30 * 1024 * 1024 {
                self.set_status(format!("File too large to open ({} MB, limit is 30MB)", meta.len() / (1024 * 1024)), now);
                return;
            }
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content.replace("\r\n", "\n").replace('\r', "\n"),
            Err(error) => {
                self.set_status(format!("Could not open {}: {error}", path.display()), now);
                return;
            }
        };
        if let Some(current) = self.open_notes.get_mut(self.tabs.active_tab) {
            current.editor = self.editor.ed.clone();
            current.title = self.notes.active_note_title.clone();
            current.scroll_y = self.editor.scroll_y;
            current.is_dirty = self.editor.is_dirty;
        }
        if self
            .open_notes
            .get(self.tabs.active_tab)
            .is_some_and(|tab| tab.file_path.is_none() && tab.id == 0 && tab.is_dirty)
        {
            let _ = self.persist_scratch_tab(self.tabs.active_tab, now);
        }
        let title = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Untitled")
            .to_string();
        self.file_versions.insert(
            path.clone(),
            crate::workspace::file_fingerprint(&path).unwrap_or(0),
        );
        let mut editor = Editor::new();
        editor.set_text(&content);
        editor.clear_history();
        self.notes.active_note_id = None;
        self.notes.active_note_title = title.clone();
        self.editor.ed = editor.clone();
        self.editor.scroll_y = 0.0;
        self.editor.is_dirty = false;
        self.misc.show_welcome = false;
        self.misc.mode = Mode::Normal;
        self.open_notes.push(OpenNote {
            id: 0,
            title: title.clone(),
            editor,
            scroll_y: 0.0,
            is_dirty: false,
            file_path: Some(path.clone()),
            language_override: None,
        });
        self.tabs.active_tab = self.open_notes.len() - 1;
        self.tabs.last_active_tab = self.tabs.active_tab;
        self.sidebar.focused=false;
        self.misc.caret.gliding=false;
        self.save_active_note_id();
        self.save_open_tabs();
        self.set_status(format!("Opened {}", self.display_file_path(&path)), now);
    }

    pub fn perform_workspace_dialog(&mut self, now: f64) {
        let kind = self.workspace.dialog;
        let dialog_parent=self.workspace.dialog_parent.clone();
        let old = self.workspace.selected_path.clone();
        if kind == Some(crate::workspace::WorkspaceDialog::Delete) {
            if let Some(path) = old.as_ref() {
                let busy = self
                    .open_notes
                    .iter()
                    .filter_map(|t| t.file_path.as_ref())
                    .any(|p| p.starts_with(path));
                if busy {
                    self.set_status("Close files in this item before deleting it", now);
                    return;
                }
            }
        }
        let created_name = self.workspace.dialog_name.trim().to_string();
        let new = if kind == Some(crate::workspace::WorkspaceDialog::Rename) {
            old.as_ref()
                .map(|p| p.with_file_name(self.workspace.dialog_name.trim()))
        } else {
            None
        };
        match self.workspace.perform() {
            Ok(()) => {
                if matches!(kind,Some(crate::workspace::WorkspaceDialog::CreateFile|crate::workspace::WorkspaceDialog::CreateFolder)) {
                    self.workspace.expanded.insert(dialog_parent.clone());
                    let _=self.workspace.refresh();
                    self.persist_workspace_expansion();
                    if kind == Some(crate::workspace::WorkspaceDialog::CreateFile) {
                        let created_file = dialog_parent.join(&created_name);
                        if created_file.is_file() {
                            self.open_file_path(created_file, now);
                        }
                    }
                }
                if kind == Some(crate::workspace::WorkspaceDialog::Delete) {
                    self.reload_db_state();
                }
                if let (Some(old), Some(new)) = (old, new) {
                    self.remap_workspace_prefix(&old, &new);
                    let versions: Vec<_> = self
                        .file_versions
                        .iter()
                        .map(|(p, v)| (p.clone(), *v))
                        .collect();
                    for (path, version) in versions {
                        if path.starts_with(&old) {
                            if let Ok(suffix) = path.strip_prefix(&old) {
                                self.file_versions.remove(&path);
                                self.file_versions.insert(new.join(suffix), version);
                            }
                        }
                    }
                    for tab in &mut self.open_notes {
                        if let Some(path) = tab.file_path.as_mut() {
                            let direct = *path == old;
                            if path.starts_with(&old) {
                                if let Ok(suffix) = path.strip_prefix(&old) {
                                    *path = new.join(suffix);
                                }
                            }
                            if direct {
                                tab.title = new
                                    .file_name()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();
                            }
                        }
                    }
                    let _ = self.workspace.refresh();
                    self.persist_workspace_expansion();
                    self.save_open_tabs();
                }
                self.set_status("Workspace updated", now);
            }
            Err(e) => {
                self.workspace.dialog_error=Some(e.to_string());
                self.set_status(format!("Filesystem operation failed: {e}"), now);
            }
        }
    }

    pub fn move_workspace_items(
        &mut self,
        sources: &[std::path::PathBuf],
        destination: &std::path::Path,
        now: f64,
    ) {
        let mut top = sources.to_vec();
        top.sort_by_key(|p| p.components().count());
        top.dedup();
        top.retain(|candidate| {
            !sources
                .iter()
                .any(|parent| parent != candidate && candidate.starts_with(parent))
        });
        let targets: Vec<_> = top
            .iter()
            .map(|p| {
                (
                    p.clone(),
                    destination.join(p.file_name().unwrap_or_default()),
                )
            })
            .collect();
        match crate::workspace::move_paths(&top, destination) {
            Ok(()) => {
                for tab in &mut self.open_notes {
                    if let Some(path) = tab.file_path.as_mut() {
                        for (from, to) in &targets {
                            if path.starts_with(from) {
                                if let Ok(suffix) = path.strip_prefix(from) {
                                    *path = to.join(suffix);
                                }
                                if path == to {
                                    tab.title = to
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .into_owned();
                                }
                                break;
                            }
                        }
                    }
                }
                for (from,to) in &targets {self.remap_workspace_prefix(from,to);}
                let versions: Vec<_> = self
                    .file_versions
                    .iter()
                    .map(|(p, v)| (p.clone(), *v))
                    .collect();
                for (from, to) in &targets {
                    for (path, version) in &versions {
                        if path.starts_with(from) {
                            if let Ok(suffix) = path.strip_prefix(from) {
                                self.file_versions.remove(path);
                                self.file_versions.insert(to.join(suffix), *version);
                            }
                        }
                    }
                }
                let _ = self.workspace.refresh();
                self.workspace.rebuild_search_index();
                self.persist_workspace_expansion();
                self.save_open_tabs();
                self.set_status("Moved selected filesystem items", now);
            }
            Err(error) => self.set_status(format!("Move failed: {error}"), now),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        if self.misc.first_frame {
            self.misc.first_frame = false;
            crate::services::blur::apply_window_blur(self.misc.blur_effect);
            if let Some(cmd) = egui::ViewportCommand::center_on_screen(ctx) {
                ctx.send_viewport_cmd(cmd);
            }
        }

        let now = ctx.input(|i| i.time);
        if self.workspace.poll_external(now) {
            self.apply_external_workspace_renames();
            if self.modal.search_open { self.update_search_results(); }
            ctx.request_repaint();
        }
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.05);
        let mode_at_frame_start = self.services.editor_controller.mode;
        crate::vim::runtime::update(self, ctx, now, mode_at_frame_start);

        // Precompute visual lines so keyboard navigation (ArrowUp, ArrowDown, PageUp, PageDown) uses accurate visual layout
        let (cw, _) = self.cell_size(ctx);
        let screen_w = ctx.screen_rect().width();
        let left_margin = if self.sidebar.open {
            crate::layout::GAP
                + self.sidebar.width
                + crate::layout::SPLITTER_BAR_W
                + crate::layout::GAP
        } else {
            crate::layout::GAP
        };
        let editor_w = (screen_w - left_margin - crate::layout::GAP).max(100.0);
        let is_preview_active = self.editor.preview_open && self.misc.mode == Mode::Normal;
        let effective_editor_w = if is_preview_active {
            let divider_w = 12.0;
            let available_w = (editor_w - divider_w).max(200.0);
            let min_w = 120.0f32;
            let max_w = (available_w - 120.0f32).max(min_w);
            (available_w * self.editor.split_ratio).clamp(min_w, max_w)
        } else {
            editor_w
        };
        let (ed_font_size, _, _) = self.misc.zoom.editor_metrics(self.misc.font_size, ctx);
        let active_ed = if self.misc.mode == Mode::Doc {
            &self.editor.doc_ed
        } else {
            &self.editor.ed
        };
        let in_vim = self.services.editor_controller.mode == EditorInputMode::Vim
            && matches!(self.misc.mode, Mode::Normal | Mode::Doc);
        let use_inline_markdown = self.editor.inline_mode
            && (self.misc.mode == Mode::Doc
                || self.active_language() == crate::language::FileLanguage::Markdown);
        self.editor.visual_lines = if in_vim {
            vec![crate::types::VisualLine {
                char_start: 0,
                char_end: active_ed.buf.len(),
            }]
        } else if use_inline_markdown {
            let gutter_w = if self.editor.show_line_numbers {
                let total_lines = (active_ed.buf.iter().filter(|&&c| c == '\n').count() + 1).max(1);
                let digits = total_lines.to_string().len().max(2);
                (digits as f32 * (ed_font_size * 0.55) + 14.0).max(28.0)
            } else {
                0.0
            };
            let pad_x = if self.editor.show_line_numbers {
                16.0
            } else {
                24.0
            };
            let safe_w = effective_editor_w;
            let effective_gutter_w = if safe_w > gutter_w + 40.0 {
                gutter_w
            } else {
                0.0
            };
            let wrap_w = (effective_editor_w - effective_gutter_w - pad_x - 24.0).max(120.0);
            let inline_layout = crate::view_editor::inline::compute_inline_layout_ctx(
                ctx,
                active_ed,
                wrap_w,
                ed_font_size,
                &self.misc.theme,
                0.0,
            );
            inline_layout.compute_visual_lines()
        } else {
            let gutter_space = if self.editor.show_line_numbers {
                42.0
            } else {
                0.0
            };
            let text_area_w = (effective_editor_w - gutter_space - 8.0).max(100.0);
            let max_cols = (text_area_w / cw).floor().max(15.0) as usize;
            if self.editor.cached_visual_cols == Some(max_cols)
                && self.editor.cached_buf_len == active_ed.buf.len()
                && !self.editor.visual_lines.is_empty()
            {
                std::mem::take(&mut self.editor.visual_lines)
            } else {
                self.editor.cached_visual_cols = Some(max_cols);
                self.editor.cached_buf_len = active_ed.buf.len();
                active_ed.compute_visual_lines(max_cols)
            }
        };

        if self.terminal.open || self.misc.mode == Mode::Terminal {
            if let Some(ref mut pane) = self.terminal.pane {
                if pane.pump() {
                    self.terminal.open = false;
                    self.terminal.focused = false;
                    if self.misc.mode == Mode::Terminal {
                        self.misc.mode = self.terminal.prev_mode_before_term;
                    }
                    self.set_status("Terminal session ended", now);
                }
            }
        }

        if self.editor.font_dirty {
            self.editor.font_dirty = false;
            crate::services::font_manager::apply_font(ctx, &self.misc.selected_font);
        }

        let typed = handle_input(self, ctx, now);

        // Activity tracking: if user interacted in the last 60s, count dt towards active editor time
        if (now - self.misc.last_char_time) < 60.0 {
            self.activity.pending_secs += dt;
        }
        if typed {
            self.activity.pending_keys += 1;
            // Track completed word if space or newline typed
            if let Some(&last_ch) = self.editor.ed.buf.get(self.editor.ed.cur.saturating_sub(1)) {
                if last_ch.is_whitespace() {
                    self.activity.pending_words += 1;
                }
            }
        }

        // Periodic auto-flush of activity stats to storage every 10 seconds
        if (now - self.activity.last_flush_time) > 10.0 {
            self.flush_activity(now);
            self.save_caret_position();
        }

        // Auto-save when idle for 1.2s in Normal mode
        let has_persistent_document = self.notes.active_note_id.is_some()
            || self
                .open_notes
                .get(self.tabs.active_tab)
                .is_some_and(|tab| tab.file_path.is_some());
        if self.editor.is_dirty
            && (now - self.misc.last_char_time) > 1.2
            && self.misc.mode == Mode::Normal
        {
            if has_persistent_document {
                self.quick_save_active_note(now);
            } else if self.persist_scratch_tab(self.tabs.active_tab, now).is_err() {
                self.set_status("Scratch autosave failed", now);
            }
            self.save_caret_position();
        }

        // Save session state immediately if window close is requested
        if ctx.input(|i| i.viewport().close_requested()) {
            self.sync_save_session();
        }

        // Check Webscan background worker channel
        if let Some(ref rx) = self.scan.scan_rx {
            if let Ok(msg) = rx.try_recv() {
                let target_url = self.scan.scan_in_progress.take().unwrap_or_default();
                self.scan.scan_rx = None;
                match msg {
                    Ok(scan_res) => {
                        // Persist scan result to filesystem scans.json
                        if let Ok(json_str) = serde_json::to_string(&scan_res.findings) {
                            crate::services::db_worker::save_stored_scan(
                                &scan_res.url,
                                scan_res.note.as_deref(),
                                &json_str,
                            );
                        }
                        self.scan.active_scan_result = Some(scan_res);
                        self.scan.active_scan_error = None;
                        self.scan.scan_report_scroll_y = 0.0;
                        self.misc.mode = Mode::ScanReport;
                        self.set_status("Scan completed successfully", now);
                    }
                    Err(err_msg) => {
                        self.scan.active_scan_result = None;
                        self.scan.active_scan_error = Some((target_url, err_msg));
                        self.scan.scan_report_scroll_y = 0.0;
                        self.misc.mode = Mode::ScanReport;
                        self.set_status("Scan failed (see report for details)", now);
                    }
                }
            }
        }

        // Open dropped folders and files as filesystem workspaces
        if crate::workspace_import::handle_drag_and_drop(ctx, &mut self.workspace) {
            if let Some(root) = self.workspace.root.clone() {
                self.open_workspace(root, now);
            }
        }

        window_shortcuts(ctx);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| {
                self.draw(ui, dt, now, typed);
            });
        // Defer recording transitions performed during this frame until the
        // next update, where the newly selected backend can reconcile state.
        self.services.editor_controller.last_observed_mode = mode_at_frame_start;

        // Repaint gating: animate caret at high rate; command/modal at ~60fps; idle at 100ms.
        // Do NOT call request_repaint() (unbounded) for command mode — it starves the Windows
        // message pump and causes "Not Responding" when `:` is typed.
        let focused = ctx.input(|i| i.focused);
        let caret_animating = self.misc.caret.is_animating(now);
        if focused && caret_animating {
            ctx.request_repaint_after(Duration::from_millis(8));
        } else if focused && typed {
            // Show typed input (e.g. the command line) on the very next frame.
            // 	yped is only true on frames that received input, so this does
            // not spin the event loop while idle.
            ctx.request_repaint();
        } else if focused
            && (self.command_bar.in_command
                || self
                    .services
                    .vim_runtime
                    .backend
                    .as_ref()
                    .is_some_and(|b| b.busy_label().is_some())
                || !self.misc.showcmd.text.is_empty()
                || self.modal.search_open
                || self.modal.settings_open
                || self.modal.help_open
                || self.modal.rename_open
                || self.modal.delete_confirm_open
                || self.misc.accent_dropdown_open)
        {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_save_session();
    }
}
