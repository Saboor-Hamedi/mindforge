use super::{Editor, EditorSnapshot};

impl Editor {
    pub fn save_undo_snapshot(&mut self) {
        self.desired_col = None;
        let snap = EditorSnapshot {
            buf: self.buf.clone(),
            cur: self.cur,
        };
        if self.undo_stack.last() != Some(&snap) {
            self.undo_stack.push(snap);
            if self.undo_stack.len() > 300 {
                self.undo_stack.remove(0);
            }
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) -> bool {
        self.in_typing_burst = false;
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(EditorSnapshot {
                buf: self.buf.clone(),
                cur: self.cur,
            });
            self.buf = prev.buf;
            self.cur = prev.cur.min(self.buf.len());
            self.selection = None;
            self.desired_col = None;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        self.in_typing_burst = false;
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(EditorSnapshot {
                buf: self.buf.clone(),
                cur: self.cur,
            });
            self.buf = next.buf;
            self.cur = next.cur.min(self.buf.len());
            self.selection = None;
            self.desired_col = None;
            self.mark_dirty();
            true
        } else {
            false
        }
    }
}
