//! Recursive filesystem scanner filtering strictly for Markdown and text documents.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub size: u64,
    pub relative_title: String,
}

/// Recursively scans paths (directories and files), extracting `.md`, `.markdown`, and `.txt` files.
/// Silently ignores all non-text formats (PDF, images, binaries, etc.) and hidden internal folders
/// such as `.obsidian`, `.git`, and `.trash`.
pub fn scan_workspace_paths(
    paths: &[PathBuf],
    cancel_token: &Arc<AtomicBool>,
) -> (Vec<ScannedFile>, u64) {
    let mut files = Vec::new();
    let mut total_bytes = 0u64;

    for path in paths {
        if cancel_token.load(Ordering::Relaxed) {
            break;
        }
        collect_recursive(path, cancel_token, &mut files, &mut total_bytes);
    }

    (files, total_bytes)
}

fn collect_recursive(
    path: &Path,
    cancel_token: &Arc<AtomicBool>,
    out: &mut Vec<ScannedFile>,
    total_bytes: &mut u64,
) {
    if cancel_token.load(Ordering::Relaxed) {
        return;
    }

    // Skip hidden folders and files (e.g. .obsidian, .git, .trash)
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if file_name.starts_with('.') && file_name != "." {
            return;
        }
    }

    if path.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                if cancel_token.load(Ordering::Relaxed) {
                    return;
                }
                // Avoid infinite loops or cycle recursion on directory symlinks
                if entry.file_type().map_or(false, |ft| ft.is_symlink()) {
                    continue;
                }
                collect_recursive(&entry.path(), cancel_token, out, total_bytes);
            }
        }
    } else if path.is_file() {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            // Accept ONLY .md, .markdown, and .txt files.
            // All other files (e.g. .pdf, .png, .jpg, .canvas, etc.) are silently ignored.
            if ext_lower == "md" || ext_lower == "markdown" || ext_lower == "txt" {
                let size = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled Note")
                    .to_string();

                *total_bytes += size;
                out.push(ScannedFile {
                    path: path.to_path_buf(),
                    size,
                    relative_title: title,
                });
            }
        }
    }
}
