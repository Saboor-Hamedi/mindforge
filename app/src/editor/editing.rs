use super::Editor;

impl Editor {
    pub fn insert(&mut self, c: char) {
        if self.has_selection() {
            self.delete_selection();
            self.in_typing_burst = true;
        } else if !self.in_typing_burst {
            self.save_undo_snapshot();
            self.in_typing_burst = true;
        }
        if c.is_whitespace() {
            self.in_typing_burst = false;
        }
        self.buf.insert(self.cur, c);
        self.cur += 1;
        self.selection = None;
    }

    #[allow(dead_code)]
    pub fn insert_str(&mut self, s: &str) {
        if self.has_selection() {
            self.delete_selection();
        } else {
            self.save_undo_snapshot();
        }
        self.in_typing_burst = false;
        let chars: Vec<char> = s.chars().filter(|&c| c != '\r').collect();
        let count = chars.len();
        let cur = self.cur.min(self.buf.len());
        self.buf.splice(cur..cur, chars);
        self.cur = cur + count;
        self.selection = None;
    }

    pub fn backspace(&mut self) {
        self.in_typing_burst = false;
        if self.delete_selection() {
            return;
        }
        if self.cur > 0 {
            self.save_undo_snapshot();

            // Remove an automatically paired delimiter in one Backspace when
            // the caret is between its opening and closing characters.
            let closes_pair = self.cur < self.buf.len()
                && matches!(
                    (self.buf[self.cur - 1], self.buf[self.cur]),
                    ('(', ')') | ('[', ']') | ('{', '}') | ('"', '"') | ('\'', '\'') | ('`', '`')
                );
            if closes_pair {
                self.buf.remove(self.cur);
                self.cur -= 1;
                self.buf.remove(self.cur);
                self.selection = None;
                return;
            }

            // Find current line start up to cursor
            let mut line_start = self.cur;
            while line_start > 0 && self.buf[line_start - 1] != '\n' {
                line_start -= 1;
            }
            let line_to_cur: String = self.buf[line_start..self.cur].iter().collect();

            // 1. If line_to_cur is only leading whitespace (e.g. 4 spaces or 2 spaces indent):
            // Dedent by up to 4 spaces (soft tab stop)
            if !line_to_cur.is_empty() && line_to_cur.chars().all(|c| c == ' ') {
                let count = line_to_cur.len();
                let to_delete = if count % 4 == 0 { 4 } else { count % 4 };
                for _ in 0..to_delete {
                    self.cur -= 1;
                    self.buf.remove(self.cur);
                }
                self.selection = None;
                return;
            }

            // 2. If line_to_cur is an empty blockquote prefix (e.g. ">>> ", ">> ", "> "):
            // Dedent quote depth by one level or clear it completely
            let trimmed_quote = line_to_cur.trim_start();
            if trimmed_quote.starts_with('>') && trimmed_quote.trim_end().chars().all(|c| c == '>')
            {
                let quote_chars_count = trimmed_quote.chars().filter(|&c| c == '>').count();
                if quote_chars_count <= 1 {
                    while self.cur > line_start {
                        self.cur -= 1;
                        self.buf.remove(self.cur);
                    }
                } else {
                    let has_trailing_space = line_to_cur.ends_with(' ');
                    while self.cur > line_start {
                        self.cur -= 1;
                        self.buf.remove(self.cur);
                    }
                    let indent_len = line_to_cur
                        .chars()
                        .take_while(|&c| c == ' ' || c == '\t')
                        .count();
                    let new_prefix = format!(
                        "{}{}{}",
                        &line_to_cur[..indent_len],
                        ">".repeat(quote_chars_count - 1),
                        if has_trailing_space { " " } else { "" }
                    );
                    for c in new_prefix.chars() {
                        self.buf.insert(self.cur, c);
                        self.cur += 1;
                    }
                }
                self.selection = None;
                return;
            }

            // 3. If line_to_cur is an empty list or task item prefix:
            // Clear the prefix completely
            let trimmed = line_to_cur.trim_start();
            if trimmed == "- "
                || trimmed == "* "
                || trimmed == "+ "
                || trimmed == "- [ ] "
                || trimmed == "- [x] "
                || trimmed == "- [X] "
                || trimmed == "- [ ]"
                || trimmed == "- [x]"
                || trimmed == "- [X]"
                || trimmed == "* [ ] "
                || trimmed == "* [x] "
                || trimmed == "* [X] "
                || trimmed == "* [ ]"
                || trimmed == "* [x]"
                || trimmed == "* [X]"
            {
                while self.cur > line_start {
                    self.cur -= 1;
                    self.buf.remove(self.cur);
                }
                self.selection = None;
                return;
            }

            // 4. If current line is an empty table row (e.g. "|  |  |" with only pipes and spaces):
            // Cleanly delete the whole row and move cursor to the end of the previous line
            let mut line_end = self.cur;
            while line_end < self.buf.len() && self.buf[line_end] != '\n' {
                line_end += 1;
            }
            let full_line: String = self.buf[line_start..line_end].iter().collect();
            let trimmed_full = full_line.trim();
            if trimmed_full.starts_with('|')
                && trimmed_full.ends_with('|')
                && trimmed_full.len() >= 2
                && trimmed_full.chars().all(|c| c == '|' || c == ' ')
            {
                let del_start = if line_start > 0 && self.buf[line_start - 1] == '\n' {
                    line_start - 1
                } else {
                    line_start
                };
                let del_end =
                    if line_start == 0 && line_end < self.buf.len() && self.buf[line_end] == '\n' {
                        line_end + 1
                    } else {
                        line_end
                    };
                self.buf.drain(del_start..del_end);
                self.cur = del_start;
                self.selection = None;
                return;
            }

            // Normal single-character backspace
            self.cur -= 1;
            self.buf.remove(self.cur);
            self.selection = None;
        }
    }

