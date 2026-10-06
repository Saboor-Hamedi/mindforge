//! Filesystem-backed workspace tree and safe file operations.
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    pub path: PathBuf,
    pub depth: usize,
    pub directory: bool,
}

#[derive(Default)]
pub struct WorkspaceState {
    pub root: Option<PathBuf>,
    pub expanded: HashSet<PathBuf>,
    pub entries: Vec<Entry>,
    pub search_entries: Vec<Entry>,
    search_rx: Option<std::sync::mpsc::Receiver<Vec<Entry>>>,
    snapshot: Vec<(PathBuf, bool, u64, Option<std::time::SystemTime>)>,
    external_renames: Vec<(PathBuf, PathBuf)>,
    poll_at: f64,
    pub dialog: Option<WorkspaceDialog>,
    pub dialog_parent: PathBuf,
    pub dialog_name: String,
    pub selected_path: Option<PathBuf>,
    pub selected_items: HashSet<PathBuf>,
    pub selection_anchor: Option<PathBuf>,
    pub dialog_error: Option<String>,
    pub dialog_focus_requested: bool,
    pub context_menu: Option<crate::ui::menu::MenuState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceDialog {
    CreateFile,
    CreateFolder,
    Rename,
    Delete,
}

impl WorkspaceState {
    pub fn open(&mut self, root: PathBuf) -> std::io::Result<()> {
        let root = normalized_path(&fs::canonicalize(root)?);
        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Workspace must be a directory",
            ));
        }
        self.root = Some(root.clone());
        self.expanded.clear();
        self.expanded.insert(root);
        self.selected_items.clear();
        self.selection_anchor = None;
        self.search_entries.clear();
        self.external_renames.clear();
        self.dialog = None;
        self.dialog_error = None;
        self.dialog_focus_requested = false;
        self.refresh()?;
        self.rebuild_search_index();
        Ok(())
    }
    pub fn refresh(&mut self) -> std::io::Result<()> {
        self.entries.clear();
        if let Some(root) = self.root.clone() {
            list_dir(&root, 0, &self.expanded, &mut self.entries)?;
        }
        self.snapshot = snapshot(self.root.as_deref(), &self.expanded);
        self.selected_items.retain(|path| path.exists());
        if self
            .selection_anchor
            .as_ref()
            .is_some_and(|path| !path.exists())
        {
            self.selection_anchor = None;
        }
        Ok(())
    }
    pub fn poll_external(&mut self, now: f64) -> bool {
        let mut changed = false;
        if let Some(receiver) = self.search_rx.as_ref() {
            match receiver.try_recv() {
                Ok(entries) => {
                    self.search_entries = entries;
                    self.search_rx = None;
                    changed = true;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.search_rx = None,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        if now - self.poll_at < 0.75 {
            return changed;
        }
        self.poll_at = now;
        let next = snapshot(self.root.as_deref(), &self.expanded);
        if next != self.snapshot {
            let removed: Vec<_> = self
                .snapshot
                .iter()
                .filter(|old| !next.iter().any(|new| new.0 == old.0))
                .collect();
            let added: Vec<_> = next
                .iter()
                .filter(|new| !self.snapshot.iter().any(|old| old.0 == new.0))
                .collect();
            self.external_renames.clear();
            for old in &removed {
                let old_key = (old.1, old.2, old.3);
                let old_count = removed
                    .iter()
                    .filter(|item| (item.1, item.2, item.3) == old_key)
                    .count();
                let matches: Vec<_> = added
                    .iter()
                    .filter(|item| (item.1, item.2, item.3) == old_key)
                    .collect();
                if old_count == 1 && matches.len() == 1 {
                    self.external_renames
                        .push((old.0.clone(), matches[0].0.clone()));
                }
            }
            let _ = self.refresh();
            self.rebuild_search_index();
            return true;
        }
        changed
    }
    pub fn rebuild_search_index(&mut self) {
        let Some(root) = self.root.clone() else {
            self.search_entries.clear();
            self.search_rx = None;
            return;
        };
        let (tx, rx) = std::sync::mpsc::channel();
        self.search_rx = Some(rx);
        std::thread::spawn(move || {
            let mut entries = Vec::new();
            if list_all(&root, 0, &mut entries).is_ok() {
                let _ = tx.send(entries);
            }
        });
    }
    pub fn take_external_renames(&mut self) -> Vec<(PathBuf, PathBuf)> {
        std::mem::take(&mut self.external_renames)
    }
    pub fn toggle(&mut self, path: &Path) {
        if !self.expanded.remove(path) {
            self.expanded.insert(path.to_path_buf());
        }
        let _ = self.refresh();
        self.rebuild_search_index();
    }
    pub fn reveal(&mut self, path: &Path) -> bool {
        let Some(root) = self.root.as_deref() else {
            return false;
        };
        if !path.starts_with(root) {
            return false;
        }
        let mut changed = false;
        let mut current = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(root)
        };
        while current.starts_with(root) {
            changed |= self.expanded.insert(current.to_path_buf());
            if current == root {
                break;
            }
            let Some(parent) = current.parent() else {
                break;
            };
            current = parent;
        }
        if self.selected_items.len() != 1 || !self.selected_items.contains(path) {
            self.selected_items.clear();
            self.selected_items.insert(path.to_path_buf());
            changed = true;
        }
        if self.selection_anchor.as_deref() != Some(path) {
            self.selection_anchor = Some(path.to_path_buf());
            changed = true;
        }
        if changed {
            let _ = self.refresh();
        }
        changed
    }
    pub fn perform(&mut self) -> std::io::Result<()> {
        let name = self.dialog_name.trim();
        match self.dialog.ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "No pending operation")
        })? {
            WorkspaceDialog::CreateFile => {
                validate_name(name)?;
                create_file(&self.dialog_parent.join(name))?;
            }
            WorkspaceDialog::CreateFolder => {
                validate_name(name)?;
                create_dir(&self.dialog_parent.join(name))?;
            }
            WorkspaceDialog::Rename => {
                validate_name(name)?;
                let from = self.selected_path.as_ref().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "No selected item")
                })?;
                rename_path(from, &from.with_file_name(name))?;
            }
            WorkspaceDialog::Delete => {
                let path = self.selected_path.as_ref().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::NotFound, "No selected item")
                })?;
                delete_path(path)?;
            }
        }
        self.dialog = None;
        self.dialog_name.clear();
        self.dialog_error = None;
        self.dialog_focus_requested = false;
        self.selected_path = None;
        self.refresh()?;
        self.rebuild_search_index();
        Ok(())
    }
}

