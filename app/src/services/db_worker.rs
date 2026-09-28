//! Asynchronous background database worker thread and message protocol.
//!
//! The UI thread sends `DbMsg` variants through a channel; a dedicated
//! background thread receives and executes them sequentially against SQLite.
//! This prevents disk I/O from blocking the UI thread, keeping the editor
//! at 120+ FPS even during heavy write operations.
//!
//! The worker also syncs settings to a `settings.json` file for external
//! tooling compatibility.

use chrono::{Local, NaiveDate};
use core::Database;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Message protocol between the UI thread and the background DB worker.
/// Each variant represents a single database operation to execute.
#[allow(dead_code)]
pub enum DbMsg {
    AddCard {
        prompt: String,
        answer: String,
        tag: Option<String>,
        today: NaiveDate,
    },
    RecordReview {
        card_id: i64,
        quality: u8,
        typed: String,
        wpm: f32,
    },
    UpdateCardSm2 {
        id: i64,
        ease: f32,
        interval: u32,
        reps: u32,
        due: NaiveDate,
    },
    SaveSetting {
        key: String,
        val: String,
    },
    SaveFocus {
        t1: String,
        t2: String,
    },
    SaveNote {
        topic: String,
        body: String,
        struggled: Option<String>,
    },
    UpdateNote {
        id: i64,
        body: String,
    },
    RenameNote {
        id: i64,
        new_topic: String,
    },
    DeleteNote {
        id: i64,
    },
    AddDecision {
        decision: String,
        reasoning: String,
        prediction: String,
        confidence: u8,
        review_on: NaiveDate,
    },
    ResolveDecision {
        id: i64,
        outcome: i32,
        lessons: String,
    },
    FlushActivity {
        date: String,
        delta_secs: u32,
        delta_keys: u32,
        delta_words: u32,
        delta_created: u32,
        delta_edited: u32,
    },
}

/// Spawns the background database worker thread.
///
/// Returns a `Sender<DbMsg>` handle. The UI thread sends messages through
/// this channel and never blocks on disk I/O — the worker thread owns the
/// exclusive `Database` connection and processes messages sequentially.
///
/// If the database fails to open, the worker exits silently (the UI will
/// show appropriate error states when operations don't complete).
pub fn spawn_db_worker() -> Sender<DbMsg> {
    let (tx, rx): (Sender<DbMsg>, Receiver<DbMsg>) = channel();
    thread::spawn(move || {
        if let Ok(db) = Database::open_default() {
            while let Ok(msg) = rx.recv() {
                let now = Local::now().naive_local();
                match msg {
                    DbMsg::AddCard {
                        prompt,
                        answer,
                        tag,
                        today,
                    } => {
                        let _ = db.add_card(&prompt, &answer, tag.as_deref(), today);
                    }
                    DbMsg::RecordReview {
                        card_id,
                        quality,
                        typed,
                        wpm,
                    } => {
                        let _ = db.record_review(card_id, quality, &typed, wpm, now);
                    }
                    DbMsg::UpdateCardSm2 {
                        id,
                        ease,
                        interval,
                        reps,
                        due,
                    } => {
                        let _ = db.update_card_sm2(id, ease, interval, reps, due);
                    }
                    DbMsg::SaveSetting { key, val } => {
                        let _ = db.set_setting(&key, &val);
                        sync_setting_json(&key, &val);
                    }
                    DbMsg::SaveFocus { t1, t2 } => {
                        let _ = db.set_focus(&t1, &t2);
                    }
                    DbMsg::SaveNote {
                        topic,
                        body,
                        struggled,
                    } => {
                        let _ = db.add_note(&topic, &body, struggled.as_deref(), now);
                    }
                    DbMsg::UpdateNote { id, body } => {
                        let _ = db.update_note(id, &body);
                    }
                    DbMsg::RenameNote { id, new_topic } => {
                        let _ = db.rename_note(id, &new_topic);
                    }
                    DbMsg::DeleteNote { id } => {
                        let _ = db.delete_note(id);
                    }
                    DbMsg::AddDecision {
                        decision,
                        reasoning,
                        prediction,
                        confidence,
                        review_on,
                    } => {
                        let _ = db.add_decision(
                            &decision,
                            &reasoning,
                            &prediction,
                            confidence,
                            review_on,
                            now,
                        );
                    }
                    DbMsg::ResolveDecision {
                        id,
                        outcome,
                        lessons,
                    } => {
                        let _ = db.resolve_decision(id, outcome, &lessons);
                    }
                    DbMsg::FlushActivity {
                        date,
                        delta_secs,
                        delta_keys,
                        delta_words,
                        delta_created,
                        delta_edited,
                    } => {
                        let _ = db.record_daily_activity(
                            &date,
                            delta_secs,
                            delta_keys,
                            delta_words,
                            delta_created,
                            delta_edited,
                        );
                    }
                }
            }
        }
    });
    tx
}

/// Mirrors a setting key-value pair to `settings.json` alongside the database.
///
/// This allows external tools and scripts to read settings without needing
/// to parse SQLite. Failures are silently ignored — the database remains
/// the source of truth.
fn sync_setting_json(key: &str, val: &str) {
    if let Ok(path) = core::Database::get_db_path() {
        if let Some(parent) = path.parent() {
            let json_path = parent.join("settings.json");
            let mut map: std::collections::BTreeMap<String, String> = std::fs::read_to_string(&json_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default();
            map.insert(key.to_string(), val.to_string());
            if let Ok(serialized) = serde_json::to_string_pretty(&map) {
                let _ = std::fs::write(json_path, serialized);
            }
        }
    }
}

