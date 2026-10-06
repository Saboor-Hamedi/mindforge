//! Configurable Vim keymap registry.
//!
//! Maps concrete keystrokes and characters to abstract `VimAction` definitions.
//! Allows seamless customization or remapping of shortcuts by users without
//! altering underlying engine or motion logic.

use crate::settings::types::{InsertPosition, TextObjectKind, VimAction, VimMotion, VimOperator};
use eframe::egui::Key;
use std::collections::HashMap;

/// Represents either a single character or a special keyboard key combination.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyStroke {
    Char(char),
    Key {
        key: Key,
        ctrl: bool,
        shift: bool,
        alt: bool,
    },
}

impl From<char> for KeyStroke {
    fn from(c: char) -> Self {
        KeyStroke::Char(c)
    }
}

impl KeyStroke {
    /// Builds a `KeyStroke` from an egui key event plus its modifiers.
    pub fn from_key(key: Key, modifiers: eframe::egui::Modifiers) -> Self {
        KeyStroke::Key {
            key,
            ctrl: modifiers.ctrl || modifiers.command,
            shift: modifiers.shift,
            alt: modifiers.alt,
        }
    }
}

/// Configurable keymap table for Vim modes.
#[derive(Debug, Clone)]
pub struct VimKeymap {
    /// Normal mode key actions.
    pub normal: HashMap<KeyStroke, VimAction>,
    /// Visual and VisualLine mode key actions.
    pub visual: HashMap<KeyStroke, VimAction>,
    /// Operator-pending actions (e.g. `(Delete, 'd') -> OperatorLine(Delete)`).
    pub operator_combinations: HashMap<(VimOperator, char), VimAction>,
    /// Two-character combination motions (e.g. `('g', 'g') -> Motion(BufferStart)`).
    pub combinations: HashMap<(char, char), VimAction>,
}

impl Default for VimKeymap {
    fn default() -> Self {
        Self::new_standard()
    }
}