    #[allow(dead_code)]
    pub fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cur < self.buf.len() {
            self.save_undo_snapshot();
            self.buf.remove(self.cur);
            self.selection = None;
        }
    }

    pub fn delete_word(&mut self) {
        while self.cur > 0 && self.buf[self.cur - 1].is_whitespace() {
            self.backspace();
        }
        while self.cur > 0 && !self.buf[self.cur - 1].is_whitespace() {
            self.backspace();
        }
    }

    pub fn delete_word_forward(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cur < self.buf.len() {
            let next = self.next_word_boundary(self.cur);
            if next > self.cur {
                self.save_undo_snapshot();
                self.buf.drain(self.cur..next);
                self.selection = None;
            }
        }
    }

    pub fn auto_pair(&mut self, open: char, close: char) {
        if let Some((start, end)) = self.selected_range() {
            self.save_undo_snapshot();
            let selected: Vec<char> = self.buf.drain(start..end).collect();
            self.buf.insert(start, open);
            let mut p = start + 1;
            for c in selected {
                self.buf.insert(p, c);
                p += 1;
            }
            self.buf.insert(p, close);
            self.cur = p + 1;
            self.selection = None;
        } else {
            self.save_undo_snapshot();
            self.buf.insert(self.cur, open);
            self.buf.insert(self.cur + 1, close);
            self.cur += 1;
            self.selection = None;
        }
    }

    pub fn indent(&mut self) {
        if self.has_selection() {
            self.indent_line();
        } else {
            self.save_undo_snapshot();
            for _ in 0..4 {
                self.buf.insert(self.cur, ' ');
                self.cur += 1;
            }
        }
    }

    pub fn dedent(&mut self) {
        self.dedent_line();
    }

    pub fn indent_line(&mut self) {
        self.save_undo_snapshot();
        if let Some((start, end)) = self.selected_range() {
            let mut anchor = self.selection.unwrap();
            let mut cur = self.cur;

            let mut first_line_start = start;
            while first_line_start > 0 && self.buf[first_line_start - 1] != '\n' {
                first_line_start -= 1;
            }

            let mut line_starts = Vec::new();
            let mut i = first_line_start;
            while i <= self.buf.len() {
                line_starts.push(i);
                if let Some(nl) = self.buf[i..].iter().position(|&c| c == '\n') {
                    let next_start = i + nl + 1;
                    if next_start > end || (next_start == end && end > start) {
                        break;
                    }
                    i = next_start;
                } else {
                    break;
                }
            }

            line_starts.reverse();
            for ls in line_starts {
                for _ in 0..4 {
                    self.buf.insert(ls, ' ');
                }
                if anchor >= ls {
                    anchor += 4;
                }
                if cur >= ls {
                    cur += 4;
                }
            }

            self.cur = cur.min(self.buf.len());
            self.selection = Some(anchor.min(self.buf.len()));
        } else {
            let mut line_start = self.cur;
            while line_start > 0 && self.buf[line_start - 1] != '\n' {
                line_start -= 1;
            }
            for _ in 0..4 {
                self.buf.insert(line_start, ' ');
            }
            self.cur = (self.cur + 4).min(self.buf.len());
            self.selection = None;
        }
    }

    pub fn dedent_line(&mut self) {
        self.save_undo_snapshot();
        if let Some((start, end)) = self.selected_range() {
            let mut anchor = self.selection.unwrap();
            let mut cur = self.cur;

            let mut first_line_start = start;
            while first_line_start > 0 && self.buf[first_line_start - 1] != '\n' {
                first_line_start -= 1;
            }

            let mut line_starts = Vec::new();
            let mut i = first_line_start;
            while i <= self.buf.len() {
                line_starts.push(i);
                if let Some(nl) = self.buf[i..].iter().position(|&c| c == '\n') {
                    let next_start = i + nl + 1;
                    if next_start > end || (next_start == end && end > start) {
                        break;
                    }
                    i = next_start;
                } else {
                    break;
                }
            }

            line_starts.reverse();
            for ls in line_starts {
                let mut spaces = 0;
                while spaces < 4 && ls + spaces < self.buf.len() && self.buf[ls + spaces] == ' ' {
                    spaces += 1;
                }
                if spaces > 0 {
                    for _ in 0..spaces {
                        self.buf.remove(ls);
                    }
                    if anchor >= ls + spaces {
                        anchor -= spaces;
                    } else if anchor > ls {
                        anchor = ls;
                    }

                    if cur >= ls + spaces {
                        cur -= spaces;
                    } else if cur > ls {
                        cur = ls;
                    }
                }
            }

            self.cur = cur.min(self.buf.len());
            self.selection = if anchor != self.cur {
                Some(anchor.min(self.buf.len()))
            } else {
                None
            };
        } else {
            let mut line_start = self.cur;
            while line_start > 0 && self.buf[line_start - 1] != '\n' {
                line_start -= 1;
            }
            let mut spaces = 0;
            while spaces < 4
                && line_start + spaces < self.buf.len()
                && self.buf[line_start + spaces] == ' '
            {
                spaces += 1;
            }
            if spaces > 0 {
                for _ in 0..spaces {
                    self.buf.remove(line_start);
                }
                if self.cur >= line_start + spaces {
                    self.cur -= spaces;
                } else if self.cur > line_start {
                    self.cur = line_start;
                }
            }
            self.selection = None;
        }
    }

    /// Toggles markdown checklist items (`- [ ]` / `- [x]`) on the current line or across all selected lines.
    ///
    /// Handles multiple selected paragraphs, plain text, bullet lists (`-`, `*`, `+`),
    /// and numbered lists (`1.`), converting them into checklist items or toggling their checked state.
    pub fn toggle_checklist(&mut self) -> bool {
        self.save_undo_snapshot();

        let (first_start, last_end, has_sel) = if let Some((start, end)) = self.selected_range() {
            let (fs, _) = self.line_bounds(start);
            let check_end = if end > start && self.buf.get(end - 1) == Some(&'\n') {
                end - 1
            } else {
                end
            };
            let (_, le) = self.line_bounds(check_end);
            (fs, le, true)
        } else {
            let (fs, le) = self.line_bounds(self.cur);
            (fs, le, false)
        };

        // Gather all line (start, end) ranges within the selection
        let mut lines = Vec::new();
        let mut i = first_start;
        while i <= last_end && i <= self.buf.len() {
            let (ls, le) = self.line_bounds(i);
            lines.push((ls, le));
            if le >= last_end || le >= self.buf.len() {
                break;
            }
            i = le + 1;
        }

        if lines.is_empty() {
            return false;
        }

        #[derive(PartialEq)]
        enum LineTaskState {
            CheckedTask(usize),
            UncheckedTask(usize),
            Bullet(usize),
            Numbered(usize),
            Plain,
            Empty,
        }

        let mut line_states = Vec::new();
        for &(ls, le) in &lines {
            let line_chars: Vec<char> = self.buf[ls..le].to_vec();
            let indent_len = line_chars
                .iter()
                .take_while(|c| **c == ' ' || **c == '\t')
                .count();
            let rest: String = line_chars[indent_len..].iter().collect();

            if rest.is_empty() {
                line_states.push(LineTaskState::Empty);
            } else if rest.starts_with("- [x] ")
                || rest.starts_with("- [X] ")
                || rest.starts_with("* [x] ")
                || rest.starts_with("* [X] ")
                || rest.starts_with("+ [x] ")
                || rest.starts_with("+ [X] ")
            {
                line_states.push(LineTaskState::CheckedTask(indent_len + 6));
            } else if rest.starts_with("- [ ] ")
                || rest.starts_with("* [ ] ")
                || rest.starts_with("+ [ ] ")
            {
                line_states.push(LineTaskState::UncheckedTask(indent_len + 6));
            } else if rest.starts_with("- ") || rest.starts_with("* ") || rest.starts_with("+ ") {
                line_states.push(LineTaskState::Bullet(indent_len + 2));
            } else if let Some(num_len) = parse_numbered_list_prefix(&rest) {
                line_states.push(LineTaskState::Numbered(indent_len + num_len));
            } else {
                line_states.push(LineTaskState::Plain);
            }
        }

        let non_empty_states: Vec<&LineTaskState> = line_states
            .iter()
            .filter(|s| **s != LineTaskState::Empty)
            .collect();
        let all_checked = !non_empty_states.is_empty()
            && non_empty_states
                .iter()
                .all(|s| matches!(s, LineTaskState::CheckedTask(_)));
        let all_unchecked = !non_empty_states.is_empty()
            && non_empty_states
                .iter()
                .all(|s| matches!(s, LineTaskState::UncheckedTask(_)));

        let mut anchor = self.selection.unwrap_or(self.cur);
        let mut cur = self.cur;

        // Apply transformations in reverse order so character offsets before ls remain invariant
        for (idx, &(ls, le)) in lines.iter().enumerate().rev() {
            let state = &line_states[idx];
            let line_chars: Vec<char> = self.buf[ls..le].to_vec();
            let indent_len = line_chars
                .iter()
                .take_while(|c| **c == ' ' || **c == '\t')
                .count();
            let indent: Vec<char> = line_chars[..indent_len].to_vec();

            let new_line: Vec<char> = if all_checked {
                if !has_sel {
                    // Single line cycle: checked -> plain text
                    let rest = &line_chars[indent_len + 6..];
                    let mut res = indent;
                    res.extend_from_slice(rest);
                    res
                } else {
                    // Multi-line: uncheck to "- [ ] "
                    let rest = &line_chars[indent_len + 6..];
                    let mut res = indent;
                    res.extend("- [ ] ".chars());
                    res.extend_from_slice(rest);
                    res
                }
            } else if all_unchecked {
                // If all are unchecked: check them to "- [x] "
                let rest = &line_chars[indent_len + 6..];
                let mut res = indent;
                res.extend("- [x] ".chars());
                res.extend_from_slice(rest);
                res
            } else {
                // Convert plain / bullet / numbered lines to "- [ ] "
                match state {
                    LineTaskState::Empty => line_chars,
                    LineTaskState::CheckedTask(_) | LineTaskState::UncheckedTask(_) => line_chars,
                    LineTaskState::Bullet(prefix_len) => {
                        let rest = &line_chars[*prefix_len..];
                        let mut res = indent;
                        res.extend("- [ ] ".chars());
                        res.extend_from_slice(rest);
                        res
                    }
                    LineTaskState::Numbered(prefix_len) => {
                        let rest = &line_chars[*prefix_len..];
                        let mut res = indent;
                        res.extend("- [ ] ".chars());
                        res.extend_from_slice(rest);
                        res
                    }
                    LineTaskState::Plain => {
                        let rest = &line_chars[indent_len..];
                        let mut res = indent;
                        res.extend("- [ ] ".chars());
                        res.extend_from_slice(rest);
                        res
                    }
                }
            };

            let old_len = le - ls;
            let new_len = new_line.len();
            let delta = new_len as isize - old_len as isize;

            self.buf.splice(ls..le, new_line);

            if anchor >= le {
                anchor = (anchor as isize + delta).max(ls as isize) as usize;
            } else if anchor > ls + indent_len {
                anchor = (anchor as isize + delta).max(ls as isize) as usize;
            }

            if cur >= le {
                cur = (cur as isize + delta).max(ls as isize) as usize;
            } else if cur > ls + indent_len {
                cur = (cur as isize + delta).max(ls as isize) as usize;
            }
        }

        self.cur = cur.min(self.buf.len());
        if has_sel {
            self.selection = Some(anchor.min(self.buf.len()));
        }

        true
    }

    /// Returns (line_start, line_end) for the line containing `pos`.
    pub fn line_bounds(&self, pos: usize) -> (usize, usize) {
        let mut line_start = pos.min(self.buf.len());
        while line_start > 0 && self.buf[line_start - 1] != '\n' {
            line_start -= 1;
        }
        let mut line_end = pos.min(self.buf.len());
        while line_end < self.buf.len() && self.buf[line_end] != '\n' {
            line_end += 1;
        }
        (line_start, line_end)
    }

    /// Returns the canonical column count, indentation, and whether a separator exists
    /// for the table containing `pos`. Scans upwards to the table header.
    pub fn table_canonical_info(&self, pos: usize) -> (usize, String, bool) {
        let (line_start, line_end) = self.line_bounds(pos);
        let mut curr_start = line_start;
        let mut header_start = line_start;
        let mut header_end = line_end;
        let mut has_sep = false;

        // Scan backwards through contiguous table lines to locate the header row
        while curr_start > 0 {
            let mut prev_start = curr_start - 1;
            while prev_start > 0 && self.buf[prev_start - 1] != '\n' {
                prev_start -= 1;
            }
            let prev_str: String = self.buf[prev_start..curr_start - 1].iter().collect();
            let pt = prev_str.trim();
            if pt.contains('|')
                && (pt.starts_with('|')
                    || pt.ends_with('|')
                    || pt.split('|').filter(|p| !p.trim().is_empty()).count() >= 2)
            {
                let is_sep_line = pt.contains('-')
                    && pt
                        .chars()
                        .all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                if is_sep_line {
                    has_sep = true;
                } else if !has_sep {
                    header_start = prev_start;
                    header_end = curr_start - 1;
                }
                curr_start = prev_start;
            } else {
                break;
            }
        }

        // Check if current line is separator
        let cur_str: String = self.buf[line_start..line_end].iter().collect();
        let cur_trim = cur_str.trim();
        if cur_trim.contains('-')
            && cur_trim
                .chars()
                .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
        {
            has_sep = true;
        }

        // Check if line below is separator
        if line_end < self.buf.len() {
            let mut next_end = line_end + 1;
            while next_end < self.buf.len() && self.buf[next_end] != '\n' {
                next_end += 1;
            }
            let next_str: String = self.buf[line_end + 1..next_end].iter().collect();
            let nt = next_str.trim();
            if nt.contains('-')
                && nt
                    .chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
            {
                has_sep = true;
            }
        }

        let header_str: String = self.buf[header_start..header_end].iter().collect();
        let indent: String = header_str
            .chars()
            .take_while(|&c| c == ' ' || c == '\t')
            .collect();
        let h_trim = header_str.trim();
        let col_count = h_trim.trim_matches('|').split('|').count().max(1);

        (col_count, indent, has_sep)
    }

    /// Handles Enter key in insert mode:
    /// - If there is an active selection, replaces it with a simple newline without auto-indent.
    /// - Preserves exact leading whitespace (spaces and tabs) of current line.
    /// - Auto-continues numbered lists (`1. Hello` -> `2. `) and bullet lists (`- Hello`, `* Hello`, `+ Hello`).
    /// - If current line is only a marker with no content (`1. `, `- `), clears the marker and inserts a plain newline.
    /// - Composes indentation with lists (`  - Hello` -> `  - `, `  1. Hello` -> `  2. `).
    /// - Whole Enter action is recorded as a single atomic undo step.
    pub fn handle_enter(&mut self) {
        if self.has_selection() {
            self.insert('\n');
            return;
        }

        self.save_undo_snapshot();

        let (line_start, line_end) = self.line_bounds(self.cur);
        let line_str: String = self.buf[line_start..line_end].iter().collect();

        // 1. Extract leading whitespace (spaces and tabs)
        let indent_len = line_str
            .chars()
            .take_while(|&c| c == ' ' || c == '\t')
            .count();
        let indent = &line_str[..indent_len];
        let rest = &line_str[indent_len..];

        // If line contains only whitespace, clear it on Enter and insert plain newline at col 0
        if line_str.trim().is_empty() && !line_str.is_empty() {
            self.buf.drain(line_start..line_end);
            self.buf.insert(line_start, '\n');
            self.cur = line_start + 1;
            self.selection = None;
            return;
        }

        // 2. Check if current line is only a list marker with no content
        let is_empty_numbered = {
            let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
            if digits_len > 0 && digits_len < rest.len() {
                let after_digits = &rest[digits_len..];
                if after_digits.starts_with('.') {
                    let after_dot = &after_digits[1..];
                    after_dot.trim().is_empty()
                } else {
                    false
                }
            } else {
                false
            }
        };

        let is_task = rest.starts_with("- [ ] ")
            || rest.starts_with("- [x] ")
            || rest.starts_with("- [X] ")
            || rest.starts_with("* [ ] ")
            || rest.starts_with("* [x] ")
            || rest.starts_with("* [X] ");
        let is_empty_task = is_task && rest[6..].trim().is_empty();

        let is_empty_bullet = {
            if rest.starts_with('-') || rest.starts_with('*') || rest.starts_with('+') {
                rest[1..].trim().is_empty()
            } else {
                false
            }
        };

        let quote_prefix = parse_quote_prefix(rest);
        let is_empty_quote = if let Some(qp) = quote_prefix {
            rest[qp.len()..].trim().is_empty()
        } else {
            false
        };

        let rest_trim = rest.trim();
        let is_table_row = rest_trim.contains('|')
            && (rest_trim.starts_with('|')
                || rest_trim.ends_with('|')
                || rest_trim
                    .split('|')
                    .filter(|p| !p.trim().is_empty())
                    .count()
                    >= 2);
        let is_table_sep = is_table_row
            && rest_trim.contains('-')
            && rest_trim
                .chars()
                .all(|c| c == '|' || c == '-' || c == ':' || c == ' ');

        let is_empty_table_row = is_table_row && !is_table_sep && {
            let parts: Vec<&str> = rest_trim.trim_matches('|').split('|').collect();
            !parts.is_empty() && parts.iter().all(|p| p.trim().is_empty())
        };

        if is_empty_numbered
            || is_empty_task
            || (!is_task && is_empty_bullet)
            || is_empty_quote
            || is_empty_table_row
        {
            self.buf.drain(line_start..line_end);
            self.buf.insert(line_start, '\n');
            self.cur = line_start + 1;
            self.selection = None;
            return;
        }

        // 3. Check for list / quote / table continuation
        let (next_prefix, cursor_offset, insert_at_end) = if is_task {
            (format!("\n{}- [ ] ", indent), None, false)
        } else if let Some(qp) = quote_prefix {
            if qp.ends_with(' ') {
                (format!("\n{}{}", indent, qp), None, false)
            } else {
                (format!("\n{}{} ", indent, qp), None, false)
            }
        } else if let Some(num) = parse_numbered_list(rest) {
            (
                format!("\n{}{}. ", indent, num.saturating_add(1)),
                None,
                false,
            )
        } else if let Some(bullet) = parse_bullet_list(rest) {
            (format!("\n{}{} ", indent, bullet), None, false)
        } else if is_table_row {
            let (canonical_cols, tbl_indent, _) = self.table_canonical_info(self.cur);
            let row_indent = if !tbl_indent.is_empty() {
                &tbl_indent
            } else {
                indent
            };

            let has_sep_above = {
                let mut prev_line_start = line_start;
                let mut found_sep = false;
                while prev_line_start > 0 {
                    let mut p_start = prev_line_start - 1;
                    while p_start > 0 && self.buf[p_start - 1] != '\n' {
                        p_start -= 1;
                    }
                    let p_str: String = self.buf[p_start..prev_line_start - 1].iter().collect();
                    let pt = p_str.trim();
                    if pt.contains('|') {
                        if pt.contains('-')
                            && pt
                                .chars()
                                .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
                        {
                            found_sep = true;
                            break;
                        }
                    } else {
                        break;
                    }
                    prev_line_start = p_start;
                }
                found_sep
            };

            let next_line_is_sep = {
                if line_end < self.buf.len() {
                    let mut next_end = line_end + 1;
                    while next_end < self.buf.len() && self.buf[next_end] != '\n' {
                        next_end += 1;
                    }
                    let next_str: String = self.buf[line_end + 1..next_end].iter().collect();
                    let nt = next_str.trim();
                    nt.starts_with('|')
                        && nt.contains('-')
                        && nt
                            .chars()
                            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ')
                } else {
                    false
                }
            };

            if !is_table_sep && !has_sep_above && !next_line_is_sep {
                // If creating a table header without a separator, auto-generate separator and first row
                let mut s = format!("\n{}|", row_indent);
                for _ in 0..canonical_cols {
                    s.push_str(" --- |");
                }
                s.push_str(&format!("\n{}|", row_indent));
                for _ in 0..canonical_cols {
                    s.push_str("  |");
                }
                let first_cell_offset = if let Some(last_newline) = s.rfind('\n') {
                    last_newline + 1 + row_indent.chars().count() + 2
                } else {
                    s.chars().count()
                };
                (s, Some(first_cell_offset), true)
            } else {
                let mut s = format!("\n{}|", row_indent);
                for _ in 0..canonical_cols {
                    s.push_str("  |");
                }
                let first_cell_offset = if let Some(last_newline) = s.rfind('\n') {
                    last_newline + 1 + row_indent.chars().count() + 2
                } else {
                    s.chars().count()
                };
                (s, Some(first_cell_offset), true)
            }
        } else {
            // Preserve exact leading whitespace
            (format!("\n{}", indent), None, false)
        };

        let insert_pos = if insert_at_end { line_end } else { self.cur };
        for (idx, ch) in next_prefix.chars().enumerate() {
            self.buf.insert(insert_pos + idx, ch);
        }
        if let Some(offset) = cursor_offset {
            self.cur = insert_pos + offset.min(next_prefix.chars().count());
        } else {
            self.cur = insert_pos + next_prefix.chars().count();
        }
        self.selection = None;
    }

    /// Exits a code block wrapper (```...```) or markdown table when Ctrl+Enter is pressed,
    /// placing the cursor on a clean new line underneath the block.
    /// Returns true if handled, false if not inside a block or table.
    pub fn exit_block_or_table(&mut self) -> bool {
        if self.buf.is_empty() {
            return false;
        }

        // 1. Check if inside a table
        let (cur_line_start, cur_line_end) = self.line_bounds(self.cur);
        let cur_line: String = self.buf[cur_line_start..cur_line_end].iter().collect();
        let cur_trim = cur_line.trim();
        let is_cur_table = cur_trim.contains('|')
            && (cur_trim.starts_with('|')
                || cur_trim.ends_with('|')
                || cur_trim.split('|').filter(|p| !p.trim().is_empty()).count() >= 2);

        if is_cur_table {
            // Find the end of this table block by scanning forward
            let mut scan_start = cur_line_start;
            let mut last_table_end = cur_line_end;

            while scan_start < self.buf.len() {
                let mut scan_end = scan_start;
                while scan_end < self.buf.len() && self.buf[scan_end] != '\n' {
                    scan_end += 1;
                }
                let line_str: String = self.buf[scan_start..scan_end].iter().collect();
                let lt = line_str.trim();
                let is_tbl = lt.contains('|')
                    && (lt.starts_with('|')
                        || lt.ends_with('|')
                        || lt.split('|').filter(|p| !p.trim().is_empty()).count() >= 2);
                if is_tbl {
                    last_table_end = scan_end;
                    if scan_end < self.buf.len() {
                        scan_start = scan_end + 1;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            self.save_undo_snapshot();
            if last_table_end == self.buf.len() {
                self.buf.insert(last_table_end, '\n');
                self.cur = last_table_end + 1;
            } else {
                let next_start = last_table_end + 1;
                if next_start < self.buf.len() && self.buf[next_start] == '\n' {
                    self.cur = next_start;
                } else {
                    self.buf.insert(last_table_end + 1, '\n');
                    self.cur = last_table_end + 1;
                }
            }
            self.selection = None;
            return true;
        }

        // 2. Check if inside a code block (between ```/~~~ fences)
        let mut in_code = false;
        let mut open_fence_start = None;
        let mut cur_in_code = false;
        let mut line_s = 0;

        while line_s <= self.buf.len() {
            let mut line_e = line_s;
            while line_e < self.buf.len() && self.buf[line_e] != '\n' {
                line_e += 1;
            }
            let line_chars: Vec<char> = self.buf[line_s..line_e].to_vec();
            let is_fence = line_chars.starts_with(&['`', '`', '`'])
                || line_chars.starts_with(&['~', '~', '~']);
            let line_contains_cur = self.cur >= line_s
                && (self.cur <= line_e || (line_e == self.buf.len() && self.cur == line_e));

            if is_fence {
                if !in_code {
                    in_code = true;
                    open_fence_start = Some(line_s);
                    if line_contains_cur {
                        cur_in_code = true;
                    }
                } else {
                    in_code = false;
                    if line_contains_cur {
                        cur_in_code = true;
                    }
                }
            } else if in_code && line_contains_cur {
                cur_in_code = true;
            }

            if line_e < self.buf.len() {
                line_s = line_e + 1;
            } else {
                break;
            }
        }

        if cur_in_code {
            self.save_undo_snapshot();
            let scan_from = open_fence_start.unwrap_or(self.cur);
            let mut s = scan_from;
            let mut found_closing_fence = None;
            let mut first_fence_skipped = false;

            while s <= self.buf.len() {
                let mut e = s;
                while e < self.buf.len() && self.buf[e] != '\n' {
                    e += 1;
                }
                let line_chars: Vec<char> = self.buf[s..e].to_vec();
                let is_fence = line_chars.starts_with(&['`', '`', '`'])
                    || line_chars.starts_with(&['~', '~', '~']);

                if is_fence {
                    if !first_fence_skipped {
                        first_fence_skipped = true;
                    } else {
                        found_closing_fence = Some((s, e));
                        break;
                    }
                }

                if e < self.buf.len() {
                    s = e + 1;
                } else {
                    break;
                }
            }

            if let Some((_cf_start, cf_end)) = found_closing_fence {
                if cf_end == self.buf.len() {
                    self.buf.insert(cf_end, '\n');
                    self.cur = cf_end + 1;
                } else {
                    let next_start = cf_end + 1;
                    if next_start < self.buf.len() && self.buf[next_start] == '\n' {
                        self.cur = next_start;
                    } else {
                        self.buf.insert(cf_end + 1, '\n');
                        self.cur = cf_end + 1;
                    }
                }
            } else {
                let (_, c_end) = self.line_bounds(self.cur);
                let close_str = "\n```\n";
                for (i, ch) in close_str.chars().enumerate() {
                    self.buf.insert(c_end + i, ch);
                }
                self.cur = c_end + close_str.chars().count();
            }
            self.selection = None;
            return true;
        }

        false
    }

    /// Navigates cell-by-cell in markdown tables via Tab / Shift+Tab.
    /// If at the end of the table on Tab, automatically appends a new row and places caret inside its first cell.
    /// Returns true if handled, false if not in a table.
    pub fn table_nav_tab(&mut self, forward: bool) -> bool {
        if self.buf.is_empty() {
            return false;
        }

        let (line_start, line_end) = self.line_bounds(self.cur);
        let line_str: String = self.buf[line_start..line_end].iter().collect();
        let trimmed = line_str.trim();

        if !trimmed.contains('|') || trimmed.len() < 2 {
            return false;
        }

        let indent: String = self.buf[line_start..line_end]
            .iter()
            .take_while(|&&c| c == ' ' || c == '\t')
            .collect();

        // Collect all pipe positions on the current line
        let pipes: Vec<usize> = self.buf[line_start..line_end]
            .iter()
            .enumerate()
            .filter(|(_, &c)| c == '|')
            .map(|(i, _)| line_start + i)
            .collect();

        if pipes.len() < 2 {
            return false;
        }

        if forward {
            // Find next pipe after self.cur
            let next_pipe_idx = pipes.iter().position(|&p| p > self.cur);
            if let Some(p_idx) = next_pipe_idx {
                // If there is another cell after this pipe on the same line
                if p_idx + 1 < pipes.len() {
                    let cell_start = pipes[p_idx];
                    let target =
                        if cell_start + 1 < self.buf.len() && self.buf[cell_start + 1] == ' ' {
                            (cell_start + 2).min(pipes[p_idx + 1])
                        } else {
                            (cell_start + 1).min(pipes[p_idx + 1])
                        };
                    self.cur = target;
                    self.selection = None;
                    return true;
                }
            }

            // If we reached the end of the current row, jump to next row
            if line_end < self.buf.len() {
                let next_start = line_end + 1;
                let mut next_end = next_start;
                while next_end < self.buf.len() && self.buf[next_end] != '\n' {
                    next_end += 1;
                }
                let next_str: String = self.buf[next_start..next_end].iter().collect();
                let next_trim = next_str.trim();

                if next_trim.contains('|')
                    && (next_trim.starts_with('|')
                        || next_trim.ends_with('|')
                        || next_trim
                            .split('|')
                            .filter(|p| !p.trim().is_empty())
                            .count()
                            >= 2)
                {
                    let is_sep = next_trim.contains('-')
                        && next_trim
                            .chars()
                            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                    if is_sep {
                        // Skip separator row to the data row beneath it
                        if next_end < self.buf.len() {
                            let data_start = next_end + 1;
                            let mut data_end = data_start;
                            while data_end < self.buf.len() && self.buf[data_end] != '\n' {
                                data_end += 1;
                            }
                            let data_str: String = self.buf[data_start..data_end].iter().collect();
                            let data_trim = data_str.trim();
                            if data_trim.contains('|') {
                                if let Some(first_pipe) = self.buf[data_start..data_end]
                                    .iter()
                                    .position(|&c| c == '|')
                                {
                                    let abs_pipe = data_start + first_pipe;
                                    let target = if abs_pipe + 1 < self.buf.len()
                                        && self.buf[abs_pipe + 1] == ' '
                                    {
                                        abs_pipe + 2
                                    } else {
                                        abs_pipe + 1
                                    };
                                    self.cur = target.min(data_end);
                                    self.selection = None;
                                    return true;
                                }
                            }
                        }
                    } else {
                        // Regular next row
                        if let Some(first_pipe) = self.buf[next_start..next_end]
                            .iter()
                            .position(|&c| c == '|')
                        {
                            let abs_pipe = next_start + first_pipe;
                            let target =
                                if abs_pipe + 1 < self.buf.len() && self.buf[abs_pipe + 1] == ' ' {
                                    abs_pipe + 2
                                } else {
                                    abs_pipe + 1
                                };
                            self.cur = target.min(next_end);
                            self.selection = None;
                            return true;
                        }
                    }
                }
            }

            // At the end of the table (or on header without separator):
            let (canonical_cols, tbl_indent, has_sep) = self.table_canonical_info(self.cur);
            let row_indent = if !tbl_indent.is_empty() {
                &tbl_indent
            } else {
                &indent
            };
            let is_sep_line = trimmed.contains('-')
                && trimmed
                    .chars()
                    .all(|c| c == '|' || c == '-' || c == ':' || c == ' ');

            if !is_sep_line && !has_sep {
                // If pressing Tab on header row without a separator, auto-generate separator and first row!
                let mut new_block = format!("\n{}|", row_indent);
                for _ in 0..canonical_cols {
                    new_block.push_str(" --- |");
                }
                new_block.push_str(&format!("\n{}|", row_indent));
                for _ in 0..canonical_cols {
                    new_block.push_str("  |");
                }
                self.save_undo_snapshot();
                for (i, ch) in new_block.chars().enumerate() {
                    self.buf.insert(line_end + i, ch);
                }
                let first_cell_offset = if let Some(last_newline) = new_block.rfind('\n') {
                    last_newline + 1 + row_indent.chars().count() + 2
                } else {
                    new_block.chars().count()
                };
                self.cur = (line_end + first_cell_offset).min(self.buf.len());
                self.selection = None;
                return true;
            } else {
                let mut new_row = format!("\n{}|", row_indent);
                for _ in 0..canonical_cols {
                    new_row.push_str("  |");
                }
                self.save_undo_snapshot();
                for (i, ch) in new_row.chars().enumerate() {
                    self.buf.insert(line_end + i, ch);
                }
                let first_cell_offset = if let Some(last_newline) = new_row.rfind('\n') {
                    last_newline + 1 + row_indent.chars().count() + 2
                } else {
                    new_row.chars().count()
                };
                self.cur = (line_end + first_cell_offset).min(self.buf.len());
                self.selection = None;
                return true;
            }
        } else {
            // Backward: Shift+Tab
            let cur_pipe_idx = pipes.iter().rposition(|&p| p <= self.cur);
            if let Some(p_idx) = cur_pipe_idx {
                if p_idx > 1 {
                    // Jump to previous cell on the same line
                    let target_pipe = pipes[p_idx - 1];
                    let target =
                        if target_pipe + 1 < self.buf.len() && self.buf[target_pipe + 1] == ' ' {
                            target_pipe + 2
                        } else {
                            target_pipe + 1
                        };
                    self.cur = target.min(pipes[p_idx]);
                    self.selection = None;
                    return true;
                } else if p_idx == 1 && self.cur > pipes[0] + 2 {
                    // Jump to start of first cell
                    self.cur = pipes[0]
                        + if pipes[0] + 1 < self.buf.len() && self.buf[pipes[0] + 1] == ' ' {
                            2
                        } else {
                            1
                        };
                    self.selection = None;
                    return true;
                }
            }

            // Jump to previous row's last cell
            if line_start > 0 {
                let mut prev_start = line_start - 1;
                while prev_start > 0 && self.buf[prev_start - 1] != '\n' {
                    prev_start -= 1;
                }
                let prev_end = line_start - 1;
                let prev_str: String = self.buf[prev_start..prev_end].iter().collect();
                let prev_trim = prev_str.trim();

                if prev_trim.contains('|') {
                    let is_sep = prev_trim.contains('-')
                        && prev_trim
                            .chars()
                            .all(|c| c == '|' || c == '-' || c == ':' || c == ' ');
                    let (target_start, target_end) = if is_sep && prev_start > 0 {
                        // Skip separator to header row above it
                        let mut h_start = prev_start - 1;
                        while h_start > 0 && self.buf[h_start - 1] != '\n' {
                            h_start -= 1;
                        }
                        (h_start, prev_start - 1)
                    } else {
                        (prev_start, prev_end)
                    };

                    let target_chars = &self.buf[target_start..target_end];
                    let prev_pipes: Vec<usize> = target_chars
                        .iter()
                        .enumerate()
                        .filter(|(_, &c)| c == '|')
                        .map(|(i, _)| target_start + i)
                        .collect();

                    if prev_pipes.len() >= 2 {
                        let last_cell_pipe = prev_pipes[prev_pipes.len() - 2];
                        let target = if last_cell_pipe + 1 < self.buf.len()
                            && self.buf[last_cell_pipe + 1] == ' '
                        {
                            last_cell_pipe + 2
                        } else {
                            last_cell_pipe + 1
                        };
                        self.cur = target.min(prev_pipes[prev_pipes.len() - 1]);
                        self.selection = None;
                        return true;
                    }
                }
            }

            false
        }
    }
}

fn parse_quote_prefix(rest: &str) -> Option<&str> {
    if !rest.starts_with('>') {
        return None;
    }
    let mut end = 0;
    let bytes = rest.as_bytes();
    while end < bytes.len() && (bytes[end] == b'>' || bytes[end] == b' ') {
        end += 1;
    }
    if end > 0 {
        Some(&rest[..end])
    } else {
        None
    }
}

fn parse_numbered_list(rest: &str) -> Option<u64> {
    let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 || digits_len >= rest.len() {
        return None;
    }
    let after_digits = &rest[digits_len..];
    if !after_digits.starts_with('.') {
        return None;
    }
    let after_dot = &after_digits[1..];
    if !after_dot.starts_with(' ') && !after_dot.starts_with('\t') {
        return None;
    }
    let content = after_dot.trim_start_matches(|c| c == ' ' || c == '\t');
    if content.is_empty() {
        return None;
    }
    rest[..digits_len].parse::<u64>().ok()
}

fn parse_bullet_list(rest: &str) -> Option<char> {
    let mut chars = rest.chars();
    let bullet = chars.next()?;
    if bullet != '-' && bullet != '*' && bullet != '+' {
        return None;
    }
    let after_bullet = &rest[bullet.len_utf8()..];
    if !after_bullet.starts_with(' ') && !after_bullet.starts_with('\t') {
        return None;
    }
    let content = after_bullet.trim_start_matches(|c| c == ' ' || c == '\t');
    if content.is_empty() {
        return None;
    }
    Some(bullet)
}

fn parse_numbered_list_prefix(rest: &str) -> Option<usize> {
    let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 || digits_len >= rest.len() {
        return None;
    }
    let after_digits = &rest[digits_len..];
    if after_digits.starts_with(". ") || after_digits.starts_with(".\t") {
        Some(digits_len + 2)
    } else {
        None
    }
}
