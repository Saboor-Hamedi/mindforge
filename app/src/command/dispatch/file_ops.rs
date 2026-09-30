//! File and document lifecycle commands (:w, :r, :d, :clear, :backup, :export, :import, :edit, :new).

use crate::app::App;
use crate::mode::Mode;
use crate::services::db_worker::DbMsg;

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "w" | "write" | "save" => {
            if app.misc.mode == Mode::Doc {
                app.set_status("Documentation files are read-only (changes not saved).", now);
                return true;
            }
            app.quick_save_active_note(now);
            true
        }
        "r" | "rename" => {
            if app.misc.mode == Mode::Doc {
                app.set_status("Documentation files are read-only and cannot be renamed.", now);
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
        "edit" | "editor" | "note" | "notes" => {
            app.misc.mode = Mode::Normal;
            app.set_status("Switched to Notes Editor", now);
            true
        }
        "new" | "n" => {
            app.create_new_note(now);
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
                        app.set_status(format!("Exported scan report: {}", out_path.display()), now);
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
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
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
        rfd::FileDialog::new()
            .add_filter("Markdown & Text Files", &["md", "txt", "markdown"])
            .pick_file()
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
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                let title = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Imported Note")
                    .to_string();
                let ext = path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("txt");
                let clean_content = content.replace("\r\n", "\n").replace('\r', "\n");
                let now_dt = chrono::Local::now().naive_local();
                if let Some(ref db) = app.services.db {
                    if let Ok(new_id) = db.add_note(&title, &clean_content, None, now_dt) {
                        app.notes.active_note_id = Some(new_id);
                        app.notes.notes_list.insert(0, core::Note {
                            id: new_id,
                            topic: title.clone(),
                            body: clean_content.clone(),
                            struggled_with: None,
                            created_at: now_dt,
                        });
                        app.notes.total_notes_count += 1;
                    }
                } else {
                    let _ = app.services.db_tx.send(DbMsg::SaveNote {
                        topic: title.clone(),
                        body: clean_content.clone(),
                        struggled: None,
                    });
                }
                app.notes.active_note_title = title.clone();
                app.editor.ed.set_text(&clean_content);
                app.editor.ed.cur = 0;
                app.editor.is_dirty = false;
                app.editor.scroll_y = 0.0;
                app.activity.pending_created += 1;
                app.set_status(format!("Imported: \"{}\" (.{})", title, ext), now);
            }
            Err(e) => {
                app.set_status(format!("Failed to import {}: {}", path.display(), e), now);
            }
        }
    } else {
        app.set_status("Import cancelled", now);
    }
}
