use super::{Editor, VisualLine};

impl Editor {
    pub fn compute_visual_lines(&self, max_cols: usize) -> Vec<VisualLine> {
        if self.buf.is_empty() {
            return vec![VisualLine { char_start: 0, char_end: 0 }];
        }

        let max_cols = max_cols.max(15);
        let mut lines = Vec::new();
        let mut line_start = 0;

        while line_start <= self.buf.len() {
            let mut line_end = line_start;
            while line_end < self.buf.len() && self.buf[line_end] != '\n' {
                line_end += 1;
            }

            let physical_len = line_end - line_start;
            if physical_len == 0 {
                lines.push(VisualLine {
                    char_start: line_start,
                    char_end: line_end,
                });
            } else {
                let mut chunk_start = line_start;
                while chunk_start < line_end {
                    let remaining = line_end - chunk_start;
                    if remaining <= max_cols {
                        lines.push(VisualLine {
                            char_start: chunk_start,
                            char_end: line_end,
                        });
                        break;
                    }

                    // Look for whitespace wrap point within max_cols
                    let limit = chunk_start + max_cols;
                    let mut wrap_at = None;
                    for idx in (chunk_start + 1..=limit).rev() {
                        if self.buf[idx - 1].is_whitespace() {
                            wrap_at = Some(idx);
                            break;
                        }
                    }

                    let chunk_end = wrap_at.unwrap_or(limit);
                    lines.push(VisualLine {
                        char_start: chunk_start,
                        char_end: chunk_end,
                    });

                    chunk_start = chunk_end;
                    while chunk_start < line_end && self.buf[chunk_start] == ' ' {
                        chunk_start += 1;
                    }
                }
            }

            if line_end >= self.buf.len() {
                break;
            }
            line_start = line_end + 1;
        }

        if lines.is_empty() {
            lines.push(VisualLine { char_start: 0, char_end: 0 });
        }

        lines
    }

    pub fn visual_row_col(&self, lines: &[VisualLine]) -> (usize, usize) {
        if lines.is_empty() {
            return (0, 0);
        }
        for (i, line) in lines.iter().enumerate() {
            if self.cur >= line.char_start && self.cur < line.char_end {
                return (i, self.cur - line.char_start);
            }
            if self.cur == line.char_end {
                if i + 1 == lines.len() || lines[i + 1].char_start > line.char_end {
                    return (i, self.cur - line.char_start);
                }
            }
            if i + 1 < lines.len() && self.cur >= line.char_end && self.cur < lines[i + 1].char_start {
                return (i, self.cur - line.char_start);
            }
        }
        let last_idx = lines.len() - 1;
        let last = &lines[last_idx];
        (last_idx, self.cur.saturating_sub(last.char_start))
    }

    pub fn up_visual(&mut self, lines: &[VisualLine]) {
        if lines.is_empty() {
            self.up();
            return;
        }
        self.selection = None;
        let (row, col) = self.visual_row_col(lines);
        if row == 0 {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);

        let mut target_row = row - 1;
        while target_row > 0
            && lines[target_row].char_start == self.cur
            && lines[target_row].char_end == self.cur
        {
            target_row -= 1;
        }
        let target_line = &lines[target_row];
        let line_len = target_line.char_end.saturating_sub(target_line.char_start);
        let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
        let max_col = if is_soft_wrapped {
            line_len.saturating_sub(1)
        } else {
            line_len
        };
        self.cur = target_line.char_start + target_col.min(max_col);
    }

    pub fn up_visual_select(&mut self, lines: &[VisualLine]) {
        if lines.is_empty() {
            self.up_select();
            return;
        }
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.visual_row_col(lines);
        if row == 0 {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);

        let mut target_row = row - 1;
        while target_row > 0
            && lines[target_row].char_start == self.cur
            && lines[target_row].char_end == self.cur
        {
            target_row -= 1;
        }
        let target_line = &lines[target_row];
        let line_len = target_line.char_end.saturating_sub(target_line.char_start);
        let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
        let max_col = if is_soft_wrapped {
            line_len.saturating_sub(1)
        } else {
            line_len
        };
        self.cur = target_line.char_start + target_col.min(max_col);
    }

    pub fn down_visual(&mut self, lines: &[VisualLine]) {
        if lines.is_empty() {
            self.down();
            return;
        }
        self.selection = None;
        let (row, col) = self.visual_row_col(lines);
        if row + 1 >= lines.len() {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);

        let target_line = &lines[row + 1];
        let line_len = target_line.char_end.saturating_sub(target_line.char_start);
        let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
        let max_col = if is_soft_wrapped {
            line_len.saturating_sub(1)
        } else {
            line_len
        };
        self.cur = target_line.char_start + target_col.min(max_col);
    }

    pub fn down_visual_select(&mut self, lines: &[VisualLine]) {
        if lines.is_empty() {
            self.down_select();
            return;
        }
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.visual_row_col(lines);
        if row + 1 >= lines.len() {
            return;
        }
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);

