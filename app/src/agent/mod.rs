//! MindForge AI Agent module powered by DeepSeek Pro.

pub mod client;
pub mod deepseek_ui;

/// Truncates by Unicode scalar values while reserving room for an ellipsis.
pub(super) fn truncate_with_ellipsis(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let prefix: String = chars.by_ref().take(max_chars.saturating_sub(3)).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        value.to_owned()
    }
}

use client::{deobfuscate_key, ApiMessage, AgentRequest, AgentResponse, AgentWorker};
use core::Note;
use eframe::egui::Rect;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: String,
    #[serde(default)]
    pub feedback: Option<bool>,
}

pub fn current_time_12h() -> String {
    chrono::Local::now().format("%-I:%M %p").to_string()
}

pub fn format_time_12h(ts: &str) -> String {
    if ts.contains("AM") || ts.contains("PM") || ts.contains("am") || ts.contains("pm") {
        return ts.to_string();
    }
    if let Ok(t) = chrono::NaiveTime::parse_from_str(ts.trim(), "%H:%M") {
        return t.format("%-I:%M %p").to_string();
    }
    ts.to_string()
}

pub struct AgentState {
    pub is_open: bool,
    pub window_rect: Option<Rect>,
    pub is_resizing: bool,
    pub chat_history: Vec<ChatMessage>,
    pub input_text: String,
    pub scroll_to_bottom: bool,
    pub is_thinking: bool,
    pub error_msg: Option<String>,
    pub worker: AgentWorker,
    pub deepseek_api_key_enc: String,
    pub deepseek_model: String,
    pub is_input_focused: bool,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            window_rect: None,
            is_resizing: false,
            chat_history: Vec::new(),
            input_text: String::new(),
            scroll_to_bottom: false,
            is_thinking: false,
            error_msg: None,
            worker: AgentWorker::new(),
            deepseek_api_key_enc: String::new(),
            deepseek_model: "deepseek-chat".to_string(), // Pro default
            is_input_focused: false,
        }
    }

    /// Check if background worker has returned an AI response
    pub fn poll_response(&mut self) {
        if let Ok(res) = self.worker.rx.try_recv() {
            self.is_thinking = false;
            match res {
                AgentResponse::Success(reply) => {
                    self.chat_history.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: reply,
                        timestamp: current_time_12h(),
                        feedback: None,
                    });
                    self.scroll_to_bottom = true;
                    self.error_msg = None;
                }
                AgentResponse::Error(err) => {
                    self.error_msg = Some(err.clone());
                    self.chat_history.push(ChatMessage {
                        role: MessageRole::Assistant,
                        content: format!("⚠️ **Error**: {}", err),
                        timestamp: current_time_12h(),
                        feedback: None,
                    });
                    self.scroll_to_bottom = true;
                }
            }
        }
    }

    /// Send current input prompt with dynamic database context to DeepSeek
    pub fn send_message(&mut self, notes: &[Note], active_note: Option<(&str, &str)>) {
        let text = self.input_text.trim().to_string();
        if text.is_empty() || self.is_thinking {
            return;
        }

        self.chat_history.push(ChatMessage {
            role: MessageRole::User,
            content: text.clone(),
            timestamp: current_time_12h(),
            feedback: None,
        });
        self.input_text.clear();
        self.is_thinking = true;
        self.scroll_to_bottom = true;
        self.error_msg = None;

        // Build system prompt with live database knowledge
        let system_prompt = build_database_system_prompt(notes, active_note);

        let mut api_messages = Vec::new();
        api_messages.push(ApiMessage {
            role: "system".to_string(),
            content: system_prompt,
        });

        let raw_key = deobfuscate_key(&self.deepseek_api_key_enc);
        if raw_key.trim().is_empty() {
            self.chat_history.push(ChatMessage {
                role: MessageRole::Assistant,
                content: "⚠️ DeepSeek API key is not configured.\n\nPlease open Settings (Ctrl+,) -> AI Agent tab and paste your API key to enable AI chatting.".to_string(),
                timestamp: current_time_12h(),
                feedback: None,
            });
            self.is_thinking = false;
            self.scroll_to_bottom = true;
            return;
        }

        // Add last 10 messages for conversational context (filtering any error notices)
        let history_start = self.chat_history.len().saturating_sub(10);
        for msg in &self.chat_history[history_start..] {
            if msg.content.starts_with("⚠️") {
                continue;
            }
            let role_str = match msg.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::System => "system",
            };
            api_messages.push(ApiMessage {
                role: role_str.to_string(),
                content: msg.content.clone(),
            });
        }

        let _ = self.worker.tx.send(AgentRequest::SendChat {
            api_key: raw_key,
            model: self.deepseek_model.clone(),
            messages: api_messages,
            temperature: 0.7,
        });
    }

    /// Clear current chat messages
    pub fn clear_chat(&mut self) {
        self.chat_history.clear();
        self.error_msg = None;
    }
}

