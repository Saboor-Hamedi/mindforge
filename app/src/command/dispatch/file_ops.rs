//! File and document lifecycle commands (:w, :r, :d, :clear, :backup, :export, :import, :edit, :new).

use crate::app::{App, EditorInputMode};
use crate::mode::Mode;

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "w" | "write" | "save" => {
            if app.misc.mode == Mode::Doc {
                app.set_status(
                    "Documentation files are read-only (changes not saved).",
                    now,
                );
                return true;
            }
            app.quick_save_active_note(now);
            true
        }
        "r" | "rename" => {
            if app.misc.mode == Mode::Doc {
                app.set_status(
                    "Documentation files are read-only and cannot be renamed.",
                    now,
                );
                return true;
            }
            if !args.is_empty() {
                app.rename_active_note(args, now);
            } else {
                app.modal.rename_open = true;
                app.modal.rename_input = app.notes.active_note_title.clone();
                app.modal.rename_just_opened = true;
            }
            true
        }
        "d" if app.services.editor_controller.mode == EditorInputMode::Vim => {
            // In Vim mode, `:d` is line deletion — let it fall through to Neovim MessagePack-RPC
            false
        }
        "d" | "delete" | "rm" => {
            if app.misc.mode == Mode::Doc {
                app.set_status("Documentation files cannot be deleted.", now);
                return true;
            }
            app.modal.delete_confirm_open = true;
            app.modal.delete_just_opened = true;
            true
        }
        "clear" | "cls" => {
            app.editor.ed.clear();
            app.editor.is_dirty = true;
            app.editor.scroll_y = 0.0;
            app.set_status("Editor cleared", now);
            true
        }
        "backup" | "snapshot" => {
            if !args.is_empty() {
                app.modal.backup_dir = args.to_string();
            }
            app.trigger_backup(now);
            true
        }
        "export" => {
            handle_export(app, args, now);
            true
        }
        "import" => {
            handle_import(app, args, now);
            true
        }
        "edit" | "editor" | "note" | "notes" | "e" => {
            let clean = args.trim_matches(|c| c == '"' || c == '\'').trim();
            if !clean.is_empty() {
                let target_path = if let Some(root) = &app.workspace.root {
                    root.join(clean)
                } else {
                    crate::workspace::default_workspace_dir().join(clean)
                };
                if !target_path.exists() {
                    if let Some(parent) = target_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&target_path, "");
                    let _ = app.workspace.refresh();
                }
                app.open_file_path(target_path, now);
            } else {
                app.misc.mode = Mode::Normal;
                app.set_status("Switched to Notes Editor", now);
            }
            true
        }
        "open" | "o" => {
            let path = if args.trim().is_empty() {
                rfd::FileDialog::new().pick_file()
            } else {
                Some(std::path::PathBuf::from(
                    args.trim_matches(|c| c == '"' || c == '\''),
                ))
            };
            if let Some(path) = path {
                app.open_file_path(path, now);
            }
            true
        }
        "new" | "n" => {
            let clean = args.trim_matches(|c| c == '"' || c == '\'').trim();
            if !clean.is_empty() {
                let target_path = if let Some(root) = &app.workspace.root {
                    root.join(clean)
                } else {
                    crate::workspace::default_workspace_dir().join(clean)
                };
                if !target_path.exists() {
                    if let Some(parent) = target_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::write(&target_path, "");
                    let _ = app.workspace.refresh();
                }
                app.open_file_path(target_path, now);
            } else {
                app.create_new_note(now);
            }
            true
        }
        "sort" | "sort!" => {
            handle_sort(app, cmd, args, now);
            true
        }
        _ => false,
    }
}

fn handle_export(app: &mut App, args: &str, now: f64) {
    if app.misc.mode == Mode::ScanReport {
        if let Some(ref res) = app.scan.active_scan_result {
            let md = crate::views::scan::export_scan_to_markdown(res);
            let safe_url = res
                .url
                .replace("https://", "")
                .replace("http://", "")
                .replace('/', "_")
                .replace(':', "_")
                .replace('?', "_");
            let default_name = format!("scan_{}.md", safe_url.trim_matches('_'));
            let clean_arg = args.trim_matches(|c| c == '"' || c == '\'').trim();
            let chosen_path = if clean_arg.is_empty() {
                rfd::FileDialog::new()
                    .set_file_name(&default_name)
                    .add_filter("Markdown Document (*.md)", &["md"])
                    .add_filter("Plain Text Document (*.txt)", &["txt"])
                    .save_file()
            } else {
                let p = std::path::PathBuf::from(clean_arg);
                if p.is_dir() {
                    Some(p.join(&default_name))
                } else if p.extension().is_none() {
                    Some(p.with_extension("md"))
                } else {
                    Some(p)
                }
            };

            if let Some(out_path) = chosen_path {
                match std::fs::write(&out_path, md) {
                    Ok(_) => {
                        app.set_status(
                            format!("Exported scan report: {}", out_path.display()),
                            now,
                        );
                    }
                    Err(e) => {
                        app.set_status(format!("Export failed: {}", e), now);
                    }
                }
            } else {
                app.set_status("Export cancelled", now);
            }
        } else {
            app.set_status("No scan report available to export", now);
        }
        return;
    }

    let clean_arg = args.trim_matches(|c| c == '"' || c == '\'').trim();
    let title = if app.notes.active_note_title.trim().is_empty() {
        "Untitled"
    } else {
        &app.notes.active_note_title
    };
    let safe_name = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let default_name = format!("{}.md", safe_name.trim());

    let chosen_path = if clean_arg.is_empty() {
        rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("Markdown Document (*.md)", &["md"])
            .add_filter("Plain Text Document (*.txt)", &["txt"])
            .save_file()
    } else {
        let p = std::path::PathBuf::from(clean_arg);
        if p.is_dir() {
            Some(p.join(&default_name))
        } else if p.extension().is_none() {
            Some(p.with_extension("md"))
        } else {
            Some(p)
        }
    };

    if let Some(out_path) = chosen_path {
        match std::fs::write(&out_path, app.editor.ed.text()) {
            Ok(_) => {
                app.set_status(format!("Exported to: {}", out_path.display()), now);
            }
            Err(e) => {
                app.set_status(format!("Export failed: {}", e), now);
            }
        }
    } else {
        app.set_status("Export cancelled", now);
    }
}

