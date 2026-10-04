//! Database connection initialization, WAL configuration, snapshots, and schema migrations.

use super::Database;
use anyhow::{Context, Result};
use directories::ProjectDirs;
use rusqlite::{params, Connection};
use std::path::PathBuf;

impl Database {
    /// Opens the default database file located in the user's data directory.
    pub fn open_default() -> Result<Self> {
        let path = Self::get_db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::open(&path)
    }

    /// Resolves the default database path on disk.
    pub fn get_db_path() -> Result<PathBuf> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "mindforge", "mindforge") {
            Ok(proj_dirs.data_local_dir().join("mindforge.db"))
        } else {
            Ok(PathBuf::from("mindforge.db"))
        }
    }

    /// Opens SQLite database at a specific path and applies schema migrations.
    pub fn open(path: &PathBuf) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open SQLite database at {:?}", path))?;
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA busy_timeout = 5000;",
        )?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Creates an atomic, non-blocking timestamped snapshot of the database using VACUUM INTO.
    pub fn backup(&self, target_dir: &std::path::Path) -> Result<PathBuf> {
        if !target_dir.exists() {
            std::fs::create_dir_all(target_dir)?;
        }
        let now = chrono::Local::now();
        let filename = format!("mindforge_backup_{}.db", now.format("%Y%m%d_%H%M%S"));
        let target_path = target_dir.join(filename);
        let target_str = target_path.to_str().context("Invalid target backup path")?;

        self.conn
            .execute("VACUUM INTO ?1", params![target_str])
            .with_context(|| format!("Failed to create backup at {}", target_str))?;
        Ok(target_path)
    }

    /// Opens an in-memory SQLite database (ideal for unit testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    /// Auto-migrates tables if they don't exist.
    pub fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS cards (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prompt TEXT NOT NULL,
                answer TEXT NOT NULL,
                tag TEXT,
                ease REAL NOT NULL DEFAULT 2.5,
                interval_days INTEGER NOT NULL DEFAULT 0,
                reps INTEGER NOT NULL DEFAULT 0,
                due TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS reviews (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                card_id INTEGER NOT NULL REFERENCES cards(id) ON DELETE CASCADE,
                quality INTEGER NOT NULL,
                typed_answer TEXT NOT NULL,
                wpm REAL NOT NULL,
                reviewed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS notes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                topic TEXT NOT NULL,
                body TEXT NOT NULL,
                struggled_with TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS decisions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                decision TEXT NOT NULL,
                reasoning TEXT NOT NULL,
                prediction TEXT NOT NULL,
                confidence INTEGER NOT NULL,
                review_on TEXT NOT NULL,
                outcome INTEGER,
                lessons TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS focus (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                topic1 TEXT NOT NULL DEFAULT '',
                topic2 TEXT NOT NULL DEFAULT ''
            );

            INSERT OR IGNORE INTO focus (id, topic1, topic2) VALUES (1, '', '');

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS daily_activity (
                date TEXT PRIMARY KEY,
                active_seconds INTEGER NOT NULL DEFAULT 0,
                keystrokes INTEGER NOT NULL DEFAULT 0,
                words_written INTEGER NOT NULL DEFAULT 0,
                notes_created INTEGER NOT NULL DEFAULT 0,
                notes_edited INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL,
                note TEXT,
                findings_json TEXT NOT NULL,
                scanned_at TEXT NOT NULL
            );
            ",
        )?;
        Ok(())
    }
}