/// Constructs a comprehensive knowledge-base system prompt so DeepSeek understands
/// the entire user database, when data was added, and what topics exist.
pub fn build_database_system_prompt(notes: &[Note], active_note: Option<(&str, &str)>) -> String {
    let now = chrono::Local::now();
    let now_str = now.format("%A, %B %d, %Y at %H:%M").to_string();

    let mut doc_index = String::new();
    for (i, n) in notes.iter().enumerate() {
        let created_str = n.created_at.format("%Y-%m-%d %H:%M").to_string();
        let word_count = n.body.split_whitespace().count();
        // Include summary / first snippet of body
        let preview = truncate_with_ellipsis(&n.body.replace('\n', " "), 180);
        doc_index.push_str(&format!(
            "{}. ID {}: \"{}\" | Added: {} | {} words | Snippet: \"{}\"\n",
            i + 1,
            n.id,
            n.topic,
            created_str,
            word_count,
            preview
        ));
    }

    let active_doc_context = if let Some((title, body)) = active_note {
        let snippet = truncate_with_ellipsis(body, 600);
        format!(
            "\n\nCURRENTLY OPEN DOCUMENT:\nTitle: \"{}\"\nContent:\n```\n{}\n```",
            title, snippet
        )
    } else {
        String::new()
    };

    format!(
        "You are MindForge AI, an advanced intelligent knowledge companion.\n\
        The current date and time is {}.\n\
        You have direct awareness of the user's SQLite knowledge base containing {} document(s).\n\n\
        USER KNOWLEDGE BASE INDEX:\n\
        {}\
        {}\n\n\
        GUIDELINES:\n\
        - Answer questions accurately using the user's notes and database timestamps.\n\
        - You know exactly when each note was created, what topics are covered, and how documents relate.\n\
        - When citing facts, mention the document topic and date created.\n\
        - Format responses cleanly with concise markdown.",
        now_str,
        notes.len(),
        if doc_index.is_empty() { "No documents created yet.\n" } else { &doc_index },
        active_doc_context
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    #[test]
    fn test_deepseek_key_obfuscation_roundtrip() {
        let raw = "sk-deepseek-1234567890abcdef";
        let enc = client::obfuscate_key(raw);
        assert_ne!(raw, enc);
        let dec = client::deobfuscate_key(&enc);
        assert_eq!(raw, dec);
    }

    #[test]
    fn test_deepseek_database_prompt_builder() {
        let dt = NaiveDateTime::parse_from_str("2026-09-23 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let notes = vec![
            Note {
                id: 1,
                topic: "Rust Ownership".to_string(),
                body: "Memory safety without garbage collection.".to_string(),
                struggled_with: None,
                created_at: dt,
            },
        ];

        let prompt = build_database_system_prompt(&notes, Some(("Rust Ownership", "Memory safety without garbage collection.")));
        assert!(prompt.contains("1 document(s)"));
        assert!(prompt.contains("Rust Ownership"));
        assert!(prompt.contains("2026-09-23 10:00"));
    }

    #[test]
    fn test_format_time_12h() {
        assert_eq!(format_time_12h("14:30"), "2:30 PM");
        assert_eq!(format_time_12h("09:15"), "9:15 AM");
        assert_eq!(format_time_12h("00:05"), "12:05 AM");
        assert_eq!(format_time_12h("12:00"), "12:00 PM");
        assert_eq!(format_time_12h("2:30 PM"), "2:30 PM");
    }

    #[test]
    fn test_chat_message_feedback_serde() {
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            content: "Hello!".to_string(),
            timestamp: "2:30 PM".to_string(),
            feedback: Some(true),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: ChatMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.feedback, Some(true));

        // Backwards compatibility with missing feedback field
        let old_json = r#"{"role":"Assistant","content":"Hi","timestamp":"2:30 PM"}"#;
        let old_parsed: ChatMessage = serde_json::from_str(old_json).unwrap();
        assert_eq!(old_parsed.feedback, None);
    }
}
