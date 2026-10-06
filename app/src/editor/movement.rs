use super::Editor;

impl Editor {
    pub fn left(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        if let Some((start, _)) = self.selected_range() {
            self.cur = start;
            self.selection = None;
        } else {
            self.cur = self.cur.saturating_sub(1);
            self.selection = None;
        }
    }

    pub fn right(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        if let Some((_, end)) = self.selected_range() {
            self.cur = end;
            self.selection = None;
        } else {
            self.cur = (self.cur + 1).min(self.buf.len());
            self.selection = None;
        }
    }

    pub fn left_select(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        self.cur = self.cur.saturating_sub(1);
    }

    pub fn right_select(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        self.cur = (self.cur + 1).min(self.buf.len());
    }

    #[allow(dead_code)]
    pub fn home(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        self.selection = None;
        while self.cur > 0 && self.buf[self.cur - 1] != '\n' {
            self.cur -= 1;
        }
    }

    #[allow(dead_code)]
    pub fn end(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        self.selection = None;
        while self.cur < self.buf.len() && self.buf[self.cur] != '\n' {
            self.cur += 1;
        }
    }

    #[allow(dead_code)]
    pub fn up(&mut self) {
        self.in_typing_burst = false;
        self.selection = None;
        let (row, col) = self.row_col();
        if row == 0 {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        self.set_row_col(row - 1, target_col);
    }

    pub fn up_select(&mut self) {
        self.in_typing_burst = false;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.row_col();
        if row == 0 {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        self.set_row_col(row - 1, target_col);
    }

    #[allow(dead_code)]
    pub fn down(&mut self) {
        self.in_typing_burst = false;
        self.selection = None;
        let (row, col) = self.row_col();
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        self.set_row_col(row + 1, target_col);
    }

    pub fn down_select(&mut self) {
        self.in_typing_burst = false;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.row_col();
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        self.set_row_col(row + 1, target_col);
    }

    pub fn set_row_col(&mut self, target_row: usize, target_col: usize) {
        self.in_typing_burst = false;
        let mut cur_row = 0;
        let mut line_start = 0;
        for (i, &c) in self.buf.iter().enumerate() {
            if cur_row == target_row {
                let mut col = 0;
                let mut idx = line_start;
                while idx < self.buf.len() && self.buf[idx] != '\n' && col < target_col {
                    idx += 1;
                    col += 1;
                }
                self.cur = idx;
                return;
            }
            if c == '\n' {
                cur_row += 1;
                line_start = i + 1;
            }
        }
        if cur_row == target_row {
            let mut col = 0;
            let mut idx = line_start;
            while idx < self.buf.len() && self.buf[idx] != '\n' && col < target_col {
                idx += 1;
                col += 1;
            }
            self.cur = idx;
        }
    }

    pub fn prev_word_boundary(&self, idx: usize) -> usize {
        let mut i = idx.min(self.buf.len());
        while i > 0 && self.buf[i - 1].is_whitespace() {
            i -= 1;
        }
        while i > 0 && !self.buf[i - 1].is_whitespace() {
            i -= 1;
        }
        i
    }

    pub fn next_word_boundary(&self, idx: usize) -> usize {
        let mut i = idx.min(self.buf.len());
        while i < self.buf.len() && !self.buf[i].is_whitespace() {
            i += 1;
        }
        while i < self.buf.len() && self.buf[i].is_whitespace() {
            i += 1;
        }
        i
    }

    pub fn word_left(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        self.cur = self.prev_word_boundary(self.cur);
        self.selection = None;
    }

    pub fn word_right(&mut self) {
        self.in_typing_burst = false;
        self.desired_col = None;
        self.cur = self.next_word_boundary(self.cur);
        self.selection = None;
    }

    pub fn word_left_select(&mut self) {
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        self.cur = self.prev_word_boundary(self.cur);
    }

    pub fn word_right_select(&mut self) {
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        self.cur = self.next_word_boundary(self.cur);
    }
}
