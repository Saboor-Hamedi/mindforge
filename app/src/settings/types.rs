//! Vim engine type definitions — enums and structures representing
//! modal states, operators, motions, text objects, and high-level editing actions.
//!
//! Designed to be completely decoupled from concrete keybindings so keymaps
//! can be dynamically customized or remapped by users in the future.

/// Active sub-mode in the Vim engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimSubMode {
    /// Normal navigation and command mode (e.g. `h`, `j`, `k`, `l`, `dd`, `yy`).
    Normal,
    /// Text insertion mode (typing characters directly into document).
    Insert,
    /// Character-wise visual selection mode.
    Visual,
    /// Line-wise visual selection mode (full line highlights).
    VisualLine,
    /// Interactive in-buffer search mode (`/` for forward, `?` for backward).
    Search {
        /// If true, searching backwards (`?`), otherwise forward (`/`).
        backward: bool,
    },
}

/// Standard Vim operators that act upon a motion or text object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VimOperator {
    /// Delete target text and copy to register (`d`).
    Delete,
    /// Yank (copy) target text to register (`y`).
    Yank,
    /// Delete target text, copy to register, and switch to Insert mode (`c`).
    Change,
}

/// Movement motions supported across Normal and Visual modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum VimMotion {
    /// Move left by characters (`h`).
    Left,
    /// Move right by characters (`l`).
    Right,
    /// Move up visually respecting soft-wrapped lines (`k`).
    UpVisual,
    /// Move down visually respecting soft-wrapped lines (`j`).
    DownVisual,
    /// Move forward to the next word boundary (`w`).
    WordForward,
    /// Move backward to the previous word boundary (`b`).
    WordBackward,
    /// Move to the start of the current visual line (`0`).
    LineStart,
    /// Move to the end of the current visual line (`$`).
    LineEnd,
    /// Jump to top of buffer (`gg`).
    BufferStart,
    /// Jump to bottom of buffer (`G`).
    BufferEnd,
}

/// Kinds of text objects supported by `i` (inner) and `a` (around).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextObjectKind {
    /// Double quotes `"..."`
    DoubleQuote,
    /// Single quotes `'...'`
    SingleQuote,
    /// Backticks `` `...` ``
    Backtick,
    /// Parentheses `(...)`
    Parentheses,
    /// Curly braces `{...}`
    Braces,
    /// Square brackets `[...]`
    Brackets,
    /// Angle brackets `<...>`
    AngleBrackets,
    /// Current word under or adjacent to cursor
    Word,
    /// Paragraph block (`p`)
    Paragraph,
}

/// Cursor destination when entering Insert mode from Normal mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsertPosition {
    /// Insert before cursor (`i`).
    AtCursor,
    /// Append after cursor (`a`).
    AfterCursor,
    /// Insert at beginning of line (`I`).
    LineStart,
    /// Append at end of line (`A`).
    LineEnd,
    /// Open new line below current line (`o`).
    LineBelow,
    /// Open new line above current line (`O`).
    LineAbove,
}

/// High-level decoupled actions generated from key strokes.
///
/// Decoupling actions from keys allows user-configurable keymaps in the future.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum VimAction {
    /// Execute a movement motion.
    Motion(VimMotion),
    /// Start a pending operator waiting for a motion or text object (`d`, `y`, `c`).
    Operator(VimOperator),
    /// Execute an operator on the entire current line (`dd`, `yy`, `cc`).
    OperatorLine(VimOperator),
    /// Execute an operator from cursor to end of current line (`D` = `d$`, `C` = `c$`).
    OperatorToEndOfLine(VimOperator),
    /// Execute an operation on a targeted text object (`di"`, `ci(`, `da{`, etc.).
    TextObject {
        op: VimOperator,
        inner: bool,
        kind: TextObjectKind,
    },
    /// Visual mode text object selection (`vi"`, `va(`, etc.).
    SelectTextObject {
        inner: bool,
        kind: TextObjectKind,
    },
    /// Switch to Insert mode at specified position.
    EnterInsert(InsertPosition),
    /// Switch to Visual or VisualLine mode.
    EnterVisual {
        is_line: bool,
    },
    /// Open in-buffer search mode.
    EnterSearch {
        backward: bool,
    },
    /// Repeat previous search in forward or backward direction (`n` / `N`).
    RepeatSearch {
        reverse: bool,
    },
    /// Single-character delete under cursor (`x`).
    DeleteChar,
    /// Undo last edit (`u`).
    Undo,
    /// Redo last undone edit (`Ctrl+R`).
    Redo,
    /// Paste register text (`p` for after, `P` for before).
    Paste {
        before: bool,
    },
    /// Duplicate line (`Ctrl+D`).
    DuplicateLine,
    /// Toggle task checkbox (`- [ ]` <-> `- [x]`) on current line or selection (`Ctrl+Shift+X`).
    ToggleTaskCheckbox,
    /// Cancel pending operation, search, or visual selection (returns to Normal mode).
    Cancel,
}