impl VimKeymap {
    /// Constructs a standard default Vim keymap.
    pub fn new_standard() -> Self {
        let mut normal = HashMap::new();
        let mut visual = HashMap::new();
        let mut operator_combinations = HashMap::new();
        let mut combinations = HashMap::new();

        // ── Normal Mode Motions ──────────────────────────────────────────────
        normal.insert('h'.into(), VimAction::Motion(VimMotion::Left));
        normal.insert('l'.into(), VimAction::Motion(VimMotion::Right));
        normal.insert('j'.into(), VimAction::Motion(VimMotion::DownVisual));
        normal.insert('k'.into(), VimAction::Motion(VimMotion::UpVisual));
        normal.insert('w'.into(), VimAction::Motion(VimMotion::WordForward));
        normal.insert('b'.into(), VimAction::Motion(VimMotion::WordBackward));
        normal.insert('0'.into(), VimAction::Motion(VimMotion::LineStart));
        normal.insert('$'.into(), VimAction::Motion(VimMotion::LineEnd));
        normal.insert('G'.into(), VimAction::Motion(VimMotion::BufferEnd));
        normal.insert(' '.into(), VimAction::Motion(VimMotion::Right));

        // Arrow and control keys in Normal mode
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowLeft,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Left),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowRight,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Right),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowUp,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::UpVisual),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowDown,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::DownVisual),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::Home,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::LineStart),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::End,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::LineEnd),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowLeft,
                ctrl: true,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::WordBackward),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::ArrowRight,
                ctrl: true,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::WordForward),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::Enter,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::DownVisual),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::Backspace,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Left),
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::Delete,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::DeleteChar,
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::Space,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Right),
        );

        // ── Normal Mode Verbs & Editing ──────────────────────────────────────
        normal.insert('x'.into(), VimAction::DeleteChar);
        normal.insert('u'.into(), VimAction::Undo);
        normal.insert(
            KeyStroke::Key {
                key: Key::R,
                ctrl: true,
                shift: false,
                alt: false,
            },
            VimAction::Redo,
        );
        normal.insert('p'.into(), VimAction::Paste { before: false });
        normal.insert('P'.into(), VimAction::Paste { before: true });
        normal.insert(
            KeyStroke::Key {
                key: Key::D,
                ctrl: true,
                shift: false,
                alt: false,
            },
            VimAction::DuplicateLine,
        );
        normal.insert(
            KeyStroke::Key {
                key: Key::X,
                ctrl: true,
                shift: true,
                alt: false,
            },
            VimAction::ToggleTaskCheckbox,
        );

        // ── Mode Transitions ─────────────────────────────────────────────────
        normal.insert('i'.into(), VimAction::EnterInsert(InsertPosition::AtCursor));
        normal.insert(
            'a'.into(),
            VimAction::EnterInsert(InsertPosition::AfterCursor),
        );
        normal.insert(
            'I'.into(),
            VimAction::EnterInsert(InsertPosition::LineStart),
        );
        normal.insert('A'.into(), VimAction::EnterInsert(InsertPosition::LineEnd));
        normal.insert(
            'o'.into(),
            VimAction::EnterInsert(InsertPosition::LineBelow),
        );
        normal.insert(
            'O'.into(),
            VimAction::EnterInsert(InsertPosition::LineAbove),
        );
        normal.insert('v'.into(), VimAction::EnterVisual { is_line: false });
        normal.insert('V'.into(), VimAction::EnterVisual { is_line: true });

        // ── Operators (Prefixes) ─────────────────────────────────────────────
        normal.insert('d'.into(), VimAction::Operator(VimOperator::Delete));
        normal.insert(
            'D'.into(),
            VimAction::OperatorToEndOfLine(VimOperator::Delete),
        );
        normal.insert('y'.into(), VimAction::Operator(VimOperator::Yank));
        normal.insert('Y'.into(), VimAction::OperatorLine(VimOperator::Yank));
        normal.insert('c'.into(), VimAction::Operator(VimOperator::Change));
        normal.insert(
            'C'.into(),
            VimAction::OperatorToEndOfLine(VimOperator::Change),
        );

        // ── In-Buffer Search ─────────────────────────────────────────────────
        normal.insert('/'.into(), VimAction::EnterSearch { backward: false });
        normal.insert('?'.into(), VimAction::EnterSearch { backward: true });
        normal.insert('n'.into(), VimAction::RepeatSearch { reverse: false });
        normal.insert('N'.into(), VimAction::RepeatSearch { reverse: true });

        // ── Operator Combinations (e.g. `dd`, `yy`, `cc`, `dw`, `yw`, `cw`) ──
        operator_combinations.insert(
            (VimOperator::Delete, 'd'),
            VimAction::OperatorLine(VimOperator::Delete),
        );
        operator_combinations.insert(
            (VimOperator::Yank, 'y'),
            VimAction::OperatorLine(VimOperator::Yank),
        );
        operator_combinations.insert(
            (VimOperator::Change, 'c'),
            VimAction::OperatorLine(VimOperator::Change),
        );

        operator_combinations.insert(
            (VimOperator::Delete, 'w'),
            VimAction::Motion(VimMotion::WordForward),
        );
        operator_combinations.insert(
            (VimOperator::Yank, 'w'),
            VimAction::Motion(VimMotion::WordForward),
        );
        operator_combinations.insert(
            (VimOperator::Change, 'w'),
            VimAction::Motion(VimMotion::WordForward),
        );

        // ── Visual Mode Bindings ─────────────────────────────────────────────
        visual.insert('h'.into(), VimAction::Motion(VimMotion::Left));
        visual.insert('l'.into(), VimAction::Motion(VimMotion::Right));
        visual.insert('j'.into(), VimAction::Motion(VimMotion::DownVisual));
        visual.insert('k'.into(), VimAction::Motion(VimMotion::UpVisual));
        visual.insert('w'.into(), VimAction::Motion(VimMotion::WordForward));
        visual.insert('b'.into(), VimAction::Motion(VimMotion::WordBackward));
        visual.insert('0'.into(), VimAction::Motion(VimMotion::LineStart));
        visual.insert('$'.into(), VimAction::Motion(VimMotion::LineEnd));
        visual.insert('G'.into(), VimAction::Motion(VimMotion::BufferEnd));
        visual.insert(' '.into(), VimAction::Motion(VimMotion::Right));

        visual.insert(
            KeyStroke::Key {
                key: Key::ArrowLeft,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Left),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::ArrowRight,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Right),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::ArrowUp,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::UpVisual),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::ArrowDown,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::DownVisual),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::Enter,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::DownVisual),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::Backspace,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Left),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::Delete,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Operator(VimOperator::Delete),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::Space,
                ctrl: false,
                shift: false,
                alt: false,
            },
            VimAction::Motion(VimMotion::Right),
        );

        visual.insert('y'.into(), VimAction::Operator(VimOperator::Yank));
        visual.insert('Y'.into(), VimAction::Operator(VimOperator::Yank));
        visual.insert('d'.into(), VimAction::Operator(VimOperator::Delete));
        visual.insert('D'.into(), VimAction::Operator(VimOperator::Delete));
        visual.insert('x'.into(), VimAction::Operator(VimOperator::Delete));
        visual.insert('c'.into(), VimAction::Operator(VimOperator::Change));
        visual.insert('C'.into(), VimAction::Operator(VimOperator::Change));
        visual.insert(
            KeyStroke::Key {
                key: Key::C,
                ctrl: true,
                shift: false,
                alt: false,
            },
            VimAction::Operator(VimOperator::Yank),
        );
        visual.insert(
            KeyStroke::Key {
                key: Key::X,
                ctrl: true,
                shift: true,
                alt: false,
            },
            VimAction::ToggleTaskCheckbox,
        );

        combinations.insert(('g', 'g'), VimAction::Motion(VimMotion::BufferStart));

        Self {
            normal,
            visual,
            operator_combinations,
            combinations,
        }
    }

    /// Resolves a character or key stroke in Normal mode.
    pub fn lookup_normal(&self, stroke: &KeyStroke) -> Option<VimAction> {
        self.normal.get(stroke).cloned()
    }

    /// Resolves a character or key stroke in Visual mode.
    pub fn lookup_visual(&self, stroke: &KeyStroke) -> Option<VimAction> {
        self.visual.get(stroke).cloned()
    }

    /// Resolves a pending operator with a second key stroke (e.g. `d` + `d`).
    pub fn lookup_operator(&self, op: VimOperator, c: char) -> Option<VimAction> {
        self.operator_combinations.get(&(op, c)).cloned()
    }

    /// Maps a two-character prefix sequence (e.g. `('g', 'g')`) to an action.
    pub fn lookup_combination(&self, prefix: char, second: char) -> Option<VimAction> {
        self.combinations.get(&(prefix, second)).cloned()
    }

    /// Returns true if `c` is the first character of any registered combination.
    pub fn is_combination_prefix(&self, c: char) -> bool {
        self.combinations.keys().any(|(p, _)| *p == c)
    }

    /// Customizes or remaps a key binding in Normal mode.
    #[allow(dead_code)]
    pub fn bind_normal(&mut self, stroke: KeyStroke, action: VimAction) {
        self.normal.insert(stroke, action);
    }

    /// Customizes or remaps a key binding in Visual mode.
    #[allow(dead_code)]
    pub fn bind_visual(&mut self, stroke: KeyStroke, action: VimAction) {
        self.visual.insert(stroke, action);
    }

    /// Resolves the filesystem path to the user's `keymap.json` file.
    pub fn get_keymap_path() -> std::path::PathBuf {
        if let Some(proj) = directories::ProjectDirs::from("com", "mindforge", "mindforge") {
            proj.config_dir().join("keymap.json")
        } else {
            std::path::PathBuf::from("keymap.json")
        }
    }

    /// Loads custom keybindings from `keymap.json` if it exists, or creates the default template.
    pub fn load_or_init() -> Self {
        let mut keymap = Self::new_standard();
        let path = Self::get_keymap_path();

        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<KeymapConfigFile>(&content) {
                    for (k, v) in cfg.normal {
                        if let (Some(stroke), Some(act)) = (parse_key_stroke(&k), parse_action(&v))
                        {
                            keymap.normal.insert(stroke, act);
                        }
                    }
                    for (k, v) in cfg.visual {
                        if let (Some(stroke), Some(act)) = (parse_key_stroke(&k), parse_action(&v))
                        {
                            keymap.visual.insert(stroke, act);
                        }
                    }
                }
            }
        } else {
            // Write default keymap.json so user can easily inspect and edit their keys
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let template = generate_default_keymap_json();
            let _ = std::fs::write(&path, template);
        }

        keymap
    }

    /// Saves current keybindings to `keymap.json`.
    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = Self::get_keymap_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let cfg = KeymapConfigFile {
            description: Some("MindForge Vim Keybindings. Edit this file to customize your keys, or use Settings > Keybindings in the app.".into()),
            normal: self.normal.iter().map(|(k, v)| (key_stroke_to_string(k), action_to_string(v))).collect(),
            visual: self.visual.iter().map(|(k, v)| (key_stroke_to_string(k), action_to_string(v))).collect(),
        };
        let json = serde_json::to_string_pretty(&cfg)?;
        std::fs::write(&path, json)
    }

    /// Rebinds one action's key in the given mode and immediately persists.
    pub fn rebind(
        &mut self,
        mode: KeymapMode,
        old: Option<&KeyStroke>,
        new: KeyStroke,
        action: VimAction,
    ) {
        let map = match mode {
            KeymapMode::Normal => &mut self.normal,
            KeymapMode::Visual => &mut self.visual,
        };
        if let Some(old) = old {
            map.remove(old);
        }
        map.insert(new, action);
        let _ = self.save_to_file();
    }

    /// Unbinds a key stroke in the given mode and immediately persists.
    pub fn unbind(&mut self, mode: KeymapMode, stroke: &KeyStroke) {
        let map = match mode {
            KeymapMode::Normal => &mut self.normal,
            KeymapMode::Visual => &mut self.visual,
        };
        map.remove(stroke);
        let _ = self.save_to_file();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeymapMode {
    Normal,
    Visual,
}

/// Which row (if any) is currently waiting for a keypress to bind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeybindCapture {
    pub mode: KeymapMode,
    pub action: VimAction,
    pub action_label: String,
    pub old_stroke: Option<KeyStroke>,
    pub staged_stroke: Option<KeyStroke>,
}

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct KeymapConfigFile {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub normal: HashMap<String, String>,
    #[serde(default)]
    pub visual: HashMap<String, String>,
}

