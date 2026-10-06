use chrono::{NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Card {
    pub id: i64,
    pub prompt: String,
    pub answer: String,
    pub tag: Option<String>,
    pub ease: f32,
    pub interval_days: u32,
    pub reps: u32,
    pub due: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub id: i64,
    pub card_id: i64,
    pub quality: u8,
    pub typed_answer: String,
    pub wpm: f32,
    pub reviewed_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: i64,
    pub topic: String,
    pub body: String,
    pub struggled_with: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: i64,
    pub decision: String,
    pub reasoning: String,
    pub prediction: String,
    pub confidence: u8, // 1 - 99
    pub review_on: NaiveDate,
    pub outcome: Option<i32>, // 1 if came true, 0 if false, None if pending
    pub lessons: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FocusState {
    pub topic1: String,
    pub topic2: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyActivity {
    pub date: String, // "YYYY-MM-DD"
    pub active_seconds: u32,
    pub keystrokes: u32,
    pub words_written: u32,
    pub notes_created: u32,
    pub notes_edited: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanRecord {
    pub id: i64,
    pub url: String,
    pub note: Option<String>,
    pub findings_json: String,
    pub scanned_at: String,
}

