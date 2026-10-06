//! Context menu data models, target definitions, and predefined IDE menu specifications.

use std::path::{Path, PathBuf};

/// Target for which the context menu was triggered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuTarget {
    File(PathBuf),
    Folder(PathBuf),
    WorkspaceRoot(PathBuf),
    EmptyWorkspace,
}

impl MenuTarget {
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(p) | Self::Folder(p) | Self::WorkspaceRoot(p) => Some(p.as_path()),
            Self::EmptyWorkspace => None,
        }
    }

    pub fn is_dir(&self) -> bool {
        matches!(self, Self::Folder(_) | Self::WorkspaceRoot(_))
    }
}

/// Action produced when a context menu item is clicked or activated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    OpenFile(PathBuf),
    ToggleFolder(PathBuf),
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    CopyPath(PathBuf),
    Reveal(PathBuf),
    OpenWorkspace,
    RefreshWorkspace,
    CloseWorkspace,
}

/// A single line item within a MenuContainer (action or visual separator).
#[derive(Debug, Clone)]
pub enum MenuItem {
    Action {
        id: &'static str,
        label: String,
        icon: Option<&'static str>,
        shortcut: Option<&'static str>,
        action: MenuAction,
        destructive: bool,
        disabled: bool,
    },
    Separator,
}

impl MenuItem {
    pub fn action(
        id: &'static str,
        label: impl Into<String>,
        icon: Option<&'static str>,
        shortcut: Option<&'static str>,
        action: MenuAction,
    ) -> Self {
        Self::Action {
            id,
            label: label.into(),
            icon,
            shortcut,
            action,
            destructive: false,
            disabled: false,
        }
    }

    pub fn destructive(
        id: &'static str,
        label: impl Into<String>,
        icon: Option<&'static str>,
        shortcut: Option<&'static str>,
        action: MenuAction,
    ) -> Self {
        Self::Action {
            id,
            label: label.into(),
            icon,
            shortcut,
            action,
            destructive: true,
            disabled: false,
        }
    }
}

/// Builds the context menu items for a file.
pub fn file_menu(path: &Path) -> Vec<MenuItem> {
    let p = path.to_path_buf();
    let parent = p.parent().unwrap_or(path).to_path_buf();
    vec![
        MenuItem::action(
            "open",
            "Open",
            Some("📄"),
            Some("Enter"),
            MenuAction::OpenFile(p.clone()),
        ),
        MenuItem::action(
            "new_file",
            "New File",
            Some("📄+"),
            None,
            MenuAction::NewFile(parent.clone()),
        ),
        MenuItem::action(
            "new_folder",
            "New Folder",
            Some("📁+"),
            None,
            MenuAction::NewFolder(parent),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "rename",
            "Rename",
            Some("✏"),
            Some("F2"),
            MenuAction::Rename(p.clone()),
        ),
        MenuItem::destructive(
            "delete",
            "Delete",
            Some("🗑"),
            Some("Del"),
            MenuAction::Delete(p.clone()),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "copy_path",
            "Copy Path",
            Some("📋"),
            None,
            MenuAction::CopyPath(p.clone()),
        ),
        MenuItem::action(
            "reveal",
            "Reveal in File Explorer",
            Some("↗"),
            None,
            MenuAction::Reveal(p),
        ),
    ]
}

/// Builds the context menu items for a folder.
pub fn folder_menu(path: &Path, is_expanded: bool) -> Vec<MenuItem> {
    let p = path.to_path_buf();
    let toggle_label = if is_expanded { "Collapse" } else { "Expand" };
    let toggle_icon = if is_expanded { "▾" } else { "▸" };

    vec![
        MenuItem::action(
            "toggle",
            toggle_label,
            Some(toggle_icon),
            Some("Enter"),
            MenuAction::ToggleFolder(p.clone()),
        ),
        MenuItem::action(
            "new_file",
            "New File",
            Some("📄+"),
            None,
            MenuAction::NewFile(p.clone()),
        ),
        MenuItem::action(
            "new_folder",
            "New Folder",
            Some("📁+"),
            None,
            MenuAction::NewFolder(p.clone()),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "rename",
            "Rename",
            Some("✏"),
            Some("F2"),
            MenuAction::Rename(p.clone()),
        ),
        MenuItem::destructive(
            "delete",
            "Delete",
            Some("🗑"),
            Some("Del"),
            MenuAction::Delete(p.clone()),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "copy_path",
            "Copy Path",
            Some("📋"),
            None,
            MenuAction::CopyPath(p.clone()),
        ),
        MenuItem::action(
            "reveal",
            "Reveal in File Explorer",
            Some("↗"),
            None,
            MenuAction::Reveal(p),
        ),
    ]
}

/// Builds context menu items for the root of a workspace folder.
pub fn root_menu(root: &Path) -> Vec<MenuItem> {
    let p = root.to_path_buf();
    vec![
        MenuItem::action(
            "new_file",
            "New File",
            Some("📄+"),
            None,
            MenuAction::NewFile(p.clone()),
        ),
        MenuItem::action(
            "new_folder",
            "New Folder",
            Some("📁+"),
            None,
            MenuAction::NewFolder(p.clone()),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "refresh",
            "Refresh Explorer",
            Some("↻"),
            None,
            MenuAction::RefreshWorkspace,
        ),
        MenuItem::Separator,
        MenuItem::action(
            "copy_path",
            "Copy Path",
            Some("📋"),
            None,
            MenuAction::CopyPath(p.clone()),
        ),
        MenuItem::action(
            "reveal",
            "Reveal in File Explorer",
            Some("↗"),
            None,
            MenuAction::Reveal(p),
        ),
        MenuItem::Separator,
        MenuItem::action(
            "close_workspace",
            "Close Workspace",
            Some("✖"),
            None,
            MenuAction::CloseWorkspace,
        ),
    ]
}
