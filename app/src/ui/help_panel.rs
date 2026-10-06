//! Quick Start Guide & Interactive Help Center.
//!
//! Provides unified guidance for notes, Vim motions, commands, carets,
//! and keyboard shortcuts in a single clean tab rendered via the Markdown
//! Preview engine. Content is embedded as a Markdown string constant.
//!
//! The help panel is opened via F1 or `:help` and displays a comprehensive
//! quick-start guide covering all major features.

use eframe::egui::{self, Rect};

/// Returned by the help panel when the user requests an action.
pub struct HelpPanelAction {
    pub should_close: bool,
}

pub const QUICK_START_GUIDE_MD: &str = r#"# ⚡ MindForge Quick Start Guide

Welcome to **MindForge**, an ultra-fast, local-first markdown notebook with living animated carets, rich live preview, authentic mechanical switch typing acoustics, native Vim motions, and embedded Linux/Bash terminal sessions.

---

## 1. Writing & Managing Notes

- **Auto-Save**: Notes are continuously saved to disk without manual effort.
- `Ctrl + N`: Instantly creates a new blank document ready for typing.
- `Ctrl + S`: Executes an immediate manual save to disk and resets dirty status.
- `Ctrl + R`: Opens the modal to rename the active document.
- `Ctrl + B`: Toggles the left-hand navigation sidebar showing your notes list.
- `Ctrl + \`: Toggles the real-time Markdown Live Preview side-by-side.
- `Ctrl + J`: Toggles the docked Linux/Bash terminal dock underneath the editor and preview.

---

## 2. Hybrid vs. Vim Modes

- **Hybrid Mode**: Modern editor experience with intuitive hotkeys, smooth cursor glide, and auto-pairing for brackets and quotes.
- **Vim Mode**: Pure home-row modal editing with Normal, Insert, Visual, and Command lines.
- **Switching Modes**: Type `:vim` or `:mode vim` / `:mode hybrid` in the command bar to switch anytime.
- **Living Carets in Vim**: Your custom animated caret stays uniform and visible across all Vim submodes.

---

## 3. Living Animated Carets & Typing Sounds

- **9 Animated Carets**: Choose from Candle, Fire, Water, Snow, Neon, Rainbow, Block, Beam, and Underline.
- **Ambient Life**: Water drips into baseline ripples, candle flickers gently, fire micro-embers dance.
- **Mechanical Sound Profiles**: Authentic synthesized switch audio: Thocky, Clacky, Creamy, Marbly, and Poppy.
- **Preferences**: Press `Ctrl + ,` to open Settings and customize carets, themes, and audio.

---

## 4. Home-Row Vim Motions & Movements

| Key | Motion Description |
| :--- | :--- |
| `h` / `j` / `k` / `l` | Left, Down, Up, and Right precision cursor navigation |
| `w` / `b` / `e` | Jump forward to next word (`w`), backward to word start (`b`), or end of word (`e`) |
| `0` / `$` | Jump to line start (`0`) or visual line end (`$`) |
| `gg` / `G` | Jump to beginning (`gg`) or end (`G`) of active document |
| `40j` / `10k` | Motion multipliers: jump 40 lines down or 10 lines up |
| `dd` / `dw` | Delete entire current line (`dd`) or delete to next word (`dw`) |
| `x` | Delete character under cursor |
| `u` / `Ctrl + R` | Undo (`u`) and Redo (`Ctrl + R`) |

---

## 5. Text Objects & Editing Chords

Text objects allow surgical edits inside or around delimiters without needing visual selection first:

- `ci"` / `ca"`: Change inside quotes or around quotes. Deletes text and enters Insert mode.
- `di"` / `da"`: Delete inside quotes or around quotes.
- `ci(` / `ca(`: Change inside parentheses (`ci(`) or around parentheses (`ca(`).
- `di(` / `da(`: Delete inside parentheses or around parentheses.
- `vi[` / `va[`: Visually select inside brackets or select enclosing brackets.
- `c` / `d` / `y`: Operator pending chords: change (`c`), delete (`d`), or yank/copy (`y`).

---

## 6. Floating ShowCmd HUD & Search

- **ShowCmd HUD**: A live floating pill in the bottom-right tracks pending keys (`VIM`), visual ranges (`VIS`), search matches (`FIND`), and commands (`CMD`).
- `:set showcmd`: Enables the floating keystroke & operator HUD card.
- `:set noshowcmd`: Disables the floating HUD (aliases: `:set nonshowcmd`, `:noshowcmd`).
- `/pattern`: Buffer search forward. Press `n` for next match, `N` for previous match.
- `?pattern`: Buffer search backward.

---

## 7. Command Palette Reference (`:`)

Type `:` in Normal mode to open the command bar at the bottom of the screen:

| Command | Action |
| :--- | :--- |
| `:w` / `:save` | Save active document immediately to disk |
| `:r <title>` | Rename the active document to a new title |
| `:d` / `:delete` | Open delete confirmation modal to remove active note |
| `:doc` / `:docs` | Open built-in documentation and guide reader |
| `:editor` | Return from documentation or stats back to notes editor |
| `:stats` | Open daily writing story, activity heatmap, and lifetime metrics |
| `:clear` | Clear all text in active document editor |
| `:q` / `:quit` | Exit the MindForge application |
| `:set showcmd` | Enable the floating keystroke and command HUD capsule |
| `:set noshowcmd` | Disable the floating HUD |
| `:vim on` / `off` | Toggle between Vim modal engine and modern Hybrid IDE input |
| `:theme <name>` | Switch theme: `green`, `amber`, `blue`, `monokai`, `rose`, `purple` |
| `:sound <type>` | Change switch sounds: `thocky`, `clacky`, `creamy`, `marbly`, `poppy`, `clicky`, `off` |
| `:caret <kind>` | Set caret style: `candle`, `fire`, `water`, `snow`, `neon`, `rainbow`, `beam`, `block` |
| `:backup` | Trigger instant atomic backup snapshot of settings and workspace |
| `:export` / `:import` | Export note to markdown file, or import external `.md` / `.txt` file |

---

## 8. Complete Keyboard Shortcuts

| Shortcut | Description |
| :--- | :--- |
| `Ctrl + N` | Create new note |
| `Ctrl + S` | Save active document |
| `Ctrl + R` | Rename document |
| `Ctrl + D` | Duplicate current line below |
| `Ctrl + [` | Move text left (dedent line) |
| `Ctrl + ]` | Move text right (indent line) |
| `Ctrl + Z` | Undo |
| `Ctrl + Y` / `Ctrl + Shift + Z` | Redo |
| `Ctrl + A` | Select all text |
| `Ctrl + C` / `Ctrl + V` | Copy / Paste |
| `Ctrl + P` / `Ctrl + F` | Open fuzzy search across all notes |
| `Ctrl + B` | Toggle notes sidebar |
| `Ctrl + \` | Toggle real-time Markdown Live Preview side-by-side |
| `Ctrl + J` / `Ctrl + \`` | Toggle embedded Linux/Bash terminal dock |
| `Ctrl + ,` | Open Preferences & Settings modal |
| `F1` / `Ctrl + H` | Toggle Quick Start Guide tab |
| `Esc` | Release focus from terminal/sidebar, or return to Normal mode |
"#;

/// Renders the unified Quick Start Guide as a full-height reader tab inside the Editor/Doc panel,
/// matching the exact styling, margins, colors, and layout of the preview CSS without any redundant header.
pub fn render_help_tab_view(
    ui: &egui::Ui,
    painter: &egui::Painter,
    rect: Rect,
    scroll_y: &mut f32,
    theme: &crate::ui::theme::Theme,
    font_size: f32,
) -> HelpPanelAction {
    crate::view_editor::render_markdown_document(
        ui,
        painter,
        rect,
        QUICK_START_GUIDE_MD,
        scroll_y,
        theme,
        font_size,
    );

    HelpPanelAction {
        should_close: false,
    }
}