pub fn key_stroke_to_string(stroke: &KeyStroke) -> String {
    match stroke {
        KeyStroke::Char(c) => c.to_string(),
        KeyStroke::Key {
            key,
            ctrl,
            shift,
            alt,
        } => {
            let mut parts = vec![];
            if *ctrl {
                parts.push("ctrl");
            }
            if *shift {
                parts.push("shift");
            }
            if *alt {
                parts.push("alt");
            }
            parts.push(key_name(*key));
            parts.join("+")
        }
    }
}

pub fn key_name(key: Key) -> &'static str {
    match key {
        Key::Enter => "enter",
        Key::Space => "space",
        Key::Backspace => "backspace",
        Key::Delete => "delete",
        Key::ArrowLeft => "left",
        Key::ArrowRight => "right",
        Key::ArrowUp => "up",
        Key::ArrowDown => "down",
        Key::Home => "home",
        Key::End => "end",
        Key::A => "a",
        Key::B => "b",
        Key::C => "c",
        Key::D => "d",
        Key::E => "e",
        Key::F => "f",
        Key::G => "g",
        Key::H => "h",
        Key::I => "i",
        Key::J => "j",
        Key::K => "k",
        Key::L => "l",
        Key::M => "m",
        Key::N => "n",
        Key::O => "o",
        Key::P => "p",
        Key::Q => "q",
        Key::R => "r",
        Key::S => "s",
        Key::T => "t",
        Key::U => "u",
        Key::V => "v",
        Key::W => "w",
        Key::X => "x",
        Key::Y => "y",
        Key::Z => "z",
        _ => "?",
    }
}

