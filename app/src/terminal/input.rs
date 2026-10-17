//! Keyboard input translation and event forwarding to the PTY backend.
//!
//! egui events are translated into standard Linux/Bash ANSI byte sequences and
//! written directly to the active session's backend with zero latency. This is
//! the sole event writer to the backend, so `egui_term`'s own focus handling is
//! disabled to avoid double-processing.

use eframe::egui::{self, Key, Modifiers};
use egui_term::BackendCommand;

use super::terminal::TerminalPane;

impl TerminalPane {
    /// Feed an egui event directly to the active session's PTY backend with zero latency.
    pub fn feed_event(&mut self, event: &egui::Event, modifiers: Modifiers) {
        if let Some(active) = self.sessions.get_mut(self.active_session_idx) {
            match event {
                egui::Event::Text(text) => {
                    if !modifiers.ctrl && !modifiers.command && !modifiers.alt {
                        active
                            .backend
                            .process_command(BackendCommand::Write(text.as_bytes().to_vec()));
                    }
                }
                egui::Event::Paste(text) => {
                    active
                        .backend
                        .process_command(BackendCommand::Write(text.as_bytes().to_vec()));
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    // Shift + Navigation keys scroll the terminal history buffer directly
                    if modifiers.shift && !modifiers.ctrl && !modifiers.alt {
                        match key {
                            Key::PageUp => {
                                active.backend.process_command(BackendCommand::Scroll(20));
                                return;
                            }
                            Key::PageDown => {
                                active.backend.process_command(BackendCommand::Scroll(-20));
                                return;
                            }
                            Key::ArrowUp => {
                                active.backend.process_command(BackendCommand::Scroll(3));
                                return;
                            }
                            Key::ArrowDown => {
                                active.backend.process_command(BackendCommand::Scroll(-3));
                                return;
                            }
                            Key::Home => {
                                active.backend.process_command(BackendCommand::Scroll(100_000));
                                return;
                            }
                            Key::End => {
                                active.backend.process_command(BackendCommand::Scroll(-100_000));
                                return;
                            }
                            _ => {}
                        }
                    }

                    if let Some(bytes) = map_key_to_terminal_bytes(*key, *modifiers) {
                        active.backend.process_command(BackendCommand::Write(bytes));
                    }
                }
                _ => {}
            }
        }
    }
}

/// Translates keys and Ctrl combinations into standard Linux/Bash ANSI sequences.
fn map_key_to_terminal_bytes(key: Key, modifiers: Modifiers) -> Option<Vec<u8>> {
    // Word jumping with Ctrl+Left / Ctrl+Right
    if (modifiers.ctrl || modifiers.command) && key == Key::ArrowLeft {
        return Some(b"\x1b[1;5D".to_vec());
    }
    if (modifiers.ctrl || modifiers.command) && key == Key::ArrowRight {
        return Some(b"\x1b[1;5C".to_vec());
    }

    // Standard Ctrl combinations in Bash/Linux
    if modifiers.ctrl || modifiers.command {
        return match key {
            Key::A => Some(vec![0x01]), // Line start
            Key::B => Some(vec![0x02]), // Back char
            Key::C => Some(vec![0x03]), // SIGINT / Break
            Key::D => Some(vec![0x04]), // EOF / Exit
            Key::E => Some(vec![0x05]), // Line end
            Key::F => Some(vec![0x06]), // Forward char
            Key::G => Some(vec![0x07]), // Bell
            Key::H => Some(vec![0x08]), // Backspace
            Key::I => Some(vec![0x09]), // Tab
            Key::J => Some(vec![0x0a]), // Linefeed
            Key::K => Some(vec![0x0b]), // Kill to end
            Key::L => Some(vec![0x0c]), // Clear screen
            Key::N => Some(vec![0x0e]), // Next line
            Key::P => Some(vec![0x10]), // Prev line
            Key::R => Some(vec![0x12]), // Reverse search
            Key::T => Some(vec![0x14]), // Transpose chars
            Key::U => Some(vec![0x15]), // Delete to start
            Key::V => Some(vec![0x16]), // Literal next / paste
            Key::W => Some(vec![0x17]), // Delete word
            Key::Z => Some(vec![0x1a]), // SIGTSTP / Suspend
            Key::ArrowUp => Some(b"\x1b[1;5A".to_vec()),
            Key::ArrowDown => Some(b"\x1b[1;5B".to_vec()),
            _ => None,
        };
    }

    // Standard navigation and control keys
    match key {
        Key::Enter => Some(vec![b'\r']),
        Key::Backspace => Some(vec![0x7f]),
        Key::Tab => Some(vec![b'\t']),
        Key::Escape => Some(vec![0x1b]),
        Key::ArrowUp => Some(b"\x1b[A".to_vec()),
        Key::ArrowDown => Some(b"\x1b[B".to_vec()),
        Key::ArrowRight => Some(b"\x1b[C".to_vec()),
        Key::ArrowLeft => Some(b"\x1b[D".to_vec()),
        Key::Home => Some(b"\x1b[H".to_vec()),
        Key::End => Some(b"\x1b[F".to_vec()),
        Key::Delete => Some(b"\x1b[3~".to_vec()),
        Key::Insert => Some(b"\x1b[2~".to_vec()),
        Key::PageUp => Some(b"\x1b[5~".to_vec()),
        Key::PageDown => Some(b"\x1b[6~".to_vec()),
        Key::F1 => Some(b"\x1bOP".to_vec()),
        Key::F2 => Some(b"\x1bOQ".to_vec()),
        Key::F3 => Some(b"\x1bOR".to_vec()),
        Key::F4 => Some(b"\x1bOS".to_vec()),
        Key::F5 => Some(b"\x1b[15~".to_vec()),
        Key::F6 => Some(b"\x1b[17~".to_vec()),
        Key::F7 => Some(b"\x1b[18~".to_vec()),
        Key::F8 => Some(b"\x1b[19~".to_vec()),
        Key::F9 => Some(b"\x1b[20~".to_vec()),
        Key::F10 => Some(b"\x1b[21~".to_vec()),
        Key::F11 => Some(b"\x1b[23~".to_vec()),
        Key::F12 => Some(b"\x1b[24~".to_vec()),
        _ => None,
    }
}