        let target_line = &lines[row + 1];
        let line_len = target_line.char_end.saturating_sub(target_line.char_start);
        let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
        let max_col = if is_soft_wrapped {
            line_len.saturating_sub(1)
        } else {
            line_len
        };
        self.cur = target_line.char_start + target_col.min(max_col);
    }

    pub fn home_visual(&mut self, lines: &[VisualLine]) {
        self.desired_col = None;
        self.selection = None;
        let (row, _) = self.visual_row_col(lines);
        if let Some(line) = lines.get(row) {
            self.cur = line.char_start;
        }
    }

    pub fn home_visual_select(&mut self, lines: &[VisualLine]) {
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, _) = self.visual_row_col(lines);
        if let Some(line) = lines.get(row) {
            self.cur = line.char_start;
        }
    }

    pub fn end_visual(&mut self, lines: &[VisualLine]) {
        self.desired_col = None;
        self.selection = None;
        self.selection_inclusive = false;
        let (row, _) = self.visual_row_col(lines);
        if let Some(line) = lines.get(row) {
            let is_soft_wrapped = line.char_end < self.buf.len() && self.buf[line.char_end] != '\n';
            if is_soft_wrapped && line.char_end > line.char_start {
                let mut target = line.char_end - 1;
                while target > line.char_start && self.buf[target].is_whitespace() {
                    target -= 1;
                }
                self.cur = target;
            } else {
                self.cur = line.char_end;
            }
        }
    }

    pub fn end_visual_select(&mut self, lines: &[VisualLine]) {
        self.desired_col = None;
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, _) = self.visual_row_col(lines);
        if let Some(line) = lines.get(row) {
            self.cur = line.char_end;
        }
    }

    pub fn page_up_visual(&mut self, lines: &[VisualLine], count: usize) {
        self.selection = None;
        let (row, col) = self.visual_row_col(lines);
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        let target_row = row.saturating_sub(count);
        if let Some(target_line) = lines.get(target_row) {
            let line_len = target_line.char_end.saturating_sub(target_line.char_start);
            let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
            let max_col = if is_soft_wrapped {
                line_len.saturating_sub(1)
            } else {
                line_len
            };
            self.cur = target_line.char_start + target_col.min(max_col);
        }
    }

    pub fn page_up_visual_select(&mut self, lines: &[VisualLine], count: usize) {
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.visual_row_col(lines);
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        let target_row = row.saturating_sub(count);
        if let Some(target_line) = lines.get(target_row) {
            let line_len = target_line.char_end.saturating_sub(target_line.char_start);
            let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
            let max_col = if is_soft_wrapped {
                line_len.saturating_sub(1)
            } else {
                line_len
            };
            self.cur = target_line.char_start + target_col.min(max_col);
        }
    }

    pub fn page_down_visual(&mut self, lines: &[VisualLine], count: usize) {
        self.selection = None;
        let (row, col) = self.visual_row_col(lines);
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        let target_row = (row + count).min(lines.len().saturating_sub(1));
        if let Some(target_line) = lines.get(target_row) {
            let line_len = target_line.char_end.saturating_sub(target_line.char_start);
            let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
            let max_col = if is_soft_wrapped {
                line_len.saturating_sub(1)
            } else {
                line_len
            };
            self.cur = target_line.char_start + target_col.min(max_col);
        }
    }

    pub fn page_down_visual_select(&mut self, lines: &[VisualLine], count: usize) {
        if self.selection.is_none() {
            self.selection = Some(self.cur);
        }
        let (row, col) = self.visual_row_col(lines);
        let target_col = self.desired_col.unwrap_or(col);
        self.desired_col = Some(target_col);
        let target_row = (row + count).min(lines.len().saturating_sub(1));
        if let Some(target_line) = lines.get(target_row) {
            let line_len = target_line.char_end.saturating_sub(target_line.char_start);
            let is_soft_wrapped = target_line.char_end < self.buf.len() && self.buf[target_line.char_end] != '\n';
            let max_col = if is_soft_wrapped {
                line_len.saturating_sub(1)
            } else {
                line_len
            };
            self.cur = target_line.char_start + target_col.min(max_col);
        }
    }
}

/// Mode-aware caret cell index within `line`.
///
/// - Normal / Visual / VisualLine: block sits ON the character. At end-of-line,
///   clamps to the last character so it never floats past it.
/// - Insert / Search / None: bar sits BETWEEN characters. At end-of-line it's
///   one cell past the last character.
/// - Empty line: 0 in every mode.
pub fn caret_cell(cur: usize, line: &VisualLine, _vim_mode: Option<&str>) -> usize {
    let line_len = line.char_end.saturating_sub(line.char_start);
    if line_len == 0 {
        return 0;
    }
    cur.saturating_sub(line.char_start).min(line_len)
}