fn validate_name(name: &str) -> std::io::Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.contains(['/', '\\', '\0'])
        || name.chars().any(char::is_control)
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Enter a valid single file or folder name",
        ));
    }
    #[cfg(windows)]
    if name.ends_with([' ', '.']) || name.chars().any(|c| "<>:\"|?*".contains(c)) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Name is not valid on Windows",
        ));
    }
    Ok(())
}

fn list_dir(
    dir: &Path,
    depth: usize,
    expanded: &HashSet<PathBuf>,
    out: &mut Vec<Entry>,
) -> std::io::Result<()> {
    let mut children: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    children.sort_by_key(|e| {
        (
            !e.path().is_dir(),
            e.file_name().to_string_lossy().to_lowercase(),
        )
    });
    for item in children {
        let path = item.path();
        let ty = item.file_type()?;
        if ty.is_symlink() {
            continue;
        }
        let directory = ty.is_dir();
        out.push(Entry {
            path: path.clone(),
            depth,
            directory,
        });
        if directory && expanded.contains(&path) {
            list_dir(&path, depth + 1, expanded, out)?;
        }
    }
    Ok(())
}
fn list_all(dir: &Path, depth: usize, out: &mut Vec<Entry>) -> std::io::Result<()> {
    let mut children: Vec<_> = fs::read_dir(dir)?.filter_map(Result::ok).collect();
    children.sort_by_key(|e| {
        (
            !e.path().is_dir(),
            e.file_name().to_string_lossy().to_lowercase(),
        )
    });
    for item in children {
        let ty = item.file_type()?;
        if ty.is_symlink() {
            continue;
        }
        let path = item.path();
        let directory = ty.is_dir();
        out.push(Entry {
            path: path.clone(),
            depth,
            directory,
        });
        if directory {
            list_all(&path, depth + 1, out)?;
        }
    }
    Ok(())
}
fn snapshot(
    root: Option<&Path>,
    expanded: &HashSet<PathBuf>,
) -> Vec<(PathBuf, bool, u64, Option<std::time::SystemTime>)> {
    let mut result = Vec::new();
    fn walk(
        path: &Path,
        expanded: &HashSet<PathBuf>,
        out: &mut Vec<(PathBuf, bool, u64, Option<std::time::SystemTime>)>,
    ) {
        if let Ok(rd) = fs::read_dir(path) {
            for e in rd.flatten() {
                let p = e.path();
                if let Ok(t) = e.file_type() {
                    if t.is_symlink() {
                        continue;
                    }
                    let m = e.metadata().ok();
                    out.push((
                        p.clone(),
                        t.is_dir(),
                        m.as_ref().map_or(0, |m| m.len()),
                        m.and_then(|m| m.modified().ok()),
                    ));
                    if t.is_dir() && expanded.contains(&p) {
                        walk(&p, expanded, out)
                    }
                }
            }
        }
    }
    if let Some(root) = root {
        walk(root, expanded, &mut result);
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

pub fn normalized_path(path: &Path) -> PathBuf {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    #[cfg(windows)]
    {
        let value = canonical.to_string_lossy();
        if let Some(rest) = value.strip_prefix("\\\\?\\UNC\\") {
            PathBuf::from(format!("\\\\{rest}"))
        } else if let Some(rest) = value.strip_prefix("\\\\?\\") {
            PathBuf::from(rest)
        } else {
            canonical
        }
    }
    #[cfg(not(windows))]
    {
        canonical
    }
}
pub fn same_path(left: &Path, right: &Path) -> bool {
    let left = normalized_path(left);
    let right = normalized_path(right);
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}
pub fn relative_display(root: &Path, path: &Path) -> Option<String> {
    let root = normalized_path(root);
    let path = normalized_path(path);
    let roots: Vec<_> = root
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let parts: Vec<_> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.len() < roots.len() {
        return None;
    }
    #[cfg(windows)]
    let prefix_ok = roots
        .iter()
        .zip(&parts)
        .all(|(a, b)| a.eq_ignore_ascii_case(b));
    #[cfg(not(windows))]
    let prefix_ok = roots.iter().zip(&parts).all(|(a, b)| a == b);
    if !prefix_ok {
        return None;
    }
    Some(
        parts
            .into_iter()
            .skip(roots.len())
            .collect::<Vec<_>>()
            .join("/"),
    )
}
pub fn file_fingerprint(path: &Path) -> std::io::Result<u64> {
    use std::hash::{Hash, Hasher};
    let meta = fs::metadata(path)?;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    meta.len().hash(&mut h);
    if let Ok(mtime) = meta.modified() {
        mtime.hash(&mut h);
    }
    Ok(h.finish())
}
pub fn default_workspace_dir() -> PathBuf {
    directories::UserDirs::new()
        .and_then(|u| u.document_dir().map(Path::to_path_buf))
        .unwrap_or_else(|| {
            directories::ProjectDirs::from("com", "mindforge", "mindforge")
                .map(|p| p.data_local_dir().to_path_buf())
                .unwrap_or_else(std::env::temp_dir)
        })
        .join("MindForge")
}
pub fn create_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}
pub fn create_file(path: &Path) -> std::io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    use std::io::Write;
    f.write_all(b"")
}
pub fn rename_path(from: &Path, to: &Path) -> std::io::Result<()> {
    if to.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "Destination already exists",
        ));
    }
    fs::rename(from, to)
}
pub fn move_paths(sources: &[PathBuf], destination: &Path) -> std::io::Result<()> {
    if !destination.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Destination is not a directory",
        ));
    }
    let dest = normalized_path(destination);
    let mut unique: Vec<PathBuf> = sources.iter().cloned().collect();
    unique.sort_by_key(|p| p.components().count());
    unique.dedup();
    for src in &unique {
        if !src.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Missing source: {}", src.display()),
            ));
        }
        let source = normalized_path(src);
        if source == dest || (src.is_dir() && dest.starts_with(&source)) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Cannot move a directory into itself or its descendant",
            ));
        }
    }
    let mut targets = Vec::new();
    for src in unique.iter().filter(|candidate| {
        !unique
            .iter()
            .any(|parent| parent != *candidate && candidate.starts_with(parent))
    }) {
        let target = destination.join(src.file_name().unwrap_or_default());
        if target.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("Destination exists: {}", target.display()),
            ));
        }
        targets.push((src.clone(), target));
    }
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    for (src, to) in targets {
        if let Err(e) = fs::rename(&src, &to) {
            for (old, new) in moved.iter().rev() {
                let _ = fs::rename(new, old);
            }
            return Err(e);
        }
        moved.push((src, to));
    }
    Ok(())
}
pub fn delete_path(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
pub fn reveal_in_file_manager(path: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer.exe")
            .arg(format!("/select,{}", path.display()))
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn();
    }
    #[cfg(all(not(windows), not(target_os = "macos")))]
    {
        let _ = std::process::Command::new("xdg-open")
            .arg(path.parent().unwrap_or(path))
            .spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "mindforge-fs-{}-{}",
            std::process::id(),
            fastrand::u64(..)
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }
    #[test]
    fn tree_refresh_tracks_nested_real_files() {
        let root = fixture();
        let nested = root.join("src").join("components");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("App.tsx"), "hello").unwrap();
        let mut ws = WorkspaceState::default();
        ws.open(root.clone()).unwrap();
        assert_eq!(ws.entries.len(), 1);
        let src = ws.entries[0].path.clone();
        let nested = src.join("components");
        ws.toggle(&src);
        ws.toggle(&nested);
        assert_eq!(ws.entries.len(), 3);
        assert!(ws.entries.iter().any(|e| e.path.ends_with("App.tsx")));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn create_rename_move_and_delete_are_real_and_conflicts_are_safe() {
        let root = fixture();
        let src = root.join("src");
        fs::create_dir(&src).unwrap();
        let file = root.join("main.py");
        create_file(&file).unwrap();
        let target = src.join("main.py");
        fs::write(&target, "keep").unwrap();
        assert!(move_paths(&[file.clone()], &src).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "keep");
        fs::remove_file(&target).unwrap();
        move_paths(&[file.clone()], &src).unwrap();
        assert!(target.exists());
        let renamed = src.join("app.py");
        rename_path(&target, &renamed).unwrap();
        assert!(renamed.exists());
        delete_path(&src).unwrap();
        assert!(!src.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn prevents_directory_into_descendant() {
        let root = fixture();
        let parent = root.join("a");
        let child = parent.join("b");
        fs::create_dir_all(&child).unwrap();
        assert!(move_paths(&[parent.clone()], &child).is_err());
        assert!(parent.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn bulk_move_keeps_selected_parent_contents_without_moving_descendants_twice() {
        let root = fixture();
        let parent = root.join("components");
        let nested = parent.join("ui");
        let dest = root.join("src");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(&dest).unwrap();
        fs::write(nested.join("Button.tsx"), "button").unwrap();
        move_paths(&[parent.clone(), nested.clone()], &dest).unwrap();
        assert_eq!(
            fs::read_to_string(dest.join("components/ui/Button.tsx")).unwrap(),
            "button"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn external_rename_is_reported_for_open_tab_remapping() {
        let root = fixture();
        let old = root.join("before.md");
        let new = root.join("after.md");
        fs::write(&old, "same bytes").unwrap();
        let mut ws = WorkspaceState::default();
        ws.open(root.clone()).unwrap();
        fs::rename(&old, &new).unwrap();
        ws.poll_at = 0.0;
        assert!(ws.poll_external(1.0));
        assert_eq!(ws.take_external_renames(), vec![(old, new)]);
        fs::remove_dir_all(root).unwrap();
    }
}