/// Human-readable form for display in the settings UI, e.g. "Ctrl+Shift+X".
pub fn key_stroke_display(stroke: &KeyStroke) -> String {
    match stroke {
        KeyStroke::Char(c) => c.to_string(),
        KeyStroke::Key {
            key,
            ctrl,
            shift,
            alt,
        } => {
            let mut parts = vec![];
            if *ctrl {
                parts.push("Ctrl".to_string());
            }
            if *shift {
                parts.push("Shift".to_string());
            }
            if *alt {
                parts.push("Alt".to_string());
            }
            parts.push(key_name(*key).to_uppercase());
            parts.join("+")
        }
    }
}

pub fn action_to_string(action: &VimAction) -> String {
    match action {
        VimAction::Motion(VimMotion::Left) => "left",
        VimAction::Motion(VimMotion::Right) => "right",
        VimAction::Motion(VimMotion::UpVisual) => "up",
        VimAction::Motion(VimMotion::DownVisual) => "down",
        VimAction::Motion(VimMotion::WordForward) => "wordforward",
        VimAction::Motion(VimMotion::WordBackward) => "wordbackward",
        VimAction::Motion(VimMotion::LineStart) => "linestart",
        VimAction::Motion(VimMotion::LineEnd) => "lineend",
        VimAction::Motion(VimMotion::BufferStart) => "bufferstart",
        VimAction::Motion(VimMotion::BufferEnd) => "bufferend",
        VimAction::DeleteChar => "deletechar",
        VimAction::Undo => "undo",
        VimAction::Redo => "redo",
        VimAction::Paste { before: false } => "paste",
        VimAction::Paste { before: true } => "pastebefore",
        VimAction::EnterInsert(InsertPosition::AtCursor) => "insert",
        VimAction::EnterInsert(InsertPosition::AfterCursor) => "append",
        VimAction::EnterInsert(InsertPosition::LineStart) => "insertlinestart",
        VimAction::EnterInsert(InsertPosition::LineEnd) => "appendlineend",
        VimAction::EnterInsert(InsertPosition::LineBelow) => "openbelow",
        VimAction::EnterInsert(InsertPosition::LineAbove) => "openabove",
        VimAction::EnterVisual { is_line: false } => "visual",
        VimAction::EnterVisual { is_line: true } => "visualline",
        VimAction::EnterSearch { backward: false } => "search",
        VimAction::EnterSearch { backward: true } => "searchbackward",
        VimAction::RepeatSearch { reverse: false } => "repeatsearch",
        VimAction::RepeatSearch { reverse: true } => "repeatsearchbackward",
        VimAction::Operator(VimOperator::Delete) => "delete",
        VimAction::Operator(VimOperator::Yank) => "yank",
        VimAction::Operator(VimOperator::Change) => "change",
        VimAction::DuplicateLine => "duplicateline",
        VimAction::ToggleTaskCheckbox => "toggletaskcheckbox",
        _ => "unknown",
    }
    .to_string()
}