fn handle_import(app: &mut App, args: &str, now: f64) {
    let clean_arg = args.trim_matches(|c| c == '"' || c == '\'').trim();
    let chosen_file = if clean_arg.is_empty() {
        rfd::FileDialog::new().pick_file()
    } else {
        let mut p = std::path::PathBuf::from(clean_arg);
        if !p.exists() {
            if let Ok(cur) = std::env::current_dir() {
                let candidate_direct = cur.join(clean_arg);
                let candidate_md = cur.join(format!("{}.md", clean_arg));
                let candidate_txt = cur.join(format!("{}.txt", clean_arg));
                if candidate_direct.exists() {
                    p = candidate_direct;
                } else if candidate_md.exists() {
                    p = candidate_md;
                } else if candidate_txt.exists() {
                    p = candidate_txt;
                }
            }
        }
        Some(p)
    };

    if let Some(path) = chosen_file {
        app.open_file_path(path, now);
    } else {
        app.set_status("Import cancelled", now);
    }
}

fn handle_sort(app: &mut App, cmd: &str, args: &str, now: f64) {
    let (target_ed, is_doc) = if app.misc.mode == Mode::Doc {
        (&mut app.editor.doc_ed, true)
    } else {
        (&mut app.editor.ed, false)
    };

    let text = target_ed.text();
    if text.trim().is_empty() {
        app.set_status("Sort: buffer is empty", now);
        return;
    }

    let has_trailing_newline = text.ends_with('\n');
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if lines.is_empty() {
        app.set_status("Sort: no lines to sort", now);
        return;
    }

    let is_reverse = cmd.ends_with('!')
        || args.contains('!')
        || args
            .split_whitespace()
            .any(|a| a == "reverse" || a == "r" || a == "!r");
    let is_unique = args.split_whitespace().any(|a| a.contains('u'));
    let is_numeric = args.split_whitespace().any(|a| a.contains('n'));
    let is_case_insensitive = args.split_whitespace().any(|a| a.contains('i'));

    if is_numeric {
        lines.sort_by(|a, b| {
            let parse_leading_num = |s: &str| -> f64 {
                let trimmed = s.trim_start();
                let num_str: String = trimmed
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                num_str.parse::<f64>().unwrap_or(0.0)
            };
            let ord = parse_leading_num(a)
                .partial_cmp(&parse_leading_num(b))
                .unwrap_or(std::cmp::Ordering::Equal);
            if is_reverse {
                ord.reverse()
            } else {
                ord
            }
        });
    } else if is_case_insensitive {
        lines.sort_by(|a, b| {
            let ord = a.to_lowercase().cmp(&b.to_lowercase());
            if is_reverse {
                ord.reverse()
            } else {
                ord
            }
        });
    } else if is_reverse {
        lines.sort_by(|a, b| b.cmp(a));
    } else {
        lines.sort();
    }

    if is_unique {
        lines.dedup();
    }

    let mut new_text = lines.join("\n");
    if has_trailing_newline && !new_text.is_empty() {
        new_text.push('\n');
    }

    let cur = target_ed.cur;
    target_ed.set_text(&new_text);
    target_ed.cur = cur.min(target_ed.buf.len());

    if !is_doc {
        app.editor.is_dirty = true;
        app.misc.last_char_time = now;
        app.sync_active_tab();
    }

    // Keep Neovim backend in sync if initialized
    if let Some(backend) = app.services.vim_runtime.backend.as_mut() {
        let _ = backend.set_document(&new_text, 0, 0);
    }

    let count = lines.len();
    let desc = match (is_reverse, is_unique, is_numeric, is_case_insensitive) {
        (true, true, _, _) => "reverse unique",
        (true, false, true, _) => "reverse numeric",
        (true, false, false, _) => "reverse alphabetical",
        (false, true, _, _) => "unique alphabetical",
        (false, false, true, _) => "numeric",
        (false, false, false, true) => "case-insensitive",
        _ => "alphabetical",
    };
    app.set_status(format!("Sorted {} lines ({})", count, desc), now);
}
