//! DeepSeek API client and background worker for MindForge AI Agent.

use serde::{Deserialize, Serialize};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Obfuscate API key with XOR + base64 so it is not stored as plain text in settings JSON.
pub fn obfuscate_key(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    const SALT: &[u8] = b"mindforge_deepseek_salt_2026";
    let bytes = raw.as_bytes();
    let xored: Vec<u8> = bytes
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ SALT[i % SALT.len()])
        .collect();
    // Simple hex encoding
    xored.iter().map(|b| format!("{:02x}", b)).collect()
}

/// De-obfuscate API key from hex-encoded XOR format.
pub fn deobfuscate_key(enc: &str) -> String {
    if enc.is_empty() || enc.len() % 2 != 0 {
        return String::new();
    }
    const SALT: &[u8] = b"mindforge_deepseek_salt_2026";
    let mut xored = Vec::new();
    for i in (0..enc.len()).step_by(2) {
        if let Ok(b) = u8::from_str_radix(&enc[i..i + 2], 16) {
            xored.push(b);
        } else {
            return String::new();
        }
    }
    let raw: Vec<u8> = xored
        .iter()
        .enumerate()
        .map(|(i, &b)| b ^ SALT[i % SALT.len()])
        .collect();
    String::from_utf8(raw).unwrap_or_default()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ApiMessage>,
    temperature: f32,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: ApiMessage,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

pub enum AgentRequest {
    SendChat {
        api_key: String,
        model: String,
        messages: Vec<ApiMessage>,
        temperature: f32,
    },
}

pub enum AgentResponse {
    Success(String),
    Error(String),
}

pub struct AgentWorker {
    pub tx: Sender<AgentRequest>,
    pub rx: Receiver<AgentResponse>,
}

impl AgentWorker {
    pub fn new() -> Self {
        let (req_tx, req_rx) = channel::<AgentRequest>();
        let (res_tx, res_rx) = channel::<AgentResponse>();

        thread::spawn(move || {
            while let Ok(req) = req_rx.recv() {
                match req {
                    AgentRequest::SendChat {
                        api_key,
                        model,
                        messages,
                        temperature,
                    } => {
                        let res = call_deepseek_api(&api_key, &model, messages, temperature);
                        let _ = res_tx.send(res);
                    }
                }
            }
        });

        Self {
            tx: req_tx,
            rx: res_rx,
        }
    }
}

fn call_deepseek_api(
    api_key: &str,
    model: &str,
    messages: Vec<ApiMessage>,
    temperature: f32,
) -> AgentResponse {
    if api_key.trim().is_empty() {
        return AgentResponse::Error("DeepSeek API key is not configured. Please open Settings (Ctrl+,) -> AI Agent to set your key.".to_string());
    }

    let payload = ChatCompletionRequest {
        model: model.to_string(),
        messages,
        temperature,
        stream: false,
    };

    let url = "https://api.deepseek.com/chat/completions";
    let body_json = match serde_json::to_string(&payload) {
        Ok(j) => j,
        Err(e) => return AgentResponse::Error(format!("Failed to serialize payload: {}", e)),
    };

    let response = ureq::post(url)
        .set("Authorization", &format!("Bearer {}", api_key.trim()))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_string(&body_json);

    match response {
        Ok(resp) => {
            if resp.status() == 200 {
                match resp.into_json::<ChatCompletionResponse>() {
                    Ok(parsed) => {
                        if let Some(first) = parsed.choices.into_iter().next() {
                            AgentResponse::Success(first.message.content)
                        } else {
                            AgentResponse::Error("DeepSeek returned an empty response choices array.".to_string())
                        }
                    }
                    Err(e) => AgentResponse::Error(format!("Failed to parse DeepSeek response: {}", e)),
                }
            } else {
                AgentResponse::Error(format!("DeepSeek API returned HTTP status {}: {}", resp.status(), resp.status_text()))
            }
        }
        Err(ureq::Error::Status(code, resp)) => {
            let body = resp.into_string().unwrap_or_default();
            AgentResponse::Error(format!("DeepSeek API Error (HTTP {}): {}", code, body))
        }
        Err(e) => AgentResponse::Error(format!("Network connection error to DeepSeek API: {}", e)),
    }
}