fn parse_key_name(s: &str) -> Option<Key> {
    match s {
        "enter" => Some(Key::Enter),
        "space" => Some(Key::Space),
        "backspace" => Some(Key::Backspace),
        "delete" => Some(Key::Delete),
        "left" => Some(Key::ArrowLeft),
        "right" => Some(Key::ArrowRight),
        "up" => Some(Key::ArrowUp),
        "down" => Some(Key::ArrowDown),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "a" => Some(Key::A),
        "b" => Some(Key::B),
        "c" => Some(Key::C),
        "d" => Some(Key::D),
        "e" => Some(Key::E),
        "f" => Some(Key::F),
        "g" => Some(Key::G),
        "h" => Some(Key::H),
        "i" => Some(Key::I),
        "j" => Some(Key::J),
        "k" => Some(Key::K),
        "l" => Some(Key::L),
        "m" => Some(Key::M),
        "n" => Some(Key::N),
        "o" => Some(Key::O),
        "p" => Some(Key::P),
        "q" => Some(Key::Q),
        "r" => Some(Key::R),
        "s" => Some(Key::S),
        "t" => Some(Key::T),
        "u" => Some(Key::U),
        "v" => Some(Key::V),
        "w" => Some(Key::W),
        "x" => Some(Key::X),
        "y" => Some(Key::Y),
        "z" => Some(Key::Z),
        _ => None,
    }
}

pub fn parse_key_stroke(s: &str) -> Option<KeyStroke> {
    let trimmed = s.trim();
    if trimmed.chars().count() == 1 {
        return Some(KeyStroke::Char(trimmed.chars().next().unwrap()));
    }
    let lower = trimmed.to_lowercase();
    let parts: Vec<&str> = lower.split('+').map(|p| p.trim()).collect();
    if parts.len() > 1 {
        let mut ctrl = false;
        let mut shift = false;
        let mut alt = false;
        let mut key_str = "";
        for part in &parts {
            match *part {
                "ctrl" => ctrl = true,
                "shift" => shift = true,
                "alt" => alt = true,
                other => key_str = other,
            }
        }
        if let Some(key) = parse_key_name(key_str) {
            return Some(KeyStroke::Key {
                key,
                ctrl,
                shift,
                alt,
            });
        }
    } else if let Some(key) = parse_key_name(&lower) {
        return Some(KeyStroke::Key {
            key,
            ctrl: false,
            shift: false,
            alt: false,
        });
    }
    None
}

