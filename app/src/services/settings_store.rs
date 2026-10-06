//! Asynchronous background filesystem settings and metadata storage worker.
//!
//! The UI thread sends `StorageMsg` variants through a channel; a dedicated
//! background thread receives and executes them against lightweight JSON files
//! (`settings.json`, `activity.json`, `scans.json`) inside `.mindforge`.
//! 100% pure filesystem storage with zero locks.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Message protocol between the UI thread and the background storage worker.
#[allow(dead_code)]
pub enum StorageMsg {
    SaveSetting {
        key: String,
        val: String,
    },
    SaveFocus {
        t1: String,
        t2: String,
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

/// Backwards-compatible alias for StorageMsg
pub type DbMsg = StorageMsg;

#[derive(Default, Serialize, Deserialize)]
struct ActivityStore {
    days: HashMap<String, ActivityDay>,
}

#[derive(Default, Serialize, Deserialize)]
struct ActivityDay {
    active_seconds: u32,
    keystrokes: u32,
    words_written: u32,
    notes_created: u32,
    notes_edited: u32,
}

fn storage_dir() -> PathBuf {
    let dir = crate::workspace::default_workspace_dir().join(".mindforge");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Spawns the background filesystem settings and metadata storage worker thread.
pub fn spawn_settings_store() -> Sender<StorageMsg> {
    let (tx, rx): (Sender<StorageMsg>, Receiver<StorageMsg>) = channel();
    thread::spawn(move || {
        let dir = storage_dir();
        let settings_file = dir.join("settings.json");
        let activity_file = dir.join("activity.json");

        // Load existing settings into memory
        let mut settings_map: HashMap<String, String> = std::fs::read_to_string(&settings_file)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();

        while let Ok(msg) = rx.recv() {
            match msg {
                StorageMsg::SaveSetting { key, val } => {
                    settings_map.insert(key, val);
                    if let Ok(json) = serde_json::to_string_pretty(&settings_map) {
                        let _ = std::fs::write(&settings_file, json);
                    }
                }
                StorageMsg::SaveFocus { t1, t2 } => {
                    settings_map.insert("focus_topic1".into(), t1);
                    settings_map.insert("focus_topic2".into(), t2);
                    if let Ok(json) = serde_json::to_string_pretty(&settings_map) {
                        let _ = std::fs::write(&settings_file, json);
                    }
                }
                StorageMsg::FlushActivity {
                    date,
                    delta_secs,
                    delta_keys,
                    delta_words,
                    delta_created,
                    delta_edited,
                } => {
                    let mut store: ActivityStore = std::fs::read_to_string(&activity_file)
                        .ok()
                        .and_then(|s| serde_json::from_str(&s).ok())
                        .unwrap_or_default();

                    let entry = store.days.entry(date).or_default();
                    entry.active_seconds = entry.active_seconds.saturating_add(delta_secs);
                    entry.keystrokes = entry.keystrokes.saturating_add(delta_keys);
                    entry.words_written = entry.words_written.saturating_add(delta_words);
                    entry.notes_created = entry.notes_created.saturating_add(delta_created);
                    entry.notes_edited = entry.notes_edited.saturating_add(delta_edited);

                    if let Ok(json) = serde_json::to_string_pretty(&store) {
                        let _ = std::fs::write(&activity_file, json);
                    }
                }
            }
        }
    });
    tx
}

/// Backwards-compatible alias for spawn_settings_store
pub use spawn_settings_store as spawn_db_worker;

/// Reads a setting directly from `settings.json`.
pub fn get_stored_setting(key: &str) -> Option<String> {
    let path = storage_dir().join("settings.json");
    let content = std::fs::read_to_string(path).ok()?;
    let map: HashMap<String, String> = serde_json::from_str(&content).ok()?;
    map.get(key).cloned()
}

/// Reads all settings from `settings.json`.
pub fn get_all_stored_settings() -> HashMap<String, String> {
    let path = storage_dir().join("settings.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Synchronously saves a single setting directly to `settings.json`.
pub fn save_setting_sync(key: &str, val: &str) {
    let dir = storage_dir();
    let settings_file = dir.join("settings.json");
    let mut map: HashMap<String, String> = std::fs::read_to_string(&settings_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    map.insert(key.to_string(), val.to_string());
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&settings_file, json);
    }
}

/// Synchronously saves multiple settings atomically directly to `settings.json`.
pub fn save_settings_batch_sync(entries: &[(&str, String)]) {
    let dir = storage_dir();
    let settings_file = dir.join("settings.json");
    let mut map: HashMap<String, String> = std::fs::read_to_string(&settings_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    for (k, v) in entries {
        map.insert((*k).to_string(), v.clone());
    }
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&settings_file, json);
    }
}

/// Reads recent activity history from `activity.json`.
pub fn get_recent_activity(days: usize) -> Vec<core::DailyActivity> {
    let path = storage_dir().join("activity.json");
    let store: ActivityStore = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    let mut records: Vec<core::DailyActivity> = store
        .days
        .into_iter()
        .map(|(date, day)| core::DailyActivity {
            date,
            active_seconds: day.active_seconds,
            keystrokes: day.keystrokes,
            words_written: day.words_written,
            notes_created: day.notes_created,
            notes_edited: day.notes_edited,
        })
        .collect();
    records.sort_by(|a, b| b.date.cmp(&a.date));
    records.truncate(days);
    records
}

/// Computes lifetime totals from `activity.json`: (total_seconds, total_keystrokes, total_words, active_days)
pub fn get_lifetime_activity() -> (u64, u64, u64, usize) {
    let path = storage_dir().join("activity.json");
    let store: ActivityStore = std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    let mut total_seconds: u64 = 0;
    let mut total_keystrokes: u64 = 0;
    let mut total_words: u64 = 0;
    let active_days = store.days.len();
    for day in store.days.values() {
        total_seconds = total_seconds.saturating_add(day.active_seconds as u64);
        total_keystrokes = total_keystrokes.saturating_add(day.keystrokes as u64);
        total_words = total_words.saturating_add(day.words_written as u64);
    }
    (total_seconds, total_keystrokes, total_words, active_days)
}

/// Reads past security scans from `scans.json`.
pub fn list_stored_scans() -> Vec<core::ScanRecord> {
    let path = storage_dir().join("scans.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default()
}

/// Appends a new scan result to `scans.json`.
pub fn save_stored_scan(url: &str, note: Option<&str>, findings_json: &str) {
    let path = storage_dir().join("scans.json");
    let mut scans: Vec<core::ScanRecord> = list_stored_scans();
    let id = (scans.len() as i64) + 1;
    scans.push(core::ScanRecord {
        id,
        url: url.to_string(),
        note: note.map(|s| s.to_string()),
        findings_json: findings_json.to_string(),
        scanned_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    });
    if let Ok(json) = serde_json::to_string_pretty(&scans) {
        let _ = std::fs::write(path, json);
    }
}
