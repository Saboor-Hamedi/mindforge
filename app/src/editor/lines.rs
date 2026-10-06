use super::Editor;

impl Editor {
    pub fn insert_line_below(&mut self) {
        self.save_undo_snapshot();
        self.end();
        self.in_typing_burst = false;
        self.buf.insert(self.cur, '\n');
        self.cur += 1;
        self.selection = None;
        self.desired_col = None;
        self.mark_dirty();
    }

    pub fn current_line_span(&self) -> (usize, usize) {
        if self.buf.is_empty() {
            return (0, 0);
        }
        let cur = self.cur.min(self.buf.len());
        let mut start = cur;
        while start > 0 && self.buf[start - 1] != '\n' {
            start -= 1;
        }
        let mut end = cur;
        while end < self.buf.len() && self.buf[end] != '\n' {
            end += 1;
        }
        if end < self.buf.len() && self.buf[end] == '\n' {
            end += 1;
        }
        (start, end)
    }

    pub fn duplicate_line(&mut self) {
        self.save_undo_snapshot();
        let (start, end) = self.current_line_span();
        let line_chars: Vec<char> = self.buf[start..end].to_vec();
        let has_newline = line_chars.last() == Some(&'\n');

        let insert_pos = end;
        if !has_newline {
            self.buf.insert(insert_pos, '\n');
            let mut next_pos = insert_pos + 1;
            for c in line_chars {
                self.buf.insert(next_pos, c);
                next_pos += 1;
            }
            self.cur = next_pos;
        } else {
            let mut next_pos = insert_pos;
            for c in line_chars {
                self.buf.insert(next_pos, c);
                next_pos += 1;
            }
            self.cur = next_pos.saturating_sub(1);
        }
        self.selection = None;
    }

    pub fn move_line_up(&mut self) {
        let (cur_start, cur_end) = self.current_line_span();
        if cur_start == 0 {
            return;
        }
        self.save_undo_snapshot();
        let prev_end = cur_start;
        let mut prev_start = prev_end.saturating_sub(1);
        while prev_start > 0 && self.buf[prev_start - 1] != '\n' {
            prev_start -= 1;
        }
        let offset = self.cur.saturating_sub(cur_start);

        let cur_line: Vec<char> = self.buf.drain(cur_start..cur_end).collect();
        let mut insert_idx = prev_start;
        for c in cur_line {
            self.buf.insert(insert_idx, c);
            insert_idx += 1;
        }
        self.cur = (prev_start + offset).min(self.buf.len());
        self.selection = None;
    }

    pub fn move_line_down(&mut self) {
        let (cur_start, cur_end) = self.current_line_span();
        if cur_end >= self.buf.len() {
            return;
        }
        self.save_undo_snapshot();
        let next_start = cur_end;
        let mut next_end = next_start;
        while next_end < self.buf.len() && self.buf[next_end] != '\n' {
            next_end += 1;
        }
        if next_end < self.buf.len() && self.buf[next_end] == '\n' {
            next_end += 1;
        }
        let offset = self.cur.saturating_sub(cur_start);

        let next_line: Vec<char> = self.buf.drain(next_start..next_end).collect();
        let mut insert_idx = cur_start;
        for c in next_line {
            self.buf.insert(insert_idx, c);
            insert_idx += 1;
        }
        self.cur = (insert_idx + offset).min(self.buf.len());
        self.selection = None;
    }

    pub fn delete_line(&mut self) -> String {
        self.save_undo_snapshot();
        let (start, end) = self.current_line_span();
        let line_str: String = self.buf[start..end].iter().collect();
        self.buf.drain(start..end);
        self.cur = start.min(self.buf.len());
        self.selection = None;
        line_str
    }

    pub fn yank_line(&self) -> String {
        let (start, end) = self.current_line_span();
        self.buf[start..end].iter().collect()
    }
}