pub fn parse_action(s: &str) -> Option<VimAction> {
    match s.trim().to_lowercase().as_str() {
        "left" => Some(VimAction::Motion(VimMotion::Left)),
        "right" => Some(VimAction::Motion(VimMotion::Right)),
        "up" | "upvisual" => Some(VimAction::Motion(VimMotion::UpVisual)),
        "down" | "downvisual" => Some(VimAction::Motion(VimMotion::DownVisual)),
        "wordforward" => Some(VimAction::Motion(VimMotion::WordForward)),
        "wordbackward" => Some(VimAction::Motion(VimMotion::WordBackward)),
        "linestart" => Some(VimAction::Motion(VimMotion::LineStart)),
        "lineend" => Some(VimAction::Motion(VimMotion::LineEnd)),
        "bufferstart" => Some(VimAction::Motion(VimMotion::BufferStart)),
        "bufferend" => Some(VimAction::Motion(VimMotion::BufferEnd)),
        "deletechar" => Some(VimAction::DeleteChar),
        "undo" => Some(VimAction::Undo),
        "redo" => Some(VimAction::Redo),
        "paste" | "pasteafter" => Some(VimAction::Paste { before: false }),
        "pastebefore" => Some(VimAction::Paste { before: true }),
        "insert" => Some(VimAction::EnterInsert(InsertPosition::AtCursor)),
        "append" => Some(VimAction::EnterInsert(InsertPosition::AfterCursor)),
        "insertlinestart" => Some(VimAction::EnterInsert(InsertPosition::LineStart)),
        "appendlineend" => Some(VimAction::EnterInsert(InsertPosition::LineEnd)),
        "openbelow" => Some(VimAction::EnterInsert(InsertPosition::LineBelow)),
        "openabove" => Some(VimAction::EnterInsert(InsertPosition::LineAbove)),
        "visual" => Some(VimAction::EnterVisual { is_line: false }),
        "visualline" => Some(VimAction::EnterVisual { is_line: true }),
        "search" => Some(VimAction::EnterSearch { backward: false }),
        "searchbackward" => Some(VimAction::EnterSearch { backward: true }),
        "repeatsearch" => Some(VimAction::RepeatSearch { reverse: false }),
        "repeatsearchbackward" => Some(VimAction::RepeatSearch { reverse: true }),
        "delete" => Some(VimAction::Operator(VimOperator::Delete)),
        "yank" => Some(VimAction::Operator(VimOperator::Yank)),
        "change" => Some(VimAction::Operator(VimOperator::Change)),
        "duplicateline" => Some(VimAction::DuplicateLine),
        "toggletaskcheckbox" => Some(VimAction::ToggleTaskCheckbox),
        _ => None,
    }
}

fn generate_default_keymap_json() -> String {
    r#"{
  "description": "MindForge Vim Keybindings. Edit this file to customize your keys. Restart or reload to apply.",
  "normal": {
    "h": "Left",
    "j": "DownVisual",
    "k": "UpVisual",
    "l": "Right",
    "w": "WordForward",
    "b": "WordBackward",
    "0": "LineStart",
    "$": "LineEnd",
    "G": "BufferEnd",
    "x": "DeleteChar",
    "u": "Undo",
    "p": "Paste",
    "P": "PasteBefore",
    "i": "Insert",
    "a": "Append",
    "I": "InsertLineStart",
    "A": "AppendLineEnd",
    "o": "OpenBelow",
    "O": "OpenAbove",
    "v": "Visual",
    "V": "VisualLine",
    "/": "Search",
    "?": "SearchBackward",
    "n": "RepeatSearch",
    "N": "RepeatSearchBackward"
  },
  "visual": {
    "h": "Left",
    "j": "DownVisual",
    "k": "UpVisual",
    "l": "Right",
    "w": "WordForward",
    "b": "WordBackward",
    "0": "LineStart",
    "$": "LineEnd",
    "G": "BufferEnd",
    "y": "Yank",
    "d": "Delete",
    "x": "Delete"
  }
}
"#.to_string()
}

/// Helper mapping delimiter character to `TextObjectKind`.
pub fn char_to_text_object_kind(c: char) -> Option<TextObjectKind> {
    match c {
        '"' => Some(TextObjectKind::DoubleQuote),
        '\'' => Some(TextObjectKind::SingleQuote),
        '`' => Some(TextObjectKind::Backtick),
        '(' | ')' | 'b' => Some(TextObjectKind::Parentheses),
        '{' | '}' | 'B' => Some(TextObjectKind::Braces),
        '[' | ']' => Some(TextObjectKind::Brackets),
        '<' | '>' => Some(TextObjectKind::AngleBrackets),
        'w' => Some(TextObjectKind::Word),
        'p' => Some(TextObjectKind::Paragraph),
        _ => None,
    }
}
