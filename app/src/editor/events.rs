use eframe::egui::{Key, Modifiers};

#[derive(Debug, Clone, Copy)]
pub struct EditorKeyEvent {
    pub key: Key,
    pub modifiers: Modifiers,
}

#[derive(Debug, Clone, Copy)]
pub enum EditorMouseEvent {
    Press {
        row: usize,
        column: usize,
        button: u8,
    },
    #[allow(dead_code)]
    Drag {
        row: usize,
        column: usize,
    },
    Release {
        row: usize,
        column: usize,
        button: u8,
    },
}
