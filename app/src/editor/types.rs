//! Editor coordinates and cursor positions.

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorPosition {
    /// Zero-based line.
    pub row: usize,
    /// Zero-based UTF-8 byte column, as used by Neovim's grid protocol.
    pub column: usize,
}
