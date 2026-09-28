use super::types::EditorMode;

/// Central selection point for an editor backend. The owner is responsible for
/// constructing a replacement before switching, so a failed startup cannot
/// discard the currently open document.
pub struct EditorController {
    pub mode: EditorMode,
    /// Mode observed at the start of the previous egui frame; used to detect
    /// one-way transitions without comparing full document buffers each frame.
    pub last_observed_mode: EditorMode,
}

impl EditorController {
    pub fn new(mode: EditorMode) -> Self {
        Self {
            mode,
            last_observed_mode: mode,
        }
    }
    pub fn mode(&self) -> EditorMode {
        self.mode
    }
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
    }

}
