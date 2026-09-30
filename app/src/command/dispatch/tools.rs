//! Integrated tools, web security audit scanner, terminal, stats, help, and application lifecycle (:term, :scan, :scans, :stats, :doc, :help, :settings, :quit).

use crate::app::App;
use crate::mode::Mode;

pub fn handle(app: &mut App, cmd: &str, args: &str, _raw: &str, now: f64) -> bool {
    match cmd {
        "help" | "guidance" | "guide" | "h" | "?" => {
            app.misc.mode = Mode::Help;
            app.modal.help_tab = 0;
            app.modal.help_scroll_y = 0.0;
            app.set_status("Help & Guidance opened as tab (Esc to return to notes)", now);
            true
        }
        "settings" | "setting" | "preferences" | "pref" | "config" => {
            app.modal.settings_open = !app.modal.settings_open;
            app.modal.settings_just_opened = app.modal.settings_open;
            if app.modal.settings_open {
                app.modal.settings_opened_at = now;
            }
            let msg = if app.modal.settings_open {
                "Preferences & Settings opened (Esc to close)"
            } else {
                "Preferences & Settings closed"
            };
            app.set_status(msg, now);
            true
        }
        "term" | "terminal" => {
            app.terminal.open = !app.terminal.open;
            if app.terminal.open {
                app.terminal.focused = true;
                app.set_status("Terminal opened (Ctrl+\\ to toggle, click editor to edit)", now);
            } else {
                app.terminal.focused = false;
                app.set_status("Terminal closed", now);
            }
            true
        }
        "stats" | "activity" => {
            app.misc.mode = Mode::Stats;
            true
        }
        "scan" | "security" => {
            if app.scan.scan_in_progress.is_some() {
                app.set_status("A scan is already in progress...", now);
                return true;
            }
            match parse_scan_args(args) {
                Ok((url, opts)) => {
                    let (tx, rx) = std::sync::mpsc::channel();
                    app.scan.scan_rx = Some(rx);
                    app.scan.scan_in_progress = Some(url.clone());
                    app.scan.prev_mode_before_scan = app.misc.mode;
                    app.set_status(format!("Scanning {}...", url), now);

                    let url_clone = url.clone();
                    std::thread::spawn(move || {
                        let result = webscan::scan(&url_clone, &opts);
                        match result {
                            Ok(res) => {
                                let _ = tx.send(Ok(res));
                            }
                            Err(err) => {
                                let _ = tx.send(Err(format!("{}: {:#}", url_clone, err)));
                            }
                        }
                    });
                }
                Err(usage) => {
                    app.set_status(usage, now);
                }
            }
            true
        }
        "scans" | "scanhistory" | "securityhistory" => {
            if let Some(ref db) = app.services.db {
                if let Ok(scans) = db.list_scans() {
                    app.scan.past_scans = scans;
                }
            }
            app.scan.prev_mode_before_scan = app.misc.mode;
            app.scan.scan_history_selected = 0;
            app.scan.scan_history_scroll_y = 0.0;
            app.misc.mode = Mode::ScanHistory;
            app.set_status("Webscan History (↑/↓ to navigate, Enter to view report, Esc to exit)", now);
            true
        }
        "doc" | "docs" | "tutorial" | "tutorials" | "document" | "documents" | "documentation" | "documentations" => {
            app.open_docs_mode(now);
            true
        }
        "bd" | "bdelete" | "close" | "tabclose" => {
            if app.misc.mode == Mode::Doc {
                app.close_doc_tab(app.tabs.active_doc_tab, now);
            } else {
                app.close_tab(app.tabs.active_tab, now);
            }
            true
        }
        "quit" | "q" => {
            if app.misc.mode == Mode::Help {
                app.misc.mode = Mode::Normal;
                app.set_status("Closed Help", now);
                return true;
            }
            std::process::exit(0);
        }
        _ => false,
    }
}

pub fn parse_scan_args(raw_args: &str) -> Result<(String, webscan::ScanOptions), String> {
    let mut url = String::new();
    let mut full = false;
    let mut probe_forms = false;
    let mut delay_ms = 200;
    let mut timeout_secs = 10;
    let mut note = None;

    let mut tokens = Vec::new();
    let mut cur_token = String::new();
    let mut in_quotes = false;
    for ch in raw_args.chars() {
        if ch == '"' || ch == '\'' {
            in_quotes = !in_quotes;
        } else if ch.is_whitespace() && !in_quotes {
            if !cur_token.is_empty() {
                tokens.push(cur_token);
                cur_token = String::new();
            }
        } else {
            cur_token.push(ch);
        }
    }
    if !cur_token.is_empty() {
        tokens.push(cur_token);
    }

    let mut i = 0;
    while i < tokens.len() {
        let tok = &tokens[i];
        if tok == "--full" {
            full = true;
        } else if tok == "--forms" || tok == "--probe-forms" {
            probe_forms = true;
        } else if tok == "--delay" {
            i += 1;
            if i < tokens.len() {
                if let Ok(d) = tokens[i].parse::<u64>() {
                    delay_ms = d;
                }
            }
        } else if tok == "--timeout" {
            i += 1;
            if i < tokens.len() {
                if let Ok(t) = tokens[i].parse::<u64>() {
                    timeout_secs = t;
                }
            }
        } else if tok == "--note" {
            i += 1;
            if i < tokens.len() {
                note = Some(tokens[i].clone());
            }
        } else if !tok.starts_with("--") && url.is_empty() {
            url = tok.clone();
        }
        i += 1;
    }

    if url.is_empty() {
        return Err("Usage: :scan <url> [--full] [--forms] [--note \"...\"]".to_string());
    }

    // Prepend https:// if protocol missing
    if !url.starts_with("http://") && !url.starts_with("https://") {
        url = format!("https://{}", url);
    }

    let opts = webscan::ScanOptions {
        full,
        probe_forms,
        delay_ms,
        timeout_secs,
        note,
    };

    Ok((url, opts))
}
