# MindForge — Bug, Hardcode & Code-Quality Audit

> Auto-generated deep audit of the full workspace (`app`, `core`, `webscan`).
> Tags: **BUG** = correctness/panic/data-loss · **HARDCODE** = magic value/path/string ·
> **UGLY** = code smell / duplication / god function · **SECURITY** = unsafe/insecure ·
> **DEADCODE** = unreachable / unused.
> Each entry: `file:line` — title — explanation. Suggestion in *italics*.

---

## 🔴 CRITICAL BUGS (panic / crash / data-loss / wrong behavior)

### ✅ Completed: UTF-8 panic findings
- `app/src/agent/deepseek_ui.rs` and `app/src/agent/mod.rs` — safe Unicode truncation for titles and generated prompt snippets.
- `app/src/modals/search.rs`, `app/src/view_editor/titlebar.rs`, `app/src/rightsidebar/backlinks.rs`, and `app/src/rightsidebar/outline.rs` — safe Unicode truncation for displayed text.
- `app/src/wikilink/wikilink_autocompletion.rs` — safe Unicode truncation for folder/domain badges and long tokens.
- `app/src/accent/accentcolor.rs` — non-ASCII hex input is rejected before byte slicing.
- `webscan/src/outdated.rs` — ASCII-only case folding keeps marker offsets aligned with the original HTML body.

### Updater is pointed at the wrong repo
- **app/src/services/updater.rs:115** — BUG/HARDCODE — `let repo = "Saboor-Hamedi/my_first_project";` is a copy-paste leftover, not the MindForge repo. Combined with `updater.rs:125` (`Err(ureq::Error::Status(404, _)) => UpdateStatus::UpToDate`), the updater reports "up to date" forever and masks real failures. *Point at the real repo and treat 404 as an error.*
- **app/src/services/updater.rs:116** — HARDCODE — GitHub API URL baked in.
- **app/src/services/updater.rs:120, 219** — HARDCODE — `.user_agent("mindforge-updater/1.0")` duplicated.
- **app/src/services/updater.rs:238** — SECURITY (minor) — `temp_dir().join(format!("mindforge_update_{new_version}_{asset_name}"))` uses a predictable name and interpolates unvalidated `asset_name` → path-traversal / symlink-collision risk.

### Memory / resource leaks
- **app/src/ui/docs.rs:92-122** — BUG — when a local `brain/` folder exists, `get_docs()` calls `Box::leak(content.into_boxed_str())` for all 11 docs on **every call**; it is invoked every frame from `shell.rs:73`, `panes.rs:199`, `notes.rs:286` → leaks 11 boxed strings per frame. *Cache the docs instead of leaking.*
- **app/src/vim/client.rs:292-305** — BUG — unbounded event mpsc channel with no backpressure; a sustained redraw burst while the UI thread is slow can grow `self.events` without bound (OOM).
- **app/src/workspace_import/scanner.rs:52-59** — BUG — recursive directory walk has no symlink-cycle protection → infinite recursion / stack overflow on a symlinked loop.
- **app/src/input/global/clipboard.rs:87-89** — SECURITY/BUG — unbounded `while *ptr.add(len) != 0` raw-memory scan with no length cap; malformed/non-null-terminated clipboard data → out-of-bounds reads (UB/crash).

### Editor / undo logic
- **app/src/editor/editing.rs:7** — BUG — undo grouping only snapshots at whitespace or on the first keystroke (`c.is_whitespace() || self.undo_stack.is_empty()`); edits made at different times/positions merge into one undo step.
- **app/src/editor/editing.rs:15-28** — BUG — `insert_str` double-saves: `delete_selection()` already snapshots (selection.rs:44), then line 20 snapshots again → undo lands on a spurious intermediate state. Inconsistent with `insert()`.
- **app/src/editor/editing.rs:168-175** — BUG — `delete_word` loops `self.backspace()` (N undo snapshots for one Ctrl+Backspace), inherits backspace's special cases mid-word, and when a selection exists with `cur == 0` the selection is never deleted (no `delete_selection()` call).
- **app/src/editor/editing.rs:968, 820, 877** — BUG — `table_nav_tab` / `exit_block_or_table` snapshot unconditionally even on cursor-only paths → undo pops a cursor-position snapshot and spuriously jumps the caret.
- **app/src/editor/editing.rs:39-43** — BUG (minor) — auto-pair backspace can't distinguish an auto-paired empty token from legitimately adjacent quotes (`don''t`, `say ""hi""`) and deletes both.
- **app/src/editor/editing.rs:417-421** — BUG (copy-paste) — `CheckedTask`/`UncheckedTask` omit `"+ [x] "`/`"+ [ ] "` even though `"+ "` is a valid bullet; toggling `+ [x] item` rewrites it as `- [ ] [x] item`.
- **app/src/editor/editing.rs:367, 395-397** — BUG (minor) — `toggle_checklist` pushes an undo snapshot before bailing out (empty lines / no-op mixed states), polluting the undo stack.
- **app/src/editor/editing.rs:921-926** — BUG (minor) — missing closing fence inserts `"\n```\n"` at line end, leaving a spurious blank line.
- **app/src/editor/mod.rs:66-78** — BUG (latent) — `ensure_line_offsets` validity check (`!offsets_dirty && line_offsets.len() >= buf.len()`) can return stale offsets because `offsets_dirty` is set only in `set_text`, never on edits. `row_col_of_fast` currently has no callers, but the cache is one refactor away from corrupting `row_col`.
- **app/src/editor/selection.rs:6-10** — BUG/DEADCODE (latent) — `selection_inclusive` branch is unreachable in production (only tests set it) and extends the range by one char; a fragile off-by-one trap for every `has_selection()` consumer.
- **app/src/editor/visual.rs:108-114** — BUG (suspicious) — upward-nav skip loop compares against the *pre-move* cursor and exists only in `up_visual`/`up_visual_select` (absent from `down_visual`/`page_up_visual`) → asymmetric row skipping.
- **app/src/editor/visual.rs:343-348** — BUG/DEADCODE — `caret_cell` ignores `_vim_mode` despite the doc promising mode-dependent clamping; block-mode carets float one cell past line end.

### Vim / Neovim integration
- **app/src/vim/backend.rs:511-524** — BUG — `grid_scroll` uses `row as isize + row_delta - top` but `col as isize - col_delta - left`; per Neovim semantics the row handling is flipped → vertical scrolling moves content the wrong direction.
- **app/src/vim/backend.rs:1292-1293** — BUG/DEADCODE — mode-message detection `row.iter().any(|c| c.text.contains("--"))` is fragile: any multi-char cell containing `--` on the last row (e.g. a `---` rule) is misrendered with a 13pt font pinned to `rect.max.y - 20.0` and its line number suppressed.
- **app/src/vim/backend.rs:265-276** — BUG/DEADCODE — negative-index handling in `apply_lines_event` is off-by-one (`-1` maps to `len` instead of `len-1`) and start/end use inconsistent formulas. (Neovim sends non-negative indices, so effectively dead.)
- **app/src/vim/backend.rs:967-1006** — UGLY — `execute_command` and `execute_lua` duplicate a ~20-line cursor-sync block; factor into `sync_cursor_from_nvim()`.
- **app/src/vim/backend.rs:1328-1331, 1443-1446** — UGLY — `font_row_height` scan repeated verbatim inside the per-row loop (O(rows²)) and again after.
- **app/src/vim/backend.rs:1686-1690** — BUG (verify) — wheel direction `if steps > 0 { "up" }` may be inverted vs egui's positive-delta-means-down; the LSP panel uses the opposite convention.
- **app/src/vim/backend.rs:1086-1092** — DEADCODE — `mouse_button` is only ever called with `0`; right/middle-click arms are unreachable.
- **app/src/vim/client.rs:43-46, 60-64** — HARDCODE/cross-platform — `APPDATA` returns `None` on Linux/macOS, so the user `init.lua` is only ever loaded on Windows.
- **app/src/vim/client.rs:397-410** — DEADCODE — `set_buffer_text` has no callers (only `set_buffer_text_async` is used).
- **app/src/vim/client.rs:230-234** — DEADCODE — `set_repaint_context` (client + backend wrapper) has no callers.
- **app/src/vim/client.rs:477-489** — UGLY — reaper thread spawned-and-forgotten inside `Drop`.
- **app/src/vim/state.rs:47, 56** — DEADCODE (minor) — redundant `truncate` after `resize_with` (which already shrinks).
- **app/src/vim/lsp_panel.rs:7 / backend.rs:1519** — HARDCODE/duplication — `MAX_ROWS = 12` declared in two modules.
- **app/src/vim/lsp_panel.rs:127-134 vs 201-209** — UGLY — panel geometry duplicated between `paint` and `contains`; can diverge.

### Database / persistence
- **core/src/db/connection.rs:83** — BUG — `reviews.card_id ... REFERENCES cards(id) ON DELETE CASCADE` is declared but `PRAGMA foreign_keys = ON;` is never issued (grep confirms no `foreign_keys` anywhere) → SQLite defaults FKs OFF, so deleting a card leaves orphaned reviews and CASCADE is inert.
- **core/src/db/connection.rs:47-49** — BUG — backup filename `mindforge_backup_{%Y%m%d_%H%M%S}` has only second precision; `VACUUM INTO` fails if the target exists, so two backups within the same second error out.
- **core/src/db/connection.rs:24** — HARDCODE/BUG — `Ok(PathBuf::from("mindforge.db"))` silently writes the DB into the CWD when `ProjectDirs` fails.
- **core/src/db/connection.rs:32-36** — HARDCODE — `busy_timeout = 5000`, WAL, synchronous=NORMAL hardcoded.
- **core/src/db/connection.rs:67-141** — UGLY/BUG — no migration framework (no schema version table, no `ALTER` path); future schema changes can never be applied to existing DBs.
- **core/src/db/notes.rs:67, 90, 113** — BUG — `NaiveDateTime::parse_from_str(...).unwrap_or_default()` swallows corrupt/legacy formats and substitutes 1970-01-01, silently corrupting data.
- **core/src/db/notes.rs:64-75, 87-98, 110-121** — UGLY — identical 11-line row-mapper closure duplicated three times; extract `row_to_note`.
- **core/src/db/notes.rs:129-148** — BUG — `update_note`/`rename_note`/`delete_note` discard `execute()`'s change count → mutations on non-existent ids silently return `Ok(())`.
- **core/src/db/cards.rs:33-34 vs 61-62** — BUG — inconsistent parse fallbacks: `get_due_cards` uses `.unwrap_or(today)` (corrupt date → due today) while `get_all_cards` uses `.unwrap_or_else(Local::now())`.
- **core/src/db/cards.rs:82-123** — BUG — review + SM-2 update are non-atomic (separate statements, separate channel messages); a crash between them records a review while card state stays stale.
- **core/src/db/activity.rs:45, 95-97** — BUG — silent date-parse failure (`unwrap_or(today)` / `unwrap_or_default()`).
- **core/src/db/activity.rs:67-73** — BUG — `resolve_decision` ignores change count (no-op returns `Ok(())`) and writes `lessons: &str` as empty string where the model is `Option<String>`.
- **core/src/sm2.rs:20** — BUG — `2 | 3 => Some(Quality::Hard)` maps SM-2 quality 2 (a failure grade) to a passing `Hard`; combined with `if (quality as u8) < 3`, a rating of 2 increments reps instead of resetting the card.
- **core/src/calibration.rs:42-48** — BUG — `bucket_ranges` covers only 50-59…90-99; confidences <50 or ==100 vanish from all buckets, yet `total_resolved` counts all decisions → `sum(bucket.total) != total_resolved`.
- **core/src/calibration.rs:30, 57** — BUG — `if outcome > 0 { 1.0 } else { 0.0 }` treats any non-zero i32 (e.g. -1, 2) as true; `resolve_decision` accepts any `i32` with no validation.
- **app/src/services/db_worker.rs:96-97** — BUG — if `Database::open_default()` fails the worker thread exits immediately; subsequent `tx.send(...)` fails silently and all DB ops are lost for the session.
- **app/src/services/db_worker.rs:106-186** — BUG — every handler does `let _ = db.add_card(...)` etc.; SQLite failures (constraint violation, disk full) are silently dropped and the UI is never notified.
- **app/src/services/db_worker.rs:204-207** — BUG — `sync_setting_json` reads `settings.json`, and on any parse error falls back to `unwrap_or_default()` (empty map) then writes back → a malformed file is replaced with only the single current key/value, destroying every other stored setting.
- **app/src/app/init.rs:25** — BUG — `let _db = Database::open_default().ok();` opens, swallows the Result, and immediately drops the handle; migration failures are invisible.
- **app/src/app/init.rs:109-176** — BUG — corrupt tab JSON: `tabs_setting_present = true` but `tabs_restored = false`, so neither restore branch runs and `show_welcome` stays `false` → blank editor with no notes and no dashboard.
- **app/src/app/init.rs:499-500** — BUG — note auto-created on exit merely for having non-empty text (not dirty), surprising users who typed then hit Escape.
- **app/src/app/init.rs:437-445** — UGLY — sync DB write (`set_setting`) on the UI thread, bypassing the `db_tx` worker.
- **app/src/app/notes.rs:89-91** — UGLY — double delete: `db.delete_note(id)` (sync) *and* `db_tx.send(DeleteNote)` (async) → redundant I/O + potential race.
- **app/src/app/notes.rs:59-68, 175-184** — BUG — unbounded `notes_list` growth (inserts at 0 without trimming to `sidebar_notes_limit`).
- **app/src/app/notes.rs:139-144** — BUG — last doc-tab close leaves stale `open_doc_tabs`/`active_doc_tab` entries.
- **app/src/app/notes.rs:269** — BUG risk — `delta_secs: secs as u32` from `f64`; values above `u32::MAX` saturate and fractional seconds truncate silently.

### Agent / AI
- **app/src/agent/mod.rs:160** — BUG — `let _ = self.worker.tx.send(AgentRequest::SendChat{…})` discards the result; if the worker thread died, `is_thinking` is never cleared and the pane is permanently stuck on "Thinking…".
- **app/src/agent/mod.rs:146** — UGLY — `if msg.content.starts_with("⚠️")` filters error notices by emoji prefix; any message starting with that emoji leaks into/ out of the API context.
- **app/src/agent/mod.rs:1, 69** — HARDCODE/doc — doc says "DeepSeek Pro" and `// Pro default`, but `deepseek-chat` is not a Pro model (options are `deepseek-chat` / `deepseek-reasoner`).
- **app/src/agent/mod.rs:69** — HARDCODE — `"deepseek-chat"` hardcoded default model in `AgentState::new()`.
- **app/src/agent/mod.rs:164** — HARDCODE — `temperature: 0.7` bare literal.
- **app/src/agent/client.rs:12, 28** — SECURITY — `obfuscate_key` XORs the API key with a hardcoded `SALT` (`b"mindforge_deepseek_salt_2026"`) and claims it is "not stored as plain text"; the salt is in the binary/source, so the key is trivially recoverable from `settings.json`/SQLite. This is obfuscation, not encryption.
- **app/src/agent/client.rs:12, 28** — UGLY — `SALT` duplicated in both functions; if they drift the round-trip silently breaks.
- **app/src/agent/client.rs:7** — UGLY — doc says "XOR + base64" but the code does XOR + **hex** encoding.
- **app/src/agent/client.rs:42** — BUG-ish — `String::from_utf8(raw).unwrap_or_default()` discards invalid UTF-8 → returns `""`, surfacing as the misleading "API key is not configured".
- **app/src/agent/client.rs:133** — HARDCODE — `let url = "https://api.deepseek.com/chat/completions";` baked in, no override.
- **app/src/agent/client.rs:142** — HARDCODE — `.timeout(Duration::from_secs(60))` bare literal.
- **app/src/agent/client.rs:147** — BUG (minor) — `if resp.status() == 200` treats any other 2xx (201, 204) as an error.
- **app/src/agent/client.rs:93-107** — UGLY — worker thread `while let Ok(req) = req_rx.recv()` has no shutdown signal; lives for the process lifetime.
- **app/src/agent/deepseek_ui.rs:314, 797** — UGLY/portability — `set_win32_clipboard` is a silent no-op on non-Windows, so Copy does nothing on macOS/Linux; the "win32" name leaks platform into the UI layer.
- **app/src/agent/deepseek_ui.rs:299, 784** — HARDCODE — `current_time - t < 1.8` "copied" feedback window duplicated as a bare literal.
- **app/src/agent/deepseek_ui.rs:380** — HARDCODE — `Duration::from_millis(120)` thinking-animation tick.

### Views / layout / rendering
- **app/src/app/right_pane.rs:152** — BUG — in `Mode::Doc` the right pane shows the *note's* outline (`self.editor.ed`) instead of the documentation buffer's (`doc_ed`); line 159 likewise passes `self.editor.ed.cur`.
- **app/src/app/scan_view.rs:41** — BUG — `serde_json::from_str(&record.findings_json).unwrap_or_default()` renders corrupted history records as "no findings" with no indication.
- **app/src/app/scan_view.rs:44-48** — BUG — replaying a historical scan fabricates `status_code: 200, response_time_ms: 0, page_size_bytes: 0` (never persisted).
- **app/src/app/terminal_drawer.rs:19** — BUG — `available_h = (editor_panel_rect.height() - divider_h).max(140.0)` differs from the layout pass (`panes.rs:29`, `.max(min_top_h + 80.0)`) → the drag knob maps `split_ratio` against a different pixel range than the one used to render.
- **app/src/app/panes.rs:433-435** — BUG — drag handler uses `available_w = (...).max(200.0)` while the layout pass uses `.max(300.0)` → the same ratio maps to different pixel splits during drag vs rest.
- **app/src/app/panes.rs:88** — BUG — `let alpha = ((self.misc.opacity * 255.0) as u8).max(225);` forces any opacity below ~0.88 to 225/255, so the opacity setting is partially ignored.
- **app/src/app/panes.rs:184-187** — DEADCODE/BUG — `TabAction::Select(idx) if idx >= note_count => {}` — clicking the synthetic "📊 Start" tab does nothing.
- **app/src/app/panes.rs:689-691** — BUG/leak risk — `TerminalPane::spawn(...).ok()` retried every frame if spawn fails (no shell available).
- **app/src/app/terminal_drawer.rs:80-82** — BUG/leak risk — same per-frame terminal respawn.
- **app/src/app/stats_view.rs:11-14** — BUG (edge) — `today_str`/`yest_str` computed from two separate `Local::now()` calls; code running exactly across midnight produces an inconsistent pair.
- **app/src/app.rs:198** — BUG — file-backed tab gets `id: 0`, colliding with the "new unsaved note" sentinel (`load_note` matches `id == 0 && n.title == topic`); a file tab titled like an untitled note can be mis-identified as the reusable pristine tab.
- **app/src/app.rs:245** — BUG — `divider_w = 12.0` here vs `11.0` in `panes.rs:54,288` — the same preview splitter modeled with two widths, skewing `effective_editor_w` used for wrapping.
- **app/src/app.rs:316-320** — BUG — word counting: `if last_ch.is_whitespace() { pending_words += 1; }` counts every whitespace keystroke as a completed word ("hello␣␣␣world" = 3 words); pasted text is never counted.
- **app/src/app/shell.rs:36 + app.rs:223** — UGLY — `apply_window_blur` called on `first_frame` in both `update()` and `draw()` (blur applied twice).
- **app/src/app/shell.rs:574** — BUG/HARDCODE — `number_columns = if show_line_numbers { 4 } else { 0 }` hardcoded; the real gutter is `digits * cw + 24` (`panes.rs:384`), so the wikilink autocomplete popup is mis-positioned for >4-digit line counts or different font metrics.
- **app/src/app/shell.rs:592** — UGLY — `position(|candidate| std::ptr::eq(candidate, line))` pointer-identity search to recover a visual-line index; `enumerate` would be clearer/safer.
- **app/src/app/shell.rs:560** — DEADCODE — `let _ = is_inserting;` immediately before `if is_inserting {`.
- **app/src/app/shell.rs:295** — DEADCODE — `search_prompt: Option<(&str,&str,usize)> = None` always `None`, threaded into `render_lunaline`.
- **app/src/app/panes.rs:480** — DEADCODE — `search_matches: Option<(&[usize],usize)> = None` always `None`, threaded into both editor renderers.
- **app/src/views/dashboard.rs:163** — BUG (edge) — `310.0f32.min(rect.width()-32.0).max(120.0)` forces buttons wider than available space for `rect.width()` in (120,152).
- **app/src/views/dashboard.rs:108-109** — BUG (minor) — `logo_start_y` clamped at top but content taller than `rect` is never clamped at the bottom → buttons render off-screen.
- **app/src/views/dashboard.rs:171, 207** — UGLY — `resp.clicked() || (hovered && primary_clicked())` double-detects the same click.
- **app/src/views/stats/cards.rs:45, 54** — BUG (minor) — `((avail_w - 3*gap)/4).max(100.0)` forces 100px/card → 4 cards + gaps overflow when `avail_w < ~436`.
- **app/src/views/stats/chart.rs:63-67** — DEADCODE — `if is_today { highlight } else if is_active { highlight }` — identical branches, so `is_today` is indistinguishable from `is_active`.
- **app/src/views/stats/format.rs:19-28 vs 3-17** — UGLY — `format_duration_u64` renders `"{:.1}h"` while `format_duration` renders `"{h}h {m}m"` — same magnitude displays differently in hero card vs journal.
- **app/src/views/scan.rs:233** — UGLY/perf — each category re-filters the entire findings list every frame (O(categories × findings)).
- **app/src/views/scan_history.rs:133-136** — UGLY/UX — single click both selects *and* opens the report (no confirm / double-click).
- **app/src/views/scan_history.rs:169-171** — UGLY — scroll-clamp formula copy-pasted from `scan.rs:291-293`.

### Input / keyboard / clipboard
- **app/src/input/global/clipboard.rs:123, 142, 166, 201** — BUG (macOS) — only `modifiers.ctrl` is checked (unlike every other handler that also accepts `modifiers.command`) → Ctrl-based clipboard shortcuts ignore Cmd on macOS.
- **app/src/input/global/editing.rs:9, 35-38** — BUG (macOS) — same Ctrl-only issue for Undo/Redo.
- **app/src/input/global/editing.rs:126** — BUG/UX — Ctrl+E mode toggle lacks the `!app.command_bar.in_command` guard every other shortcut has → fires while typing `:` commands.
- **app/src/input/global/navigation.rs:142** — HARDCODE/BUG — scan report scroll uses hardcoded step `40.0` and an arbitrary `.clamp(0.0, 5000.0)` ceiling → reports taller than 5000px can't be scrolled to the end.
- **app/src/input/global/clipboard.rs:29-35, 73-79** — UGLY — blocking retry loop (10 tries × 2ms sleep) on the UI thread.
- **app/src/input/global/clipboard.rs:91** — improper error handling — `String::from_utf16(slice).ok()` silently discards invalid UTF-16.
- **app/src/input/global/clipboard.rs:219** — improper error handling — `let _ = backend.paste(&text);` ignores the paste Result.
- **app/src/input/editor.rs:97** — UGLY/perf — `target_ed.buf.clone()` copies the entire `Vec<char>` buffer on every key event in Hybrid mode just to detect dirtiness.
- **app/src/input/global/mod.rs:63-74** — UGLY — Escape handling for wikilink autocomplete/hover duplicated here and inside the widgets themselves (double dismissal logic).
- **app/src/input/global/tabs.rs:31** — HARDCODE — Ctrl+W lacks the `!i.modifiers.alt` guard used elsewhere.

### Wikilinks / backlinks / outline
- **app/src/wikilink/mod.rs:163-165** — BUG — `find_backlinks` does not normalize `\` → `/` (unlike `resolve_wikilink` at 119-127), so `[[folder\note]]` resolves but never appears in backlinks for `folder/note`.
- **app/src/wikilink/mod.rs:165** — UGLY — `format!("/{}", clean_target)` allocated inside the per-link inner loop; `eq_ignore_ascii_case`/`strip_suffix(".md")` are ASCII-only/case-sensitive so non-ASCII or `.MD` targets silently fail.
- **app/src/wikilink/hover_wikilink.rs:269** — BUG — wheel scroll inverted: `state.scroll_y = (state.scroll_y - scroll_delta)` while ArrowDown *adds* `35.0`; in egui positive `scroll_delta.y` means scroll down, so wheel-down clamps to 0 and wheel-up scrolls down.
- **app/src/wikilink/hover_wikilink.rs:137-138** — BUG — `popup_w`/`popup_h` can go negative on windows narrower than 40px / shorter than 60px → invalid `Rect`.
- **app/src/wikilink/wikilink_autocompletion.rs:494** — BUG/UX — `Tab` treated as a confirm key and consumed globally, breaking normal focus/Tab navigation while autocomplete is active.
- **app/src/wikilink/wikilink_autocompletion.rs:480-482** — UGLY — `primary_clicked() || button_pressed(Primary)` is redundant.
- **app/src/rightsidebar/mod.rs:49-53** — BUG — backlinks cache key is only `(active_note_title, active_note_id)`; backlinks depend on *other* notes' bodies, so a new link added elsewhere doesn't refresh the panel until the user switches notes (stale cache).
- **app/src/rightsidebar/mod.rs:157** — UGLY/perf — `extract_outline_headings(ed)` re-scans the whole buffer every frame (inconsistent with the caching used for backlinks).
- **app/src/rightsidebar/outline.rs:27, 32, 56** — BUG — `char_count += line.chars().count() + 1` assumes 1-char `\n` line endings; with CRLF text `text.lines()` strips `\r\n` but only `+1` is added, so every `char_offset` drifts by one per line and JumpToChar lands early.
- **app/src/sidebar/body.rs:138-154** — UGLY/perf — per-character truncation loop calls `layout_no_wrap` once per character → O(n²) font layouts for every long title, every frame.
- **app/src/sidebar/body.rs:206-261** — UGLY — "See more"/"Show less" button blocks (~50 lines) copy-pasted, differing only in label/color.
- **app/src/sidebar/footer.rs:57** — UGLY — `resp.clicked() || (is_hovered && primary_clicked())` redundant.

### View editor / inline / preview
- **app/src/view_editor/inline/render.rs:117-135 + decorations.rs:79-90** — BUG — code-block copy-button click is handled **twice** in the same frame (pre-pass loop in `render_inline_editor` and again in `decorations::render_code_block_containers`; input state is not consumed) → one click copies twice, inserts the temp key twice, requests repaint twice.
- **app/src/view_editor/inline/classify.rs:129** — BUG — `s_trim.split('|').filter(|p| !p.trim().is_empty()).count() >= 2` classifies *any* line containing a pipe and two non-empty segments as a `TableRow` (e.g. prose `I like cats | dogs` or a URL `site.com/a|b`), and `classify_lines` groups consecutive such lines into a "table".
- **app/src/view_editor/inline/classify.rs:28-33** — BUG (minor) — horizontal rule allows mixed marker chars (`- *_-`); GFM requires a single repeated character.
- **app/src/view_editor/inline/classify.rs:71-94** — inconsistency — task items support only `- `/`* ` prefixes, but bullets (line 97) also accept `+ ` → `+ [ ] task` is not a checkbox.
- **app/src/view_editor/preview/parser.rs:430-441** — BUG — `***bold italic***` mis-parsed: the bold check runs before italic and matches the leading `**`, so the bold body includes the third `*`; preview has no BoldItalic support (unlike the inline editor).
- **app/src/view_editor/preview/parser.rs:214-231** — BUG (inconsistency) — checkbox requires trailing space (`strip_prefix("- [ ] ")`), so a line ending in `- [ ]` falls through to the `"- "` bullet branch and renders literal `[ ]` text; the inline editor handles both forms.
- **app/src/view_editor/preview/parser.rs:88-101** — BUG (missing feature) — Setext headings (`Title\n===`) render as paragraph text, and code fences only recognize ` ``` ` (not `~~~`).
- **app/src/view_editor/preview/parser.rs:459-470** — BUG (minor) — only single-backtick code spans; double-backtick spans `` `code` `` can't match.
- **app/src/view_editor/preview/parser.rs:291-399** — UGLY/inconsistency — `substitute_ligatures` is a near-copy of `ligatures.rs::detect_ligature` but omits `('=','=')` (DoubleEquals) → the editor draws `==` as a ligature while the preview leaves `==` unsubstituted.
- **app/src/view_editor/preview/render.rs:316** — BUG — `(bullet.len() as f32 * font_size * 0.58).max(18.0)` uses byte length as char count; for the `"•"` bullet (3 UTF-8 bytes) the width is computed 3× too wide, misaligning the bullet column.
- **app/src/view_editor/preview/syntax.rs:87, 100, 159, 161, 180, 190, 193** — HARDCODE — One-Dark token colors are fixed `Color32::from_rgb(...)` and ignore `Theme`, so light themes get dark-theme token colors.
- **app/src/view_editor/preview/syntax.rs:112-114** — BUG (minor) — a trailing `!` after a word is consumed into the word (`a!=b` tokenizes `a!` as a macro-colored word).
- **app/src/view_editor/preview/syntax.rs:95** — BUG (minor) — number scan accepts multiple dots (`1.2.3` becomes one token).
- **app/src/view_editor/inline/charmap.rs:46-52** — BUG (latent) — the 1:1 char_map invariant is only checked with `debug_assert_eq!` (compiled out in release); a caller bug silently corrupts cursor hit-testing in release builds.
- **app/src/view_editor/inline/charmap.rs:58-62** — DEADCODE — `glyph_count()` marked `#[allow(dead_code)]`, no callers.
- **app/src/view_editor/inline/layout.rs:149 vs render.rs:91-93** — HARDCODE/fragile — duplicated magic-number width math that only matches because `wrap_w = width - 24`; a future change to either side silently desyncs table cell layout from the drawn container.
- **app/src/view_editor/inline/layout.rs:82** — DEADCODE — `compute_inline_layout_ctx` takes `ed_origin_x` and ignores it.
- **app/src/view_editor/inline/layout.rs:162** — HARDCODE — `indent_px = indent_spaces * (base_font_size * 0.55)` hardcoded 0.55em average char width.
- **app/src/view_editor/inline/parser.rs:1-6** — DEADCODE/UGLY — pure re-export bridge with `#[allow(unused_imports)]`.
- **app/src/view_editor/inline/spans.rs:196-243** — UGLY — autolink/HTML branch is a deeply nested if/else-if with two nearly identical span-push blocks.
- **app/src/view_editor/inline/render.rs:421-427** — UGLY — `Code` and `Quote` match arms both return `theme.surface()`; identical arms could be collapsed.
- **app/src/view_editor/inline/render.rs:406** — UGLY/perf — `total_glyphs` recomputed per ligature inside the per-ligature `while` loop; hoist per line.
- **app/src/view_editor/inline/render.rs:445** — HARDCODE — `Color32::from_rgb(97,175,239)` ligature color ignores the theme.
- **app/src/view_editor/inline/render/active.rs:171-228** — UGLY — five copy-pasted match arms (`Code`/`Bold`/`Italic`/`BoldItalic`/`Strike`), duplicated in `inactive.rs:225-257`.
- **app/src/view_editor/inline/render/active.rs:127-129** — UGLY — comment says "Reserve clean transparent space for the vector checkbox widget (22px)" but the code appends `"   "` (3 spaces); the "22px" is a stale magic number.
- **app/src/view_editor/inline/render/inactive.rs:50** — HARDCODE/inconsistency — Setext underline metrics differ wildly between active (`base_font_size*0.85`, `1.3`) and inactive (`0.5`, `4.0`).
- **app/src/view_editor/inline/render/inactive.rs:165-168, 179** — HARDCODE — table-cell magic numbers incl. `approx_text_w = char_count * (font_size * 0.52)` which misaligns proportional-font columns.
- **app/src/view_editor/inline/render/inactive.rs:207** — DEADCODE — unreachable `else` branch (`classify_line` never returns `prefix_len > n`).
- **app/src/view_editor/inline/render/inactive.rs:268-280** — UGLY — WikiLink rendering duplicated verbatim from `active.rs:234-246`.
- **app/src/view_editor/inline/render/inactive.rs:284** — HARDCODE — hardcoded image placeholder emoji `"🖼 "`.
- **app/src/view_editor/inline/render/decorations.rs:92** — `last_copied.unwrap()` is safe (guarded by `is_copied` at line 71) but an `expect("...")` would be clearer.
- **app/src/view_editor/ligatures.rs:256, 275** — HARDCODE — near-black `Color32::from_rgb(10,12,16)` for block-cursor char and ligature contrast stroke regardless of theme; on a light theme the cursor char can become invisible against the accent block.
- **app/src/view_editor/ligatures.rs:96-103, 161, 215-222** — HARDCODE (minor) — dense hand-tuned geometry constants, undocumented.
- **app/src/view_editor/ligatures.rs:356** — BUG (test) — comment says "find !=" but the assertion expects `NotTripleEquals`; the guard `&& chars.get(arrow_pos).is_some()` is always true (dead condition).
- **app/src/view_editor/body.rs:152-153** — BUG (latent) — `visual_row_col` returns `(0,0)` for an empty slice, which would panic here; safe today only because `compute_visual_lines` always returns ≥1 line.
- **app/src/view_editor/body.rs:106-148** — UGLY — triple copy-pasted mouse-handling blocks (`primary_pressed`, `is_decidedly_dragging`, `primary_clicked`).
- **app/src/view_editor/body.rs:84** — SAFE (minor) — division by `lh` would produce garbage if `lh` were 0.0 (never is in practice).
- **app/src/view_editor/tabs.rs:64** — BUG — tab width estimate uses byte length `display_title.len() as f32 * 7.0` instead of char count → non-ASCII titles get wildly wrong widths.
- **app/src/view_editor/tabs.rs:77** — BUG (minor) — `total_content_w = current_offset - tab_gap + initial_pad` double-counts `initial_pad`, inflating `max_scroll` by 6px.
- **app/src/view_editor/tabs.rs:102** — BUG (minor) — horizontal wheel scroll `*scroll_offset -= delta * 2.0` is inverted (wheel-right should increase offset).

### Caret / effects
- **app/src/caret.rs:13-31** — DEADCODE — nine `pub use … as …` aliases wrapped in `#[allow(unused_imports)]`; pointless re-exports.
- **app/src/caret.rs:133-142** — DEADCODE/UGLY — `resolve_caret_kind` ignores `input_mode` and `_vim_mode` (both arms return `custom_kind`); the match and parameter are dead.
- **app/src/caret/effects.rs:33** — BUG (latent) — `1.0 - b.age / 0.12` assumes bolt lifetime `0.12` matches `caret.rs:376`; if one changes, alpha underflows (saturating cast hides it).
- **app/src/caret/snow.rs:4** — DEADCODE — `_lh` parameter never used in `emit_snow`.
- **app/src/caret/*.rs** — HARDCODE — pervasive magic timings, velocities, lifetimes, and raw RGB colors throughout `beam.rs`, `candle.rs`, `effects.rs`, `fire.rs`, `neon.rs`, `particles.rs`, `rainbow.rs`, `snow.rs`, `water.rs` (e.g. `caret.rs:242-419` magic `0.4`/`0.5`/`52.0`/`200`/`220.0`; `effects.rs:9-104` bolt geometry, matrix speed, alpha ceilings, glitch RGB pairs).

### Settings / command / UI chrome
- **app/src/setting/keybindings_tab.rs:851-869** — UGLY/perf — `is_action_modified` / `default_strokes_for_action` both call `VimKeymap::new_standard()`, rebuilding the entire ~90-entry keymap on every row, every frame (~30 full HashMap builds/frame).
- **app/src/setting/keybindings_tab.rs:871-962** — UGLY/duplication — `VimAction` hand-enumerated twice; `action_display_name` ends in `_ => "Other Action"`, silently hiding unlisted actions.
- **app/src/setting/keybindings_tab.rs:460** — HARDCODE — `text_w = (disp.len() as f32 * 7.5).max(18.0)` char-count width estimate.
- **app/src/setting/keybindings_tab.rs:73-75** — HARDCODE — magic modal geometry `460.0`/`215.0`.
- **app/src/setting/keybindings_tab.rs:543-584** — UGLY/duplication — reset-default `match active_mode { Normal => …, Visual => … }` written three times in a row.
- **app/src/setting/keybindings_tab.rs:330** — UGLY — 4-tuple `stroke_to_commit_before_switch: Option<(KeymapMode, Option<KeyStroke>, KeyStroke, VimAction)>` "soup"; should be a struct.
- **app/src/setting/keybindings_tab.rs:229-234** — UGLY — `all_actions.contains(act)` inside a loop → O(n²) dedup; use `HashSet`.
- **app/src/setting/keymap.rs:495-541** — BUG/duplication — `generate_default_keymap_json` is a hand-maintained JSON template that has drifted from `new_standard()` (omits arrow keys, Ctrl combos, `Key::Space`, operator combinations, `gg`); the on-disk `keymap.json` written on first run does not match the in-memory defaults.
- **app/src/setting/keymap.rs:206-215** — DEADCODE — `bind_normal`/`bind_visual` marked `#[allow(dead_code)]`, no callers.
- **app/src/setting/keymap.rs:219** — HARDCODE — `ProjectDirs::from("com","mindforge","mindforge")`.
- **app/src/setting/keymap.rs:431** — UGLY — `unwrap()` in non-test code (safe only because of the `count()==1` guard two lines up).
- **app/src/setting/appearance.rs:13-35** — DEADCODE/UGLY — `render_appearance_section` is a vestigial wrapper with unused params (`_selected_font`, `_font_size`, `_caret`) that only forwards to `render_theme_tab`, ignoring the imported `render_carets_tab`/`render_font_settings`.
- **app/src/setting/theme.rs:249-252** — UGLY/duplication — `lerp_color` byte-for-byte identical to `accent/accentcolor.rs:616-619`; should be one shared util.
- **app/src/setting/theme.rs:20-21** — DEADCODE — `_opacity`/`_blur_effect` params unused (the tab claims to render blur/opacity controls but does not).
- **app/src/setting/theme.rs:47,50,173,186,221,235,240** — HARDCODE — magic layout offsets and alpha literals scattered through the card renderer.
- **app/src/setting/mod.rs:17-18** — UGLY — two confusingly-named sibling modules `sound` (120 bytes) and `sounds` (6.8 KB).
- **app/src/setting/setting_panel.rs:23-45** — UGLY — `render_setting_panel` takes 17 positional parameters (needs a parameter object); the giant `match active_tab` is a god-function.
- **app/src/command/command_suggestion.rs:192-209 vs 275-283** — UGLY/duplication — Enter handler and `SuggestionAction::Execute` arm both repeat `record_history` + `showcmd.record_action` + `execute_command`.
- **app/src/command/command_suggestion.rs:131** — HARDCODE/UGLY — `return Some(1000 + (target.len() as i64 * -2))`; for targets >500 chars the prefix score goes negative and can rank below fuzzy matches.
- **app/src/command/command_suggestion.rs:390** — UGLY — `is_hovered && hover_pos().is_some()` is redundant given `is_hovered`.
- **app/src/command/dispatch/mod.rs:34** — UGLY — `trimmed[parts[0].len()..].trim()` byte-index slicing; safe only because `split_whitespace` guarantees `parts[0]` starts at offset 0.
- **app/src/command/dispatch/appearance.rs:105** — BUG/HARDCODE — stale usage string lists only 10 of the 19 `ThemeKind` variants (omits cream, latte, white, shell, cyberpunk, green, amber, ice, everforest, kanagawa).
- **app/src/command/dispatch/appearance.rs:61-77** — BUG — `:volume`/`mute`/`unmute` match the arm but run neither inner branch, returning `true` — a silent no-op that reports success.
- **app/src/command/dispatch/appearance.rs:14** — UGLY — heuristic percent-vs-fraction detection with magic thresholds.
- **app/src/command/dispatch/mode_vim.rs:8-73** — UGLY/duplication — `:vim on/off` and `:mode vim/hybrid` duplicate the same mode-switch + `SaveSetting` + status logic.
- **app/src/command/dispatch/mode_vim.rs:133** — UGLY — fragile heuristic `raw.starts_with("vim.") || raw.starts_with("require(")` to distinguish Lua from Ex commands.
- **app/src/command/dispatch/tools.rs:116** — BUG — `:quit` hard-exits the process (`std::process::exit(0)`) even when the active note is dirty, losing unsaved changes (no save prompt).
- **app/src/command/dispatch/tools.rs:126-127** — HARDCODE — magic scan defaults `delay_ms = 200`, `timeout_secs = 10`.
- **app/src/command/dispatch/tools.rs:159,166** — UGLY — parse failures silently ignored (no error surfaced).
- **app/src/command/dispatch/file_ops.rs:282-287** — BUG/UGLY — substring flag matching: `:sort min` sets both `is_numeric` ('n') and `is_case_insensitive` ('i'); `:sort run` sets `is_unique` ('u') and `is_numeric` ('n'). Should be exact-token matching.
- **app/src/command/dispatch/file_ops.rs:348-356** — UGLY — `case-insensitive` is only reported when `is_numeric` is false, so `:sort ni` mislabels itself "numeric".
- **app/src/command/dispatch/view_settings.rs:241-248** — BUG — `:live`/`inline`/`livepreview` sets `inline_mode = false` — identical to `:raw`, i.e. the opposite of its documented intent ("Switch to inline WYSIWYG editor").
- **app/src/command/dispatch/view_settings.rs:270-422** — UGLY/duplication — `handle_set` re-implements nearly every toggle already in `handle` (~150-line copy).
- **app/src/command/dispatch/view_settings.rs:21** — HARDCODE — `"synatx"` typo kept as a command alias.
- **app/src/command/input.rs:34-57** — UGLY/duplication — `handle_command_paste` vs `handle_command_text` have identical bodies.
- **app/src/ui/terminal_pane.rs:94-98** — UGLY/SECURITY — `get_shell_candidates` mutates process-global env (`std::env::set_var("HOME", …)`) as a side effect; `set_var` is unsound under Rust 2024 and not thread-safe.
- **app/src/ui/terminal_pane.rs:146-157, 205, 215, 237** — HARDCODE — hardcoded Windows paths (drive letters `C/D/E/F`, `"{drive}:\Program Files\Git\…"`, `"C:\tools\git\bin\bash.exe"`, `"C:\Windows\System32\wsl.exe"`, `"C:\Windows\System32\cmd.exe"`).
- **app/src/ui/terminal_pane.rs:484** — HARDCODE/BUG — `format!("[PTY-0{}: {}]", …)` manual zero-pad produces `PTY-010` for the 10th session.
- **app/src/ui/terminal_pane.rs:303** — UGLY — spawn errors swallowed with no logging.
- **app/src/ui/palette.rs:40-284** — UGLY/duplication — six near-identical `*_picker_items` functions (`theme`/`sound`/`caret`/`font`/`mode`/`luna`); should be one generic helper.
- **app/src/ui/palette.rs:288-352** — UGLY/duplication — `match_subpicker` repeats `raw.strip_prefix(">X").or_else(|| raw.strip_prefix("> X"))` six times.
- **app/src/ui/palette.rs:170** — HARDCODE — hardcoded default font id `"jetbrains_mono"`.
- **app/src/ui/showcmd.rs:88-116** — UGLY/duplication — `set_command`/`set_search` inline-duplicate the truncation logic `Self::truncate` already provides, with a subtly different `MAX_DISPLAY_CHARS - 1` bound.
- **app/src/ui/showcmd.rs:297-307** — UGLY — prefix-based `infer_kind` mislabels any pending text starting with `v`/`V`/`<` as `Visual`.
- **app/src/ui/theme.rs:154** — UGLY — `is_light() || relative_luminance(bg) > 0.5` can disagree with `kind.is_light()` once `AccentOverrides` mutates `bg`-adjacent colors; two sources of truth.
- **app/src/ui/help_panel.rs:17-137** — HARDCODE/BUG — the entire guide is one giant hardcoded string with **incorrect** shortcut docs: line 123 claims `Ctrl+D` = "Duplicate current line below" (actual: opens delete-note modal); line 130 claims `Ctrl+F` opens fuzzy search (no such handler); line 135 claims `Ctrl+H` toggles the help tab (actual: focuses sidebar).
- **app/src/ui/zoom.rs:76-80** — HARDCODE/BUG — character-width calibration uses a hardcoded 100-`M` sample and `/ 99.0`; editing the sample string silently breaks the metric.
- **app/src/ui/zoom.rs:69-84** — UGLY/perf — `editor_metrics` lays out two galleys on every call (every frame) instead of caching per font size.
- **app/src/ui/zoom.rs:52,58,105-107,123,176,179,183,192** — HARDCODE — zoom step `1.08`, wheel factor `0.002` with `0.85..1.15` clamp, fade windows, HUD font `48.0`, pill padding.
- **app/src/ui_components.rs:31** — DEADCODE — `_id_salt` param unused.
- **app/src/ui_components/icons.rs:311-335** — UGLY — stringly-typed icon dispatch; unknown ids fall through to rendering the raw id string as text (can leak debug strings into the UI).
- **app/src/ui_components/icons.rs:74-80, 134, 306** — HARDCODE — fixed palette RGB values.
- **app/src/ui_components/toggle.rs:31,63-65,89-90,97** — HARDCODE — animation time `0.15`, knob insets `2.5`/`3.0`, pill `40.0`×`22.0`, label font `12.5`.
- **app/src/accent/accentcolor.rs:605** — DEADCODE — `(resp.lost_focus() && key_pressed(Enter)) || resp.lost_focus()` ≡ `resp.lost_focus()`, so the Enter-specific check is dead; the field commits on *any* focus loss, contradicting the "Commits on Enter or when it loses focus" comment.
- **app/src/accent/accentcolor.rs:391** — DEADCODE — `let _ = alpha;` (computed then discarded).
- **app/src/accent/accentcolor.rs:616-619** — UGLY/duplication — `lerp_color` identical to `setting/theme.rs:249-252`.
- **app/src/lunaline/render.rs:87-668** — UGLY — `render_lunaline` is a ~580-line god function (mode badge, AI button, encoding, language picker, progress, cursor, word/reading-time, command input, search, status, spinner, progress bar, popup selector).
- **app/src/lunaline/render.rs:49-84** — HARDCODE — mode colors hardcoded as raw RGB.
- **app/src/lunaline/render.rs:335** — HARDCODE — `read_mins = (total_words / 200).max(1)` magic 200 WPM.
- **app/src/lunaline/render.rs:238,251,301,316,342,368,432,448,487** — HARDCODE/UGLY — nine copies of the `len() as f32 * 7.x + N` width heuristic with different `7.0`/`7.2`/`7.4`/`7.5`/`7.8` constants.
- **app/src/app/modals.rs:12-505** — UGLY — `render_modals` is a 490-line function with a 30-arm `match` on `PaletteAction`.
- **app/src/app/modals.rs:145-182** — UGLY/duplication — `ToggleBacklinks`/`ToggleOutline` near-identical 18-line blocks.
- **app/src/app/modals.rs:193-201** — BUG — `PaletteAction::ToggleAi` only ever *opens* the AI pane (`preview_open = true`); there is no close path despite the "Toggle" name.
- **app/src/app/modals.rs:199, shell.rs:367,374, panes.rs:655, input/global/panels.rs:64,109, agent/deepseek_ui.rs:438** — HARDCODE — `egui::Id::new("deepseek_prompt_input")` magic string duplicated across 7 files; should be a shared const (also hardcodes the vendor name "deepseek").
- **app/src/app/modals.rs:276-277** — HARDCODE — `set_text(":scan ")` + `cur = 6` (magic byte length of the prefix).
- **app/src/app/modals.rs:312** — DEADCODE — `PaletteAction::ShowSoundPicker => { /* handled in modals.rs */ }` does nothing and no other handler sets `>sound` from this action.
- **app/src/app/modals.rs:322,337,353,372** — UGLY — inconsistent call style (`crate::notes::update_search_results(self)` vs `self.update_search_results()`).
- **app/src/app/modals.rs:22-28** — UGLY — closure re-created per frame (clones the channel sender every frame the settings modal is open).
- **app/src/app.rs:115** — HARDCODE — `lh = (g1.size().y * 1.30).round()` magic line-height factor.
- **app/src/app.rs:114** — HARDCODE — `cw = (g100.size().x - g1.size().x) / 99.0` (100-char sample, `/99.0` divisor).
- **app/src/app.rs:144** — HARDCODE — `"note.md".to_owned()` fallback buffer name (also `runtime.rs:111`, `backend.rs:674`).
- **app/src/app.rs:265, panes.rs:357, shell.rs:533** — UGLY/duplication — gutter-width formula `(digits * (font_size * 0.55) + 14.0).max(28.0)` copy-pasted three times.
- **app/src/app.rs:283,285** — HARDCODE — `gutter_space = { 42.0 }`, `.max(15.0)`.
- **app/src/app.rs:310,324,330** — HARDCODE — timing thresholds `< 60.0`, `> 10.0`, `> 1.2` (seconds).
- **app/src/app.rs:256-281** — DEADCODE — inline-markdown branch unreachable (`inline_mode` force-set to `false` at `init.rs:354` and `view_settings.rs:242,251`; `toggle_inline_mode()` has no non-test callers).
- **app/src/app.rs:398,403,417,419** — HARDCODE — `8`/`16`/`100` ms repaint gates.
- **app/src/app.rs:401** — UGLY — comment typo `// \typed is only true on frames...` (stray tab, missing "T").
- **app/src/app/init.rs:15-18** — HARDCODE — `ProjectDirs::from("com","mindforge","mindforge")` and `"mindforge_backup"` magic strings.
- **app/src/app/init.rs:227,232 (notes.rs:233), chart.rs:38** — HARDCODE/duplication — `db.get_recent_activity(14)` 14-day window duplicated in 3+ files.
- **app/src/app/init.rs:354-358** — DEADCODE/HARDCODE — `self.editor.inline_mode = false;` plus a persisted `"inline_mode" = "false"` hard override makes all `inline_mode` code paths (`app.rs:256`, `panes.rs:342/575`) unreachable (~150 lines dead).
- **app/src/app/init.rs:376-403** — UGLY/SECURITY — `settings.json` unconditionally rewritten on every launch (ignoring errors) and persists `deepseek_api_key_enc` (XOR-with-hardcoded-salt obfuscation) next to the DB.
- **app/src/app/init.rs:528** — HARDCODE — `if p.file_name()… == Some("mindforge_backup")` duplicates the literal from line 16.
- **app/src/app/init.rs:282,293,324** — HARDCODE — `clamp(1.0,10.0)`, `clamp(0.2,1.0)`, `clamp(12.0,48.0)` bare literals.
- **app/src/app/notes.rs:39-41** — UGLY — blocking DB write (`set_setting("note_caret_{}")`) on the UI thread on every tab switch.
- **app/src/app/notes.rs:250** — HARDCODE — `if self.activity.pending_secs < 0.1 && …` bare literal.
- **app/src/app/notes.rs:320-326** — DEADCODE — `open_help_tab_index` marked `#[allow(dead_code)]`, zero callers.
- **app/src/app/notes.rs:363, shell.rs:444, init.rs:247, …** — HARDCODE/duplication — `"Untitled Note"` literal appears in ≥6 files instead of a shared constant.
- **app/src/app/panes.rs:11-710** — UGLY — `render_editor_panes` is a ~700-line god function.
- **app/src/app/panes.rs:24-31** — HARDCODE — `tab_bar_h + 60.0`, `min_top_h + 80.0`, `min_h = 80.0`.
- **app/src/app/panes.rs:135-137** — HARDCODE — accent dropdown rect `(… - 320.0).max(8.0)`, `vec2(320.0, 400.0)`.
- **app/src/app/panes.rs:342-379, 575-603** — DEADCODE — dead inline-markdown paths (require `inline_mode`, forced `false`), incl. the `dummy_dirty` hack at 576.
- **app/src/app/panes.rs:408** — UGLY — `if let (Some(_), Some(divider_rect)) = (preview_rect_opt, divider_rect_opt)` — `preview_rect_opt` discarded.
- **app/src/app/panes.rs:434** — HARDCODE — `(150.0 / available_w).min(0.45)` unexplained cap.
- **app/src/app/panes.rs:690** — UGLY — `.ok()` swallows the `anyhow` error; the `else` prints a generic message without the cause.
- **app/src/app/right_pane.rs:34** — UGLY/perf — `let target_key = (self.notes.active_note_title.clone(), self.notes.active_note_id);` clones the title every frame just to compare cache identity.
- **app/src/app/right_pane.rs:168-169** — HARDCODE — `line_h = font_size * 1.55`, `- 40.0`.
- **app/src/app/shell.rs:10-753** — UGLY — `draw` is a 744-line god function.
- **app/src/app/shell.rs:30** — HARDCODE — `Stroke::new(1.0, Color32::from_rgb(32,34,40))` bypasses the theme system.
- **app/src/app/shell.rs:50** — HARDCODE — `pos.y <= bounds.min.y + 7.0` grab-strip height.
- **app/src/app/shell.rs:150,154** — HARDCODE — `drag_x < 70.0`, `bounds.width() - 200.0`.
- **app/src/app/shell.rs:468** — HARDCODE — `if self.notes.sidebar_notes_limit >= 100 { 50 } else { 100 }`.
- **app/src/app/shell.rs:514-536** — UGLY/duplication — split-ratio rect computation and gutter-width formula (3rd copy) re-implemented as a fallback.
- **app/src/app/shell.rs:676-684, 690, 697, 707** — HARDCODE — hover-corridor values `24.0`/`80.0`/`10.0` and the `0.40` s grace window.
- **app/src/app/terminal_drawer.rs:44** — HARDCODE — `.clamp(0.12, 0.85)`.
- **app/src/notes.rs:116-119** — UGLY — four-level deep option chain.
- **app/src/notes.rs:107** — UGLY/logic — after deletion the app loads `notes_list.first()` (most-recently-created), not the list-adjacent note.
- **app/src/notes.rs:133-137** — BUG (minor) — the no-active-id branch clears `editor.ed` but leaves `active_note_title` and the open tab title stale.
- **app/src/modals/confirm.rs:6-149** — DEADCODE — entire module `#[allow(dead_code)]`, only re-exported (`modals/mod.rs:9`) and never called (`delete.rs` implements its own confirm UI).
- **app/src/modals/delete.rs:66-68** — UGLY (minor) — `doc_title.chars().count() > 36` then `take(36)` counts chars, not display width → can overflow the 480px modal for wide (CJK) glyphs.
- **app/src/modals/rename.rs:32** — UGLY — `interact_pos().unwrap_or_default()` defaults to `Pos2(0,0)`, making `clicked_outside` vacuously true if a click arrives without an interact position.
- **app/src/modals/rename.rs:66** — UGLY — uses `egui::text_edit::TextEditState::load` while `search.rs:275` uses `egui::TextEdit::load_state` — two different APIs for the same job.
- **app/src/modals/rename.rs:26** — HARDCODE — `bounds.center().y - 40.0`.
- **app/src/modals/search.rs:180-239 vs 334-390** — UGLY/duplication — keyboard `nav_enter` and mouse-click handling are copy-paste twins.
- **app/src/modals/search.rs:61** — BUG (edge) — `620.0f32.min(bounds.width() - 32.0)` goes negative if the window is narrower than 32px → invalid `Rect`.
- **app/src/modals/search.rs:69,76,83** — HARDCODE — `.min(6)`, `40.0`, `0.35` s.
- **app/src/modals/search.rs:452** — HARDCODE — `item.badge.contains("Active")` matches a magic string instead of a structured flag.
- **app/src/modals/search.rs:13-500** — UGLY — 500-line single function with a 7-level picker-detection if/else chain.
- **app/src/state/services.rs:68** — improper error handling — `core::Database::open_default().ok()` silently discards DB open failure.
- **app/src/state/services.rs:74** — improper error handling — `let _ = self.db_tx.send(msg);` ignores send errors (worker dead → saves silently lost).
- **app/src/state/modal.rs:58** — UGLY — `VimKeymap::load_or_init()` performs file I/O inside `Default::default()` (a side effect in a constructor that can fail silently).
- **app/src/state/misc.rs:89-90** — HARDCODE — default font `"JetBrains Mono"`, `font_size: 16.0`.
- **app/src/state/notes.rs:30,56** — HARDCODE — sidebar limit `50`/`100` toggle values.
- **app/src/state/right_pane.rs:32, state/sidebar.rs:30, state/terminal.rs:29, state/editor.rs:69** — HARDCODE — layout constants `280.0`, `260.0`, `0.35`, `0.5`.
- **app/src/state/command_bar.rs:70** — HARDCODE — history cap `200` (documented, acceptable).
- **app/src/workspace_import/drag_drop.rs:12** — UGLY/perf — `i.raw.dropped_files.clone()` allocates a `Vec` every frame even when empty.
- **app/src/workspace_import/drag_drop.rs:64,81,89** — HARDCODE — overlay alpha `140`, label `"📥 DROP TO IMPORT VAULT"`, shrink `28.0`, radius `8.0`.
- **app/src/workspace_import/import_ui.rs** — HARDCODE — dozens of magic layout numbers (`540.0`, `350.0`, `44.0`, `130.0`, `32.0`, `16.0`, `60.0`, `36.0`, `14.0`, `58.0`, `48.0`, `100.0`×`32.0`, `160.0`×`32.0`, `120.0`×`32.0`).
- **app/src/workspace_import/import_ui.rs:232** — UGLY — conditional styling via string comparison `*label == "INSERTED"` instead of an enum/flag.
- **app/src/workspace_import/import_ui.rs:140,144,262** — HARDCODE — error red `239,68,68` repeated three times.
- **app/src/workspace_import/worker.rs:15-24** — BUG — `spawn_import_worker` has no guard against a second concurrent import; two drops in quick succession race to overwrite `cancel_token`/`is_running` and both threads write to the same DB/stats.
- **app/src/workspace_import/worker.rs:77** — HARDCODE — batch size `100`.
- **app/src/workspace_import/worker.rs:98-103** — UGLY — on non-UTF-8 files the file is read twice (once as `String`, once as bytes); errors fully swallowed via `unwrap_or_default()`.
- **app/src/workspace_import/state.rs:87,91** — improper error handling — `RwLock::read().map(…).unwrap_or_default()` silently ignores lock poisoning (a panicked writer leaves the importer reporting stale/default stats forever).
- **app/src/services/font_manager.rs:157-159** — HARDCODE — `PathBuf::from(r"C:\Windows\Fonts")`, `LOCALAPPDATA` → `Microsoft\Windows\Fonts`.
- **app/src/services/font_manager.rs:239** — HARDCODE — `std::fs::read(r"C:\Windows\Fonts\seguiemj.ttf")`.
- **app/src/services/font_manager.rs:161** — HARDCODE — `ProjectDirs::from("com","mindforge","mindforge")`.
- **app/src/services/font_manager.rs:165-167** — HARDCODE — `PathBuf::from("fonts")`, `"app/assets"`, `"assets"`.
- **app/src/services/font_manager.rs:176** — UGLY/perf — `std::fs::read(&p)` loads the entire font file (potentially MBs) just to inspect the first 4 bytes; read only a small header slice.
- **app/src/services/font_manager.rs:300** — UGLY — uses egui *temp* storage (`insert_temp`) as the init guard for `ensure_editor_font` (temp storage can be cleared → re-init storms).
- **app/src/services/font_manager.rs:334** — UGLY (minor) — `assert_eq!(SUPPORTED_FONTS.len(), 5)` breaks whenever a font is added.
- **app/src/services/sound.rs:101** — HARDCODE — `let sr = 22050;` bare literal.
- **app/src/services/sound.rs:230** — HARDCODE — `const NUM_CHANNELS: usize = 4;`.
- **app/src/services/sound.rs:104-133** — UGLY/duplication — six `synthesize_*` arrays repeat the identical 3-pitch-variation pattern.
- **app/src/services/sound.rs:260** — HARDCODE/BUG — `waveOutOpen(&mut hwo, 0xFFFFFFFF, …)` (bare `WAVE_MAPPER`) opens 4 independent devices; combined with `play()` calling `waveOutReset` on the channel *before* each write (294-298), the 5th keystroke resets channel 0 which may still be playing → the advertised 4-channel overlap is much less than 4.
- **app/src/services/sound.rs:299** — UGLY — `self.headers[idx].lp_data = samples.as_ptr() as *mut u8;` stores a raw pointer to data owned by the same struct (correct only because the `Vec`s are created once in `new()`; fragile and undocumented).
- **app/src/services/blur.rs:63** — SECURITY/thread-safety — `pub static mut FOUND_HWND: HWND = null_mut();` written by the `EnumWindows` callback and read at line 151; `static mut` is unsound under concurrency (hard error in Rust 2024); not safe to call from multiple threads.
- **app/src/services/blur.rs:159** — HARDCODE — `let window_title = b"mindforge\0";` `FindWindowA` fallback stops matching if the window title ever changes.
- **app/src/services/blur.rs:172,189,263** — UGLY/unsafe — `std::mem::transmute` of function pointers.
- **app/src/services/blur.rs:265** — HARDCODE — `BlurEffect::Acrylic => (…, 0x01181818)` bare magic color.
- **app/src/services/blur.rs:276** — HARDCODE — `Attribute: 19, // WCA_ACCENT_POLICY`.
- **app/src/services/blur.rs:96-107** — DEADCODE — `DWMSBT_AUTO`, `DWMSBT_TABBLEDWINDOW`, `ACCENT_ENABLE_GRADIENT`, etc. defined but never used.
- **app/src/services/fuzzy.rs:549-550** — BUG (minor) — `s_title.or(s_snip).or(s_badge)` returns the *first* matching score rather than the *maximum*, so a high-scoring snippet/badge match is discarded whenever the title also matches with a lower score → mis-ranking.
- **app/src/services/fuzzy.rs:473-511 vs 422-457** — UGLY/inconsistency — search uses `fuzzy_match` (subsequence) but `extract_snippet` locates the match with a contiguous `windows(...)` substring comparison → a fuzzy (non-contiguous) match fails `match_pos` and the snippet falls back to the first 60 chars.
- **app/src/services/fuzzy.rs:96-409** — UGLY — `BUILTIN_COMMANDS` is a ~313-line mega-constant; several entries route to the same tab via copy-paste (e.g. `OpenSetting(SettingTab::Theme)` at 124, 145, 152, 379; "Window Opacity" and "Window Blur" both open the Theme tab).
- **app/src/services/updater.rs:88** — UGLY — `self.status.lock().map(|s| s.clone()).unwrap_or(UpdateStatus::Idle)` silently returns `Idle` if the mutex is poisoned.
- **app/src/services/updater.rs:380** — UGLY — `updater_bat.to_str().unwrap_or("mindforge_updater.bat")` falls back to a relative filename that won't resolve if the temp path is non-UTF-8.
- **app/src/services/updater.rs:119,218** — HARDCODE — `Duration::from_secs(12)` / `Duration::from_secs(300)`.
- **app/src/services/updater.rs:251** — HARDCODE — `let mut buffer = [0u8; 64 * 1024];`.
- **app/src/editor/undo.rs:12-14** — HARDCODE/UGLY — magic limit `300`, and `self.undo_stack.remove(0)` is an O(n) memmove on every snapshot once full (`VecDeque` would be O(1)).
- **app/src/editor/undo.rs:10** — UGLY — `if self.undo_stack.last() != Some(&snap)` compares full buffer clones on every keystroke (O(n) per snapshot) just to dedupe.
- **app/src/editor/visual.rs:9** — HARDCODE — `let max_cols = max_cols.max(15);` duplicates the `.max(15.0)` clamp already applied by both callers (`app.rs:285`, `panes.rs:389`) — magic 15 in three places.
- **app/src/editor/visual.rs:95-333** — UGLY — four copy-pasted function pairs (`up_visual`/`up_visual_select`, `down_visual`/`down_visual_select`, `page_up_visual`/`page_up_visual_select`, `page_down_visual`/`page_down_visual_select`) whose bodies are duplicated 8 times; only the anchor-setting line differs. Needs a `select: bool` parameter or shared helper.
- **app/src/editor/visual.rs:229-257** — INCONSISTENCY — `end_visual` trims trailing whitespace on soft-wrapped lines and resets `selection_inclusive`, while `end_visual_select` does neither → Shift+End and End behave differently beyond selection.
- **app/src/editor/lines.rs:34-50** — UGLY/INCONSISTENT — `duplicate_line` inserts char-by-char (O(n²)); the two branches leave the cursor inconsistently (no-newline branch sets `cur = next_pos`; newline branch sets `cur = next_pos.saturating_sub(1)`).
- **app/src/editor/events.rs:14,21** — UGLY — `button: u8` magic-number encoding for mouse buttons (egui provides a proper enum); undocumented.
- **app/src/editor/types.rs:13** — UGLY/INCONSISTENT — doc says `column` is a "Zero-based UTF-8 **byte** column, as used by Neovim's grid protocol", but `Editor` itself (`buf: Vec<char>`, `cur`) is **char**-indexed — two coordinate systems with no conversion helper; conversion is scattered in `vim/backend.rs`.
- **app/src/editor/backend.rs:12** — DEADCODE — `#[allow(dead_code)]` on `EditorBackend` is stale (implemented by `VimBackend`, used in `panes.rs:562`).
- **app/src/editor/backend.rs:17-29** — UGLY — `render_in_rect` is a 9-parameter god signature; `EditorResult<T> = Result<T, String>` is stringly-typed error handling.
- **app/src/editor/controller.rs:6-27** — UGLY — anemic controller: both fields are `pub` and mutated externally (`app.rs:390` writes `last_observed_mode` directly), and `set_mode` does **not** update `last_observed_mode`, so the field documented as "mode observed at the start of the previous frame" is maintained outside the controller, violating its own contract. The `mode()` getter appears unused.
- **app/src/editor/movement.rs:42,51,60,85** — DEADCODE — stale `#[allow(dead_code)]` on `home`/`end`/`up`/`down` (all are called).
- **app/src/editor/movement.rs:26-40** — BUG (latent) — `*_select` variants never reset `selection_inclusive`; if it is `true` (only tests set it today), ranges silently extend one char past the cursor, and mixing inclusive/non-inclusive motions corrupts the selection. Public mutable state with no invariant enforcement.
- **app/src/editor/movement.rs:104-131** — UGLY — `set_row_col` duplicates the column-walk block twice; also a full O(buf) scan per keypress while the (currently dead, buggy) `row_col_of_fast` exists in `mod.rs`.
- **app/src/editor/editing.rs:61-63** — HARDCODE — soft-tab dedent uses magic `4` (`let to_delete = if count % 4 == 0 { 4 } else { count % 4 };`), repeated at lines 218, 257, 275, 312, 345. Should be a named `const TAB_WIDTH`. Also only triggers for spaces — a tab-indented line falls through to single-char backspace (inconsistent tab/space handling).
- **app/src/editor/editing.rs:83-98** — UGLY — quote-dedent deletes the whole prefix char-by-char, then rebuilds and re-inserts a new prefix char-by-char (O(n²) `buf.insert` calls) instead of a single `splice`.
- **app/src/editor/editing.rs:107-112** — UGLY/Bug-prone — 16-entry copy-pasted literal list (`"- "`, `"* "`, `"+ "`, `"- [ ] "`, …). It recognizes `"- [ ]"`/`"- [x]"` without trailing space, but `toggle_checklist` (419) and `handle_enter` (642) do not — three inconsistent prefix grammars in one file.
- **app/src/editor/editing.rs:130,138** — BUG (minor) — `trimmed_full.len() >= 2` mixes byte length with char-based checks; `self.buf[line_end] == '\n'` at 138 is always true when `line_end < self.buf.len()` (redundant guard).
- **app/src/editor/editing.rs:156** — DEADCODE — `#[allow(dead_code)]` on `delete` is stale (called from `command/input.rs:122`, `input/editor.rs:149`).
- **app/src/editor/editing.rs:232,286** — UGLY — `self.selection.unwrap()` — safe but redundant unwrap immediately after `selected_range()` returned `Some`.
- **app/src/editor/editing.rs:240-253 vs 294-307** — UGLY — `indent_line`/`dedent_line` are ~95% copy-paste; should share a helper.
- **app/src/editor/editing.rs:246,300** — BUG (subtle) — break condition `if next_start > end || (next_start == end && end > start)` means a selection ending exactly at a line start excludes that line, while one ending after its newline includes the next line — the `end > start` clause is confusing and behaves inconsistently at selection boundaries.
- **app/src/editor/editing.rs:366-520** — UGLY — `toggle_checklist` is a ~155-line god function with a local enum, duplicated parsing (indent_len/line_chars computed twice: 410-428 and 440-442), and magic strings `"- [ ] "`, `"- [x] "` repeated 6+ times.
- **app/src/editor/editing.rs:499-511** — UGLY — manual `isize` delta arithmetic with casts to keep anchor/cur in bounds — fragile and obscures the invariant.
- **app/src/editor/editing.rs:691,692** — INCONSISTENCY — `parse_numbered_list` (1173-1191) and `parse_numbered_list_prefix` (1210-1221) are near-duplicates with different semantics (one requires non-empty content and rejects `".\t"`, the other accepts it).
- **app/src/editor/editing.rs:737-761, 1048-1083** — UGLY/HARDCODE — the "auto-generate table" block (`" --- |"` / `"  |"` fillers, `+2` cell offsets at 746/757/1060/1076) is copy-pasted between `handle_enter` and `table_nav_tab`.
- **app/src/editor/editing.rs:769-776** — UGLY — `for (idx, ch) in next_prefix.chars().enumerate() { self.buf.insert(insert_pos + idx, ch); }` is O(n²) char-by-char insertion; `next_prefix.chars().count()` recomputed.
- **app/src/editor/editing.rs:843-874, 883-905** — UGLY/Bug-prone — two nearly identical fence-scanning loops; `line_chars.starts_with(&['`','`','`'])` treats any line starting with 3+ backticks as a fence (no fence-length matching), and the closing fence is always inserted as `"```"` (922) even for blocks opened with `~~~` (detected at 849/889) — inconsistent fence syntax.
- **app/src/editor/editing.rs:938-1154** — UGLY — `table_nav_tab` is a ~216-line god function. The table-row predicate `contains('|') && (starts_with('|') || ends_with('|') || split('|').filter(…).count() >= 2)` is copy-pasted at lines 552, 663, 793, 807, 998, 1117; the separator predicate `contains('-') && chars().all(|c| c == '|' || c == '-' || c == ':' || c == ' ')` at 553, 569, 581, 709, 729, 999, 1044, 1118. Extract helpers.
- **app/src/editor/editing.rs:1099,1101** — HARDCODE — magic `+ 2` offsets ("skip pipe + space") repeated throughout table navigation.
- **app/src/editor/editing.rs:15** — DEADCODE — `#[allow(dead_code)]` on `insert_str` is stale (called from `app.rs:189`, `app/notes.rs:180,300`, `app/init.rs:115,189`).
- **app/src/webscan/src/lib.rs:30-38** — HARDCODE — `delay_ms: 200`, `timeout_secs: 10` magic numbers (duplicated in the test at 91-92).
- **app/src/webscan/src/lib.rs:53** — BUG — `let is_https = url.starts_with("https://");` is case-sensitive and breaks on URLs with leading whitespace or no scheme → misclassifies HTTPS pages and wrongly skips HSTS/Secure-cookie checks (also duplicated at `stats.rs:38`).
- **app/src/webscan/src/stats.rs:39-48** — BUG/HARDCODE — `protocol: Some("TLSv1.3 / TLSv1.2".into())` is a hardcoded string presented as a measured value; every HTTPS scan reports the same fake protocol while `cipher`/`issuer`/`expiry` are always `None`.
- **app/src/webscan/src/stats.rs:23** — HARDCODE — `.redirect(reqwest::redirect::Policy::limited(10))` hardcodes 10.
- **app/src/webscan/src/stats.rs:19** — UGLY (minor) — `rate_limit::throttle(opts.delay_ms)` sleeps before the client is even built, so the delay is wasted if the build then fails.
- **app/src/webscan/src/stats.rs:51** — UGLY — `resp.bytes()` loads the entire response into memory with no size cap; a huge page can exhaust memory.
- **app/src/webscan/src/injection.rs:14** — HARDCODE — `None => abs_start + 500.min(lower_body.len() - abs_start)` uses an unexplained 500-char window when `</form>` is absent.
- **app/src/webscan/src/injection.rs:21-24** — DEADCODE — `_token` and `authenticity_token` are both subsumed by the earlier `form_slice.contains("token")`, making those two conditions unreachable-as-distinct checks.
- **app/src/webscan/src/injection.rs:18** — BUG — only `method="post"`/`method='post'` are matched; unquoted `method=post` and `method = "post"` (whitespace) variants are missed → false negatives.
- **app/src/webscan/src/injection.rs:33** — BUG — `break; // Report once per page` suppresses findings for additional POST forms missing CSRF tokens.
- **app/src/webscan/src/headers.rs:11** — BUG — `if !v_str.contains("max-age")` flags a valid HSTS header using `Max-Age=` (directive names are case-insensitive per RFC 6797) as missing max-age → false positive.
- **app/src/webscan/src/headers.rs:41** — BUG — `if !headers.contains_key("x-frame-options") && !headers.contains_key("content-security-policy")` suppresses the clickjacking finding for any CSP, even one lacking `frame-ancestors` (the only CSP directive that prevents framing) → false negative.
- **app/src/webscan/src/headers.rs:52** — UGLY — `val.to_str().unwrap_or("")` treats non-UTF8 header values as empty string and flags them; acceptable but silent.
- **app/src/webscan/src/cookies.rs:14-16** — DEADCODE — `if parts.is_empty() { continue; }` can never be true because `str::split` always yields at least one element.
- **app/src/webscan/src/cookies.rs:18** — DEADCODE — `parts[0].split('=').next().unwrap_or("unknown")` — `next()` on `Split` is always `Some`, so the fallback is unreachable.
- **app/src/webscan/src/cookies.rs:22,32,42** — BUG — `!lower.contains("httponly")`, `!lower.contains("secure")`, `!lower.contains("samesite")` match substrings anywhere: a cookie named `secure_session=x` or a value containing "secure"/"httponly" falsely satisfies the check and suppresses real missing-flag findings.
- **app/src/webscan/src/cookies.rs:49** — BUG — `lower.contains("samesite=none")` misses `SameSite = None` (spaces around `=`), and the paired `!lower.contains("secure")` re-uses the substring weakness.
- **app/src/webscan/src/outdated.rs:48** — HARDCODE — the hand-counted constant `pos + 32` should be derived from the search literal (e.g. `TAG.len()`).
- **app/src/webscan/src/outdated.rs:47** — BUG — only matches `<meta name="generator" content="` with exact attribute order and double quotes; single-quoted or reordered attributes are missed → false negative.
- **app/src/webscan/src/outdated.rs:10** — BUG — `srv.chars().any(|c| c.is_ascii_digit())` flags any `Server` header containing a digit (e.g. `Server: v2`, IP-based names), not just version banners.
- **app/src/webscan/src/disclosure.rs:16** — BUG/UGLY — `Err(_) => return findings` swallows client construction errors; the caller sees an empty result indistinguishable from "nothing found".
- **app/src/webscan/src/disclosure.rs:11** — HARDCODE — `.timeout(Duration::from_secs(opts.timeout_secs.min(5)))` overrides the user's configured timeout with an unexplained magic 5.
- **app/src/webscan/src/disclosure.rs:32,36** — UGLY — `if let Ok(resp) = client.get(&target).send()` and `resp.text().unwrap_or_default()` discard network/read errors, making failed probes indistinguishable from non-existent files.
- **app/src/webscan/src/disclosure.rs:38** — BUG — `!text.contains("<!DOCTYPE")` lets a lowercase `<!doctype` 404 page pass the `.env` validity check → false positive.
- **app/src/webscan/src/disclosure.rs:39** — BUG — `text.len() == 41` assumes exactly 40 hex chars + `\n`; a `.git/HEAD` without a trailing newline (40) or with CRLF (42) is rejected → false negative.
- **app/src/webscan/src/model.rs:7** — DEADCODE — `Category::Tls` is never produced (no scanner module emits a TLS finding; `stats.rs` fabricates `TlsInfo` but pushes no finding), yet the variant exists and the UI renders a TLS category (`app/src/views/scan.rs:206`).
- **core/src/lib.rs:6-9** — UGLY — `pub use calibration::*; pub use models::*; pub use sm2::*;` glob re-exports flatten the namespace and invite name collisions.
- **core/src/models.rs:17-24** — DEADCODE — `Review` is never constructed or read; reviews are only written (`record_review`) and aggregated as tuples.
- **core/src/models.rs:41** — UGLY/BUG — `pub confidence: u8, // 1 - 99` documents 1-99, but nothing validates it and `calibration.rs:87` tests with 100.
- **core/src/models.rs:56** — UGLY — `pub date: String, // "YYYY-MM-DD"` pushes parsing burden to callers and admits invalid dates; `NaiveDate` would be type-safe.
- **core/src/sm2.rs:17-25** — DEADCODE — `Quality::from_u8` has no callers.
- **core/src/sm2.rs:29-57** — DEADCODE — `calculate_sm2` is test-only (no production caller computes SM-2 schedules; the app never sends `UpdateCardSm2`).
- **core/src/calibration.rs:22-79** — DEADCODE — `compute_brier_score` and `generate_calibration_report` have no production callers (only tests).
- **core/src/db/cards.rs:18-19** — HARDCODE — `"INSERT INTO cards … VALUES (?1, ?2, ?3, 2.5, 0, 0, ?4)"` hardcodes `ease=2.5, interval=0, reps=0`, duplicating the schema `DEFAULT`s (connection.rs:75-77) — two sources of truth for the same constants.
- **core/src/db/cards.rs:126-128** — UGLY — `let count: usize = … r.get(0)?` reads `COUNT(*)` directly as `usize`, while `notes.rs:56` and `activity.rs:218` read `i64` then cast — inconsistent style for the same pattern.
- **core/src/db/cards.rs:132-147** — UGLY — the query does `ORDER BY day DESC LIMIT ?1` and then `results.reverse();` to produce ascending order; `ORDER BY day ASC` would be direct.
- **core/src/db/cards.rs:54,125,132** — DEADCODE — `get_all_cards`, `get_total_cards_count`, `get_reviews_per_day` have no callers.
- **core/src/db/cards.rs:25-52** — DEADCODE — `get_due_cards`'s only caller is the `mod.rs` unit test; the review pipeline that would use it is never wired up.
- **core/src/db/activity.rs:42-59 vs 92-109** — UGLY — the ~17-row `Decision` mapping closure is copy-pasted between `get_pending_decisions` and `get_all_decisions`.
- **core/src/db/activity.rs:87-115** — DEADCODE — `get_all_decisions` has no callers.
- **core/src/db/activity.rs:75-85** — DEADCODE — `get_resolved_decisions_for_calibration` has no callers; the calibration report it feeds is never generated.
- **core/src/db/activity.rs:119-127** — BUG (minor) — `get_focus` returns `QueryReturnedNoRows` if the `INSERT OR IGNORE` singleton row was ever dropped; no recovery path.
- **core/src/db/connection.rs:75,111-116** — HARDCODE — `ease REAL NOT NULL DEFAULT 2.5` and the singleton `focus` row (`CHECK (id = 1)` + `INSERT OR IGNORE INTO focus … VALUES (1, '', '')`) hardcode magic values in SQL.
- **core/src/db/connection.rs:70-138** — UGLY (perf) — no indexes: `cards.due` (queried `WHERE due <= ?1`), `reviews.card_id`, and `daily_activity(date)` have no indexes → full table scans.
- **core/src/db/notes.rs:30-31** — UGLY — `pub fn add_notes_batch(&mut self, …)` requires mutable access, but rusqlite 0.32's `Connection::transaction(&self)` takes `&self`; inconsistent with every other method and needlessly restricts callers.
- **core/src/db/notes.rs:35,48** — UGLY — manual `let mut count = 0; … count += 1;` inside the loop instead of `notes.len()`.
- **core/src/db/mod.rs:46** — HARDCODE (test-only) — `db.record_daily_activity("2026-09-21", …)` uses a fixed literal date; harmless but brittle-looking.
- **core/src/db/scans.rs:17-24** — BUG (minor) — `findings_json` is stored and returned as opaque `TEXT` with no validation, so a corrupt string propagates silently to the UI. Otherwise this file is clean (fully parameterized).
- **app/src/services/db_worker.rs:18** — DEADCODE — `DbMsg` is marked `#[allow(dead_code)]`, and no sender ever dispatches `AddCard`, `RecordReview`, `UpdateCardSm2`, `SaveFocus`, `AddDecision`, or `ResolveDecision` (only `SaveSetting`, `SaveNote`, `UpdateNote`, `RenameNote`, `DeleteNote`, `FlushActivity` are sent). Consequently `add_card`, `get_due_cards`, `record_review`, `update_card_sm2`, `calculate_sm2`, `Quality`, `add_decision`, `resolve_decision`, and the entire `calibration.rs` module have **zero production callers** — they exist only for their own unit tests.
- **Cross-cutting — the entire SM-2 flashcard + decision/calibration subsystem is unwired dead code.**
- **Cross-cutting — inline-markdown rendering is fully dead code** (`inline_mode` force-disabled at `init.rs:354` and `view_settings.rs:242,251`), making `app.rs:256-281`, `panes.rs:342-379`, `panes.rs:575-603` unreachable (~150 lines).
- **Cross-cutting — status bar is dead**: `app/src/statusbar.rs:6-293` `render_bottom_dock` is marked `#[allow(dead_code)]` and has zero callers; the entire file is dead. Its command-input/selection-highlight/cursor-blink/search-prompt rendering (93-207) duplicates `lunaline/render.rs:361-444` almost verbatim.
- **Cross-cutting — god functions**: `shell.rs::draw` (744 lines), `panes.rs::render_editor_panes` (~700), `lunaline/render.rs::render_lunaline` (580), `modals/search.rs::render_search_modal` (500), `deepseek_ui.rs::render_ai_pane` (586) + `render_chat_markdown` (478), `app/modals.rs::render_modals` (490), `vim/runtime.rs::update` (126), `editor/editing.rs::table_nav_tab` (216), `editor/editing.rs::toggle_checklist` (155), `setting_panel.rs::render_setting_panel` (17 params).
- **Cross-cutting — duplicated `lerp_color`** in three places (`setting/theme.rs:249`, `accent/accentcolor.rs:616`, plus inline `gamma_multiply`/`lerp_to_gamma` variants) — consolidate into one color util.
- **Cross-cutting — magic char-width `7.0`–`7.5`** used to estimate text pixel width in `tabs.rs:64`, `titlebar.rs:74`, `backlinks.rs:137`, `outline.rs:205`, `hover_wikilink.rs:189`, `wikilink_autocompletion.rs:598`, `keybindings_tab.rs:460`, `lunaline/render.rs` (×9) — breaks for proportional fonts and non-ASCII.
- **Cross-cutting — hardcoded Windows shell/font paths** concentrated in `terminal_pane.rs::get_shell_candidates` (which also performs a global env mutation) and `font_manager.rs` — good candidates for extraction into `services::shell` / `services::fonts` with pure functions.
- **Cross-cutting — always-`None`/dead threaded params**: `search_prompt` (`shell.rs:295`), `search_matches` (`panes.rs:480`), `render_bottom_dock` (`statusbar.rs`) — dead plumbing that should be removed.
- **Cross-cutting — Ctrl-only shortcut guards** (macOS Cmd ignored) in `clipboard.rs` and `editing.rs`, inconsistent with the rest of the codebase.
- **Cross-cutting — silent error swallowing** (`unwrap_or_default`, `.ok()`, `let _ =`) throughout `clipboard.rs`, `services.rs`, `state.rs`, `worker.rs`, `scanner.rs`, `db_worker.rs`.
- **Cross-cutting — `ProjectDirs::from("com","mindforge","mindforge")`** hardcoded in 3 files (`font_manager.rs:161`, `keymap.rs:219`, `init.rs:15-18`).
- **Cross-cutting — `"Untitled Note"`** literal in ≥6 files.
- **Cross-cutting — 14-day activity window** (`get_recent_activity(14)`) in `init.rs:227`, `notes.rs:233`, `chart.rs:38`.
- **Cross-cutting — gutter-width formula** in `app.rs:265`, `panes.rs:357`, `shell.rs:533`.
- **Cross-cutting — scan scroll-clamp formula** in `scan.rs:291`, `scan_history.rs:169`.
- **Cross-cutting — `"deepseek_prompt_input"` egui id** in 7 files.

---

## 🟡 HARDCODED VALUES — consolidated index
- **Paths**: `C:\Windows\Fonts` (`font_manager.rs:157`), `C:\Windows\Fonts\seguiemj.ttf` (`:239`), `LOCALAPPDATA\Microsoft\Windows\Fonts` (`:158-159`), `fonts`/`app/assets`/`assets` (`:165-167`), `mindforge.db` CWD fallback (`connection.rs:24`), `brain/` + `../brain/` (`docs.rs:96-99`), `settings.json` (`db_worker.rs:203`), Windows shell paths (`terminal_pane.rs:146-237`).
- **Endpoints / URLs**: `https://api.deepseek.com/chat/completions` (`agent/client.rs:133`), `https://api.github.com/repos/{repo}/releases/latest` (`updater.rs:116`), repo `Saboor-Hamedi/my_first_project` (`updater.rs:115`).
- **Identifiers**: `ProjectDirs::from("com","mindforge","mindforge")` (×3), `"mindforge_backup"` (×2), `"mindforge_updater.bat"`, `"mindforge_deepseek_salt_2026"` (×2), `"deepseek_prompt_input"` (×7), `"Untitled Note"` (×6), `"note.md"` (×3), `"deepseek-chat"`, user agent `"mindforge-updater/1.0"` (×2), window title `"mindforge\0"` (`blur.rs:159`), `CREATE_NO_WINDOW = 0x08000000` (`vim/client.rs:69`).
- **Magic numbers** (sample): `TAB_WIDTH` implied `4` (`editing.rs:61` etc.), undo limit `300` (`undo.rs:12`), `max_cols` `15` (`visual.rs:9`), `busy_timeout` `5000` (`connection.rs:32`), `NUM_CHANNELS` `4` + `WAVE_MAPPER` `0xFFFFFFFF` (`sound.rs`), `sample rate 22050` (`sound.rs:101`), `MAX_ROWS` `12` (×2), `batch size 100` (`worker.rs:77`), `history cap 200` (`command_bar.rs:70`), `read 200 WPM` (`lunaline/render.rs:335`), `8`/`16`/`100`/`120`/`300`/`400`/`800` ms timings (vim/agent), `1.30` line-height (`app.rs:115`), `0.55`/`0.52`/`0.58`/`0.7`/`7.0`–`7.8` char-width multipliers, `64 * 1024` chunk (`updater.rs:251`), `limited(10)` redirects (`stats.rs:23`), `clamp(0.12,0.85)` (`terminal_drawer.rs:44`), `clamp(1.0,10.0)`/`clamp(0.2,1.0)`/`clamp(12.0,48.0)` (`init.rs`), `0.45`/`0.88`/`0.85..1.15`/`1.08`/`0.002` (panes/zoom).
- **Colors bypassing `Theme`**: `syntax.rs` One-Dark palette, `ligatures.rs:256,275` near-black, `render.rs:445` ligature blue, `caret.rs:412-419` + `caret/*.rs` raw RGB, `lunaline/render.rs:49-84` mode colors, `shell.rs:30` border, `ui_components/icons.rs` palette, `blur.rs:265` acrylic color, `import_ui.rs` error red (×3), `ui_components.rs:46` close red, `titlebar.rs` close red.

---

## 🟢 UGLY CODE — consolidated index (top offenders)
- **God functions** (see cross-cutting list above).
- **Copy-paste duplication**: `active.rs`/`inactive.rs` span arms; `body.rs` mouse handlers; `palette.rs` six pickers; `keybindings_tab.rs` triple reset block; `command_suggestion.rs` duplicated Enter handlers; `command/input.rs` `handle_command_paste` vs `handle_command_text`; `mode_vim.rs` `:vim` vs `:mode`; `view_settings.rs` `handle` vs `handle_set`; `keymap.rs` `generate_default_keymap_json` vs `new_standard()`; `lunaline/render.rs` nine width heuristics; `notes.rs`/`activity.rs`/`cards.rs` row-mapper closures; `vim/backend.rs` `execute_command`/`execute_lua` cursor-sync; `lsp_panel.rs` `paint`/`contains` geometry; `deepseek_ui.rs` thumbs up/down, copy buttons, horizontal scroll; `sidebar/body.rs` see-more/show-less; `modals/search.rs` keyboard/mouse twins; `setting/theme.rs` vs `accentcolor.rs` `lerp_color`.
- **Stringly-typed / magic-string matching**: `icons.rs` icon dispatch, `search.rs:452` `badge.contains("Active")`, `import_ui.rs:232` `*label == "INSERTED"`, `agent/mod.rs:146` `starts_with("⚠️")`, `keybindings_tab.rs` `_ => "Other Action"`.
- **Perf smells**: `keybindings_tab.rs` rebuilds keymap every row/frame; `zoom.rs` lays out two galleys every frame; `sidebar/body.rs` O(n²) truncation; `input/editor.rs` clones the whole buffer every key event; `scan.rs` re-filters findings per category per frame; `rightsidebar/mod.rs` re-scans outline every frame; `shell.rs:592` pointer-identity search; `vim/backend.rs:1328` O(rows²) height scan; `font_manager.rs:176` reads whole font to check 4 bytes; `undo.rs:10` compares full buffer clones per keystroke; `drag_drop.rs:12` clones dropped files every frame; `right_pane.rs:34` clones the note title every frame.

---

## 🔵 SECURITY — consolidated index
- **Fake encryption of the DeepSeek API key**: XOR with a hardcoded salt (`agent/client.rs:12,28`) — trivially reversible; the key is recoverable from `settings.json`/SQLite. *Use a real keychain / OS credential store, or at minimum a per-machine random key.*
- **Updater path traversal / predictable temp path**: `updater.rs:238` interpolates unvalidated `asset_name` into a predictable temp filename. *Sanitize `asset_name`, use a random temp name, and verify a checksum/signature.*
- **Unbounded raw-memory scan (UB)**: `clipboard.rs:87-89` `while *ptr.add(len) != 0` with no length cap. *Use `GetClipboardData` + `GlobalSize` to bound the scan.*
- **Global env mutation**: `terminal_pane.rs:94-98` `std::env::set_var("HOME", …)` inside a getter (unsound under Rust 2024, not thread-safe). *Return candidates purely; set env only in the spawn path with proper synchronization.*
- **`static mut` shared state**: `blur.rs:63` `FOUND_HWND` written by an `EnumWindows` callback and read elsewhere — unsound under concurrency. *Use an `AtomicPtr`/`Mutex`.*
- **Arbitrary `dofile` of user config**: `vim/client.rs:355-384` re-`dofile`s whatever sits at the config path (acceptable for the user's own file, but unvalidated).
- **Unvalidated JSON blob**: `core/src/db/scans.rs:17-24` stores/returns `findings_json` as opaque TEXT with no validation.

---

## ⚫ DEAD CODE — consolidated index
- Entire SM-2 flashcard + decision + `calibration.rs` subsystem (see `db_worker.rs:18` note) — unwired, test-only.
- `inline_mode` force-disabled → `app.rs:256-281`, `panes.rs:342-379`, `panes.rs:575-603` (~150 lines).
- `app/src/statusbar.rs` whole file (`render_bottom_dock`).
- `app/src/modals/confirm.rs` whole module.
- `vim/client.rs::set_buffer_text` (`:397`), `set_repaint_context` (`:230` + backend wrapper).
- `editor/backend.rs` `EditorBackend` `#[allow(dead_code)]` (stale).
- `editor/movement.rs` `home`/`end`/`up`/`down` `#[allow(dead_code)]` (stale).
- `editor/editing.rs` `insert_str` (`:15`) and `delete` (`:156`) `#[allow(dead_code)]` (stale).
- `editor/visual.rs::caret_cell` ignores `_vim_mode` (dead param).
- `caret.rs` nine `pub use` aliases; `caret.rs::resolve_caret_kind` dead match; `caret/snow.rs` `_lh`.
- `charmap.rs::glyph_count`, `inline/layout.rs` `_ed_origin_x`, `inline/parser.rs` re-export bridge, `inline/render/inactive.rs:207` unreachable else.
- `setting/appearance.rs::render_appearance_section` (vestigial), `setting/theme.rs` `_opacity`/`_blur_effect`, `setting/keymap.rs::bind_normal`/`bind_visual`.
- `app/modals.rs::ShowSoundPicker` no-op arm; `app/notes.rs::open_help_tab_index`; `app/panes.rs` `search_matches` + `app/shell.rs` `search_prompt` dead plumbing; `app/shell.rs:560` `let _ = is_inserting`.
- `accentcolor.rs:605` dead Enter check, `:391` discarded `alpha`; `ui_components.rs` `_id_salt`.
- `vim/backend.rs` `cmdline` branch (`:384`), `mouse_button` right/middle arms; `vim/state.rs` redundant truncates; `webscan/cookies.rs` unreachable branches; `webscan/model.rs::Category::Tls`.
- `core/sm2.rs::Quality::from_u8`, `calculate_sm2`; `core/calibration.rs` `compute_brier_score`/`generate_calibration_report`; `core/db/cards.rs` `get_all_cards`/`get_total_cards_count`/`get_reviews_per_day`/`get_due_cards`; `core/db/activity.rs` `get_all_decisions`/`get_resolved_decisions_for_calibration`; `core/models.rs::Review`; `blur.rs` dead constants.
- Many stale `#[allow(dead_code)]`/`#[allow(unused_imports)]` attributes throughout.

---

## 📌 Suggested fix priority
1. **Panics**: fix all UTF-8 byte-slice sites (use `char_indices`/`floor_char_boundary`), `accentcolor.rs` hex parsing, `outdated.rs` byte-offset mismatch, `clipboard.rs` unbounded scan, `scanner.rs` symlink cycles.
2. **Updater**: point at the real repo and stop treating 404 as "up to date".
3. **Memory leaks**: cache `docs.rs` instead of `Box::leak`; bound the vim event channel.
4. **Data integrity**: enable `PRAGMA foreign_keys = ON`; make review+SM-2 atomic; stop silently overwriting corrupt `settings.json`; surface DB write errors.
5. **Editor correctness**: fix `delete_word`, undo grouping, `insert_str` double-snapshot, cursor-only undo pollution, `grid_scroll` row sign.
6. **Security**: replace XOR "encryption" with a real credential store; sanitize updater temp path; bound the clipboard scan; remove `set_var`/`static mut`.
7. **Dead code**: delete or wire up the SM-2/calibration subsystem, `inline_mode` paths, `statusbar.rs`, `modals/confirm.rs`.
8. **Hardcodes**: extract shared constants (colors → `Theme`, paths, ids, `ProjectDirs`, `"Untitled Note"`, 14-day window, gutter formula, `lerp_color`).
   9. **Ugliness**: split god functions, de-duplicate the copy-pasted blocks listed above, replace magic char-width `7.x` with real glyph metrics.

---

## 🔁 SECOND PASS — ADDITIONAL FINDINGS (not in first pass)

### 🔴 NEW UTF-8 byte-slice panic sites (latent panics on multi-byte text)
- **app/src/setting/ai_engine.rs:191** — BUG — `format!("✓ Key configured ({}...{})", &clean[..4], &clean[clean.len() - 4..])` guarded only by `clean.len() > 8`; a pasted key with multi-byte UTF-8 (clipboard accepts arbitrary text) slices a non-char-boundary → panic.
- **app/src/setting/updates.rs:108** — BUG — `if notes.len() > 80 { format!("{}…", &notes[..80]) }` slices GitHub release notes at a fixed byte offset; release notes commonly contain emoji/multi-byte chars → panic.
- **app/src/setting/updates.rs:156** — BUG — `if msg.len() > 90 { format!("{}…", &msg[..90]) }` same panic risk on error strings (may contain URLs/unicode from reqwest).
- **app/src/setting/backup.rs:60** — BUG — `format!("...{}", &backup_dir[backup_dir.len() - 39..])` panics when the last 39 bytes are not a char boundary; Windows paths with non-ASCII (e.g. `C:\Users\José\…`) are realistic.
- **app/src/setting/backup.rs:142** — BUG — `format!("...{}", &s[s.len() - 29..])` same non-char-boundary panic on `last_backup_status` text.
- **app/src/setting/shortcuts.rs:334** — BUG — `format!("...{}", &path_display[path_display.len().saturating_sub(40)..])` slices the keymap path at a byte offset; non-ASCII user profile directories (common on Windows) → panic.

### 🔴 NEW bugs
- **app/src/setting/sounds.rs:54-56** — BUG — muting then unmuting always restores `SoundProfile::Thocky` (`sound.profile = SoundProfile::Thocky;`), discarding whatever profile (Clacky, Creamy, …) the user had before muting.
- **app/src/setting/shortcuts.rs:384** — BUG — `let thumb_h = (visible_h * visible_h / content_h).clamp(24.0, visible_h);` panics ("min is greater than max") whenever the list viewport is shorter than 24px (`visible_h < 24.0` while `max_scroll > 0.0`); latent today only because the modal clamps its height (`setting_container.rs:45`).
- **app/src/setting/shortcuts.rs:368** — BUG/HARDCODE — `let _ = std::process::Command::new("explorer").arg(parent).spawn();` hardcodes the Windows `explorer` (silently no-ops on macOS/Linux) and discards the `Result` with no user feedback.
- **app/src/setting/lunaline_tab.rs:73-75, 146-148, 237-239** — BUG (improper error handling) — `if let Ok(json) = serde_json::to_string(config) { on_save_setting(...) }` discards serialization errors with no log or fallback, so a toggle/style change would silently fail to persist.
- **app/src/app/build.rs:9** — BUG (improper error handling) — `let _ = res.compile();` silently ignores failure to compile the Windows resource (e.g. missing icon file), producing a shipped binary with no icon and no build warning.
- **app/src/vim/runtime.rs:76-78** — BUG (improper error handling) — `PendingVimInput::Text(text) => { let _ = backend.handle_text(&text); }` (and `paste`/`handle_key` on the next lines) discard all errors from the Neovim backend during pending-input replay, so failed replayed keystrokes go unnoticed.
- **app/src/bin/publish.rs:95-105** — BUG — `bump_patch` uses `filter_map(|p| p.parse().ok())`, so a version like `1.2.3-beta` yields `parts.len() == 2` and bails with "Unexpected version format" instead of bumping the patch segment (pre-release versions rejected).
- **app/src/bin/publish.rs:79-93, 107-116** — BUG/UGLY — `read_version` matches the first line `starts_with("version")` (also matches `[workspace.package]`/dependency lines) and keeps trailing comments (`version = "1.0" # x` → `1.0" # x`), after which `write_version`'s `replacen` silently fails to match and the version is never updated (no verification that a replacement occurred).
- **app/src/bin/publish.rs:170** — BUG (improper error handling) — `io::stdin().read_line(&mut input).ok();` silently ignores stdin failures (closed/redirected stdin), defaulting to the bumped version with no warning.
- **app/src/bin/publish.rs:143** — BUG (improper error handling) — `std::env::current_dir().expect("cannot get cwd")` panics instead of using the file's own `bail()` helper used everywhere else.
- **app/src/app/init.rs:575** — UGLY — `eprintln!` used for shutdown-save error reporting in production (no log aggregation; message only visible if launched from a terminal).

### 🔒 NEW security / misleading security claims
- **app/src/setting/ai_engine.rs:39** — SECURITY — card title reads `"API Key (Encrypted Storage)"`, but storage is `obfuscate_key()` (reversible XOR with a hardcoded salt), not encryption.
- **app/src/setting/ai_engine.rs:289** — SECURITY — info text claims `"• API keys are hashed and encrypted before storing in your local settings."` — the key is neither hashed (reversible via `deobfuscate_key`) nor encrypted (XOR key is a source literal), misleading users about key safety.

### 🟡 NEW hardcodes
- **app/src/bin/publish.rs:247** — HARDCODE — `println!("  {CYAN}https://github.com/Saboor-Hamedi/my_first_project/releases/tag/{tag}{RESET}")` — the `publish` binary ALSO hardcodes the leftover `my_first_project` repo URL (same bug as `updater.rs:115`); should be one shared constant.
- **app/src/vim/runtime.rs:111** — HARDCODE — fabricates a literal `"note.md"` name for Doc-mode buffers passed to Neovim (also `app.rs:144`, `backend.rs:674`).
- **app/src/vim/runtime.rs:143** — HARDCODE — `(80, 24)` terminal fallback + `.max(20.0)`/`.max(5.0)` minimum grid dims (139-140) are bare magic numbers.
- **app/src/setting/ai_engine.rs:55, 174, 216-219, 265** — HARDCODE — setting keys `"deepseek_reveal_key"`, `"deepseek_api_key_enc"`, `"deepseek_model"` and model IDs `"deepseek_chat"`/`"deepseek-reasoner"` are string literals scattered through the render function; also URL `"platform.deepseek.com"` (184) and hint `"Paste API key (sk-...)"` (163).
- **app/src/setting/ai_engine.rs:137-138, 146, 186, 200** — HARDCODE — inline `Color32::from_rgba_unmultiplied(220, 60, 60, 24)`, `Color32::from_rgb(235, 160, 50)`, `Color32::from_rgb(46, 204, 113)` duplicate colors also hardcoded in `sounds.rs` instead of theme tokens.
- **app/src/setting/ai_engine.rs:32-300** — HARDCODE — pervasive f32 layout literals: `(panel_rect.width() - 56.0).max(320.0)`, `cur_y + 58.0`, card heights `42.0`/`54.0`/`110.0`, offsets `22.0`/`26.0`/`48.0`/`74.0`, button geometry `46.0`/`40.0`/`48.0`/`44.0`/`42.0`.
- **app/src/setting/updates.rs:107, 136-137** — HARDCODE — `.take(2)` line limit, `80`/`90` truncation widths, `1_048_576.0` MB divisor.
- **app/src/setting/updates.rs:18, 37, 61, 191, 194** — HARDCODE — `panel_rect.width() - 56.0`, `p_origin.y + 54.0`, `+ 132.0`, `210.0_f32.min(card_w)`, `p_origin.y + 266.0`.
- **app/src/setting/backup.rs:58, 200** — HARDCODE — `"Default: mindforge_backup/"` and `"mindforge_backup_YYYYMMDD_HHMMSS.db"` hardcode the backup naming convention in the UI (the backup service owns this format, so the two can drift).
- **app/src/setting/backup.rs:59-60, 141-142** — HARDCODE — truncation thresholds `42`/`39` and `32`/`29` are unexplained.
- **app/src/setting/backup.rs:39, 67, 86, 121, 124, 155, 186, 189** — HARDCODE — card heights `90.0`/`82.0`/`132.0`, gaps `105.0`/`98.0`, button geometry `card_w - 150.0`/`126.0`/`180.0`, `112.0`/`166.0`.
- **app/src/setting/carets.rs:57-61, 64-65, 125** — HARDCODE — `chip_start_y = 46.0`, `col_gap = 12.0`, `row_gap = 10.0`, `row_h = 44.0`, hardcoded 4-column grid (`/ 4.0`, `idx % 4`, `idx / 4`, `(len + 3) / 4`).
- **app/src/setting/carets.rs:169, 185** — HARDCODE — `(1.0 + t * 9.0).round().clamp(1.0, 10.0)` and `((caret.width - 1.0) / 9.0)` hardcode the 1–10 px width range in two places that must stay in sync.
- **app/src/setting/lunaline_tab.rs:57-58, 130-131, 196-198** — HARDCODE — `col_gap = 10.0`, hardcoded column counts (`/ 4.0`, `/ 3.0`), `row_h = 32.0`, `(available_w - 20.0) * 0.5`, `pill_w = 40.0`.
- **app/src/setting/tabs.rs:21-33** — HARDCODE/BUG — `pub const ALL: [SettingTab; 11]` must be kept in sync with the enum by hand; adding a variant compiles but silently omits it from the settings UI (no compile-time tie to the enum).
- **app/src/setting/shortcuts.rs:263** — HARDCODE — `row_left + 8.0 + (g_label.len() as f32 * 6.5) + 12.0` guesses label width with a hardcoded 6.5 px/char, which breaks for proportional fonts and non-ASCII labels.
- **app/src/setting/shortcuts.rs:29-32, 109-112, 182-215** — HARDCODE — `200.0f32.min(panel_rect.width() - 40.0)`, `pad_x = 20.0`, `row_h = 36.0`, `header_h = 24.0`, `content_h … + 42.0 + 24.0`.
- **app/src/setting/sounds.rs:64, 71, 89** — HARDCODE — `Color32::from_rgba_unmultiplied(220, 60, 60, 28)` / `Color32::from_rgb(220, 80, 80)` duplicate the same "danger red" hardcoded in `ai_engine.rs:137-138` instead of a shared theme token.
- **app/src/setting/sounds.rs:93-99** — HARDCODE — `available_w = panel_rect.width() - 56.0` (no `.max()`, can go negative), hardcoded `cols = 4`, `col_gap/row_gap = 10.0`, `s_row_h = 52.0`.
- **app/src/setting/sounds.rs:156-158** — HARDCODE/UGLY — `Color32::from_rgb((theme.accent.r() as f32 * 0.75) as u8, …)` hand-rolls color dimming with truncating casts; the theme already provides `lerp_to_gamma` used elsewhere.
- **app/src/setting/editor_mode.rs:31-32, 36, 87, 91** — HARDCODE — `panel_rect.width() - 56.0`, `p_origin.y + 52.0`, card height `128.0`, gap `138.0` (and `card_w` lacks a `.max()` floor, unlike sibling tabs).
- **app/src/setting/setting_container.rs:40-46, 58, 103** — HARDCODE — backdrop alpha `90`/`160`, modal clamps `(bounds.width() - 80.0).clamp(640.0, 880.0)` / `.clamp(480.0, 680.0)`, `tab_w = 180.0`, `0.35` s click-debounce.
- **app/src/setting/setting_font.rs:48** — HARDCODE — `(selected_font.is_empty() && font.id == "jetbrains_mono")` hardcodes the default font ID instead of a named constant (also `palette.rs:170`).
- **app/src/setting/setting_font.rs:128** — HARDCODE — `format!("{} • Install in Windows or drop .ttf into fonts/", font.description)` bakes in a Windows-only instruction and a relative `fonts/` path.
- **app/src/setting/setting_font.rs:159-160, 171, 187** — HARDCODE — `slider_x = origin.x + 76.0`, `slider_w = 160.0`, `(12.0 + t * 16.0).round().clamp(12.0, 28.0)` hardcode the 12–28 px range in two places.
- **app/src/app/build.rs:5-8** — HARDCODE — `res.set_icon("assets/icon.ico")` and the `ProductName`/`FileDescription`/`CompanyName` strings are hardcoded relative paths/literals (break if the crate root moves).

### 🟢 NEW ugly code
- **app/src/setting/ai_engine.rs:7-300** — UGLY — `render_ai_tab` is a ~290-line god function mixing layout math, hit-testing, clipboard access, and painting for three cards with no helpers.
- **app/src/setting/ai_engine.rs:171-175** — UGLY/perf — `key_changed` is set by `resp.changed()` (line 167), so each character typed re-obfuscates the key and calls `on_save_setting("deepseek_api_key_enc", …)` — a settings write per keypress.
- **app/src/setting/ai_engine.rs:71, 97, 130** — UGLY/duplication — the identical `resp.clicked() || (hover && ui.input(|i| i.pointer.primary_clicked()))` block (hover cursor + rect + stroke + text) is repeated 3× and also appears in `backup.rs:113`, `sounds.rs:53`, `carets.rs:118`.
- **app/src/setting/updates.rs:70-165** — UGLY — the 6-arm `match &status` duplicates the `status_card.min + vec2(16.0, …)` text-painting boilerplate in each arm; only the label/color differ.
- **app/src/setting/carets.rs:7-8** — UGLY — `/// Per-style accent colors shown as indicator dots on each caret chip.` is pasted twice verbatim above `caret_dot_color`.
- **app/src/setting/carets.rs:12-14** — UGLY — `CaretKind::Block => theme.accent, CaretKind::Beam => theme.accent, CaretKind::Underline => theme.accent` — three identical arms that should be one `_ =>`/or-pattern.
- **app/src/setting/carets.rs:148-213** — UGLY/duplication — the hand-rolled slider (label + track + active fill + thumb circle + value text) is near-identical to `setting_font.rs:147-210`; the codebase even has an unused `properties.rs` module that could host it.
- **app/src/setting/carets.rs:165** — UGLY — `i.pointer.primary_down() || i.pointer.primary_clicked()` — `primary_clicked()` implies `primary_down()` within the same frame, so the disjunction is dead logic.
- **app/src/setting/lunaline_tab.rs:61-118 vs 134-183** — UGLY/duplication — the style-chip loop and color-chip loop are structurally identical (hover check, cursor icon, click handler, stroke, rounded rect, label); only the data source differs — a shared chip-grid helper would collapse ~60 lines (same pattern again in `carets.rs` and `sounds.rs`).
- **app/src/setting/tabs.rs:51-137** — UGLY — `draw_icon` is an 86-line `match` with hand-coded vector coordinates for 11 icons (e.g. `pos2(center.x - 3.5, center.y - 6.0)`), untestable and full of magic numbers.
- **app/src/setting/editor_mode.rs:34-85 vs 89-140** — UGLY/duplication — the Hybrid card and Vim card are structurally identical ~50-line blocks (same geometry `128.0`/`138.0`/`34.0`/`17.5`/`110.0`/`14.0`, same hover/click/stroke/paint logic) differing only in text; a `render_mode_card(...)` helper would remove ~40 duplicated lines.
- **app/src/setting/setting_container.rs:23** (and `setting_panel.rs:31`) — UGLY — `theme: &mut Theme` is only ever used immutably (every child takes `&Theme`); the mutable reference propagates through two layers for no reason.
- **app/src/setting/setting_font.rs:147-210** — UGLY/duplication — the hand-rolled font-size slider is a near-line-for-line duplicate of `carets.rs:148-213` (same track/fill/thumb/label structure); should be a shared widget.
- **app/src/wikilink/wikilink_autocompletion.rs:215** — UGLY — `for j in 0..lookahead.len().saturating_sub(1)` index loop instead of iterator.
- **Cross-cutting — hand-rolled slider widget** duplicated verbatim in `carets.rs:148-213` and `setting_font.rs:147-210`, while `properties.rs` — a module clearly created to host shared widgets like this — is 100% dead code.
- **Cross-cutting — "chip grid" pattern** (hover → cursor icon → click → stroke → rounded rect → label) copy-pasted across `carets.rs`, `lunaline_tab.rs` (twice), and `sounds.rs`.
- **Cross-cutting — "danger red" `Color32::from_rgb(220, 60, 60)`** hardcoded independently in `ai_engine.rs:137-138` and `sounds.rs:64,71,89`.
- **Cross-cutting — non-char-boundary byte slicing** (`&s[..n]`, `&s[len-n..]`) now appears in `ai_engine.rs:191`, `updates.rs:108,156`, `backup.rs:60,142`, `shortcuts.rs:334` (in addition to the first-pass sites) — all latent panics on multi-byte UTF-8; `char_indices`/`graphemes` or `is_char_boundary` checks are needed.
- **Cross-cutting — ~290 `as usize`/`as f32`/`as i64`/`as u32` casts** across the workspace (truncation/`f64`→`u32` saturation risk); notable `f64`→`u32` saturation at `app/notes.rs:269`.
- **Cross-cutting — ~160 `.clone()` calls** in render/draw paths (`app/`, `lunaline/`, `view_editor/`, `sidebar/`, `rightsidebar/`) — per-frame allocation pressure; several already flagged individually.

### ⚫ NEW dead code
- **app/src/setting/properties.rs:7-190** — DEADCODE — entire module: none of the six public helpers (`render_section_header`, `render_property_row`, `render_toggle`, `render_card_frame`, `render_property_slider`, `render_choice_chip`) have any caller anywhere in the crate (grep-verified); meanwhile the tabs re-implement the same widgets inline (`carets.rs`/`setting_font.rs` sliders).
- **app/src/setting/properties.rs:70** — BUG (improper error handling) — `result.unwrap()` in `render_property_row` relies on the closure always executing; safe today only because the module is never called.
- **app/src/setting/types.rs:9-23** — DEADCODE — `pub enum VimSubMode` has zero usages anywhere in the crate (grep-verified: only the definition matches).
- **app/src/setting/types.rs:38, 106** — DEADCODE — `#[allow(dead_code)]` on `VimMotion` and `VimAction`, but both are actively used by `keymap.rs` and `keybindings_tab.rs`; the attributes will mask genuine future dead-code warnings.
- **app/src/setting/sound.rs:3** — DEADCODE — `pub use super::sounds::{render_sounds_tab, sound_wave_heights};` re-exports `sound_wave_heights` which is never consumed through this path (`setting_panel.rs:90` only uses `render_sounds_tab`); the file is a pointless indirection layer.
- **app/src/bin/publish.rs:39-42** — DEADCODE — `warn()` is marked `#[allow(dead_code)]` and never called.
- **app/src/bin/publish.rs:62-76** — DEADCODE — `run_output()` is marked `#[allow(dead_code)]` and never called.
- **app/src/services/updater.rs:352, 357, 384, 398** — BUG (verify) — four `std::process::exit(0)` calls inside the updater (in addition to `command/dispatch/tools.rs:116`); hard-exiting the process from a background update path can drop unsaved state — confirm each is intentional.


---

## 🚀 COMPREHENSIVE LSP INTEGRATION & CODEBASE HARDENING (ZERO-BUG ROADMAP)

This section provides an exhaustive, production-grade audit of MindForge's Language Server Protocol (LSP) integration and surrounding editor systems. It details root causes, systemic risks, and concrete architectural suggestions to leave zero bugs in the LSP pipeline. Strictly suggestions and analysis — no implementation code.

---

### 1. Neovim & LSP Buffer Lifecycle Architecture (Server Kill-and-Respawn Flaw)
- **Location**: `app/assets/nvim/mindforge/lsp/init.lua` (function `reset_buffer`) and `app/src/vim/backend.rs` (method `set_document_for_language`)
- **Root Cause**: MindForge forces every file tab and note to share a single Neovim buffer instance (buffer 0). When the user switches documents, `reset_buffer()` iterates over every active LSP client attached to buffer 0 and calls `client.stop()`.
- **Systemic Failure**:
  - Whenever the user switches between tabs (for example, navigating back and forth between two files), the underlying language server processes (such as `rust-analyzer`, `pyright`, or `typescript-language-server`) are forcefully terminated.
  - Upon loading the new tab, MindForge calls `refresh()`, which relaunches the entire language server process from scratch.
  - This completely destroys the language server's in-memory symbol caches, syntax trees, module dependency graphs, and type caches. The language server re-indexes the entire project on every single tab switch, pegging CPU cores and disk I/O, delaying completion, and causing intermittent frozen states.
- **Improvement Suggestions**:
  - Cease destroying running language servers during document navigation.
  - Allocate a distinct, persistent Neovim buffer handle for each open document tab using Neovim buffer allocation APIs rather than recycling a single buffer.
  - When switching tabs, simply set the active Neovim window buffer to the target document's buffer handle. Keep background language servers alive and attached across all active project files.
  - If sharing buffer handles is temporarily retained, use clean buffer detach mechanisms rather than terminating the server process, keeping server background indexing intact.

---

### 2. Neovim Buffer Renaming Failure and Silent Error Swallowing
- **Location**: `app/src/vim/backend.rs` (method `set_document_for_language`)
- **Root Cause**: When loading a document, MindForge issues an RPC request to rename buffer 0 to the target document's file path via `nvim_buf_set_name`. The response is discarded and the error is completely ignored.
- **Systemic Failure**:
  - In Neovim, assigning a buffer name that already exists in the global buffer table results in an immediate RPC error.
  - Because the error is ignored, the buffer name does not change. Neovim continues to associate buffer 0 with the previous file path.
  - Consequently, language servers receive mismatched document URIs, attach the wrong language server rules, query wrong root markers, and publish diagnostic notifications for the previous file rather than the active file.
- **Improvement Suggestions**:
  - Never discard errors from Neovim RPC requests that define editor state.
  - Before setting a buffer name, check whether a buffer with that file path already exists. If it exists, switch focus to it; otherwise, rename or allocate a fresh buffer.
  - Implement comprehensive error logging for all Neovim RPC communications so that failures are immediately visible during development and runtime.

---

### 3. Asynchronous Buffer Population vs Immediate Cursor Positioning Panic
- **Location**: `app/src/vim/backend.rs` (method `set_document_for_language`)
- **Root Cause**: Setting buffer text is dispatched asynchronously via `set_buffer_text_async`, immediately followed by an asynchronous cursor position call `set_cursor_async`.
- **Systemic Failure**:
  - Because text transfer is asynchronous, if the newly loaded document has 100 lines and the previous document had only 10 lines, requesting the cursor to jump to line 50 arrives before Neovim has processed the new lines.
  - Neovim evaluates the cursor jump against the stale 10-line buffer, throws an invalid cursor position error, and desynchronizes the internal cursor state from MindForge's visual caret.
- **Improvement Suggestions**:
  - Sequence buffer loading and cursor positioning: only dispatch the cursor position update after Neovim confirms the buffer text lines have been applied and recalculated.
  - Validate cursor row and column coordinates against the buffer line count prior to sending cursor jump commands to Neovim.

---

### 4. Stale Completion Popup State Surviving Tab Switches
- **Location**: `app/src/vim/backend.rs` (methods `set_document_for_language` and `apply_redraw_event`)
- **Root Cause**: When switching documents, MindForge resets row layouts and line metadata, but leaves popup menu tracking variables intact (including popup candidate items, metadata, documentation previews, selected indices, and screen anchor coordinates).
- **Systemic Failure**:
  - If a completion popup was open when the user switched tabs, Neovim does not emit a popup dismissal event simply because buffer lines were set.
  - As a result, completion candidates from the previous file remain floating on screen over the new document.
- **Improvement Suggestions**:
  - Explicitly clear all completion popup state variables inside `set_document_for_language`.
  - Issue an explicit menu cancellation command to Neovim whenever switching active documents or navigating away from an active editor pane.

---

### 5. Mutation of Vim `iskeyword` Corrupting Native Vim Motions
- **Location**: `app/assets/nvim/mindforge/lsp/snippets.lua` (function `attach`)
- **Root Cause**: `snippets.lua` mutates `vim.bo[bufnr].iskeyword` to append exclamation points, angle brackets, plus signs, asterisks, periods, and hashes for all markup buffers so that abbreviations can be expanded as single tokens.
- **Systemic Failure**:
  - In Vim, `iskeyword` governs how words are delineated across all motions and text objects.
  - Mutating `iskeyword` breaks core Vim movements and operator commands across HTML, JSX, TSX, and Markdown files. Commands such as `w`, `b`, `e`, `dw`, `cw`, `*`, `#`, and `ciw` stop treating periods, asterisks, and punctuation as word boundaries. For example, moving over `user.profile.name` treats the entire chain as a single monolithic word.
- **Improvement Suggestions**:
  - Completely remove all modifications to `iskeyword`.
  - Perform abbreviation matching and text replacements strictly within the LSP completion provider by providing precise replacement ranges (`textEdit.range`) rather than altering global buffer tokenization semantics.

---

### 6. Destructive Insert-Mode Enter (`<Esc>O`) Indentation Hack
- **Location**: `app/assets/nvim/mindforge/lsp/completion.lua` (function `setup_keys`)
- **Root Cause**: In Insert mode, pressing Enter is intercepted. When the cursor sits between opening and closing curly braces, it evaluates to `<CR><Esc>O`.
- **Systemic Failure**:
  - Injecting `<Esc>O` causes the editor to exit Insert mode to Normal mode and re-enter Insert mode with an open-line command.
  - This fragments the editor's undo tree into separate undo records, triggers Normal mode event hooks, resets temporary mode states, and causes visible caret animation stutter.
  - Because this rule is evaluated globally, it triggers inside string literals, multiline comments, and regular prose.
- **Improvement Suggestions**:
  - Remove the synthetic `<Esc>O` key macro.
  - Allow Neovim's standard filetype indentation scripts and indent settings to govern block splitting, or rely on MindForge's structured pair-handling pipeline without breaking mode boundaries.

---

### 7. Input Event Pipeline Interruption and Dropped Frame Events
- **Location**: `app/src/input/mod.rs` (around line 138)
- **Root Cause**: When a completion popup is visible, MindForge intercepts the Enter key so that it accepts the completion rather than inserting a newline. However, upon handling Enter, the function executes an early return of `false`.
- **Systemic Failure**:
  - Returning early from the main input loop completely aborts event processing for the remaining events in that frame.
  - Any simultaneous input events delivered in that frame — such as typed characters, modifier releases, arrow keys, or pointer clicks — are silently discarded without execution.
- **Improvement Suggestions**:
  - Consume only the specific Enter event from the event collection rather than aborting the entire frame's event loop.
  - Allow remaining queued events in the frame to continue through standard input dispatch.

---

### 8. Spurious Redraw Commands Inside Debounced Typing Timers
- **Location**: `app/assets/nvim/mindforge/lsp/completion.lua` (function `setup_triggers`)
- **Root Cause**: Inside the 60-millisecond debounce timer for `InsertCharPre`, the timer callback invokes `vim.cmd('redraw')` before querying completion candidates.
- **Systemic Failure**:
  - Forcing a Neovim redraw from an asynchronous libuv timer thread triggers extra RPC redraw batches to the MindForge host.
  - During fast typing, this causes rapid visual screen invalidations, caret position flickering, and potential race conditions between pending keystrokes and arriving redraw packets.
- **Improvement Suggestions**:
  - Remove manual `redraw` invocations from background timers.
  - Allow Neovim's natural display engine to flush redraw updates automatically when buffer contents and completion menus are updated.

---

### 9. Complete Absence of Diagnostic Message Presentation & Metadata Engine
- **Location**: `app/src/vim/backend.rs` (lines 1455-1468) and `app/assets/nvim/mindforge/lsp/completion.lua` (lines 67-77)
- **Root Cause**: `completion.lua` disables virtual text, disables diagnostic signs, and enables only underlines. In MindForge's Rust backend, diagnostics are rendered purely by detecting underline styling on highlight attributes.
- **Systemic Failure**:
  - MindForge has zero diagnostic metadata. It does not know the diagnostic severity (Error, Warning, Info, Hint), the error message, the reporting source (such as ESLint or Pyright), or the error code.
  - Users have no way to inspect why a line is underlined. Hovering over an error shows nothing.
  - LunaLine status bar has no diagnostic counts (e.g. 0 Errors, 3 Warnings).
  - There is no diagnostic panel or shortcut to jump between project errors.
- **Improvement Suggestions**:
  - Create a dedicated Neovim-to-MindForge diagnostic bridge via RPC notifications triggered on Neovim's `DiagnosticChanged` autocmd.
  - Transmit structured diagnostic records to MindForge, including line, column, severity, message, source, and code.
  - Display diagnostic counts (errors and warnings) in the LunaLine status bar.
  - Render an interactive floating tooltip when the cursor or mouse rests over an underlined code range.
  - Provide a modal or drawer panel to browse, filter, and jump to active diagnostics across all open buffers.

---

### 10. Pointer Scroll and Click Leakage in the Completion Popup
- **Location**: `app/src/vim/backend.rs` (lines 1710-1750)
- **Root Cause**: Mouse wheel scrolling over the popup menu emits `<C-p>` or `<C-n>`, but does not mark the wheel event as consumed. Clicking an item activates `accept_popup_item`, but does not consume the pointer click event.
- **Systemic Failure**:
  - When scrolling through completion items, the editor text beneath the popup scrolls simultaneously, shifting the cursor and popup anchor.
  - Clicking on an item can bleed through to background editor elements, inadvertently repositioning the caret or triggering file navigation.
- **Improvement Suggestions**:
  - When the mouse pointer resides within the popup menu bounding rectangle, consume pointer wheel delta events so they do not propagate to the editor scroll layer.
  - Consume pointer click events immediately when selecting a completion candidate so background text does not register a press.

---

### 11. Completion Item Selection Fallback and Keystroke Race Conditions
- **Location**: `app/src/vim/backend.rs` (method `accept_popup_item`)
- **Root Cause**: When a popup item is clicked, MindForge attempts `nvim_select_popupmenu_item`. If that fails, it falls back to feeding simulated navigation keys (`<C-n>` or `<C-p>`) followed by `<C-y>` via `nvim_feedkeys`.
- **Systemic Failure**:
  - In recent Neovim versions, `nvim_select_popupmenu_item` requires four parameters including an options dictionary table. Calling it with fewer parameters can trigger the fallback.
  - Feeding raw keystrokes asynchronously races against keystrokes the user might be actively typing, interleaving typed characters into the accepted completion string and corrupting code.
- **Improvement Suggestions**:
  - Supply the complete four-parameter signature to `nvim_select_popupmenu_item` including an empty options table.
  - Never simulate keystrokes over the input queue to accept completions; execute completion resolution atomically through Neovim Lua API calls.

---

### 12. Unreliable Language Server Process Lifecycle and Arbitrary Restarts
- **Location**: `app/assets/nvim/mindforge/lsp/commands.lua` (function `:LspRestart`)
- **Root Cause**: `:LspRestart` stops all clients using `client:stop()`, and then relies on a fixed 600ms timer (`vim.defer_fn`) before re-triggering filetype autocmds.
- **Systemic Failure**:
  - On heavily loaded systems or large codebases, heavy language servers like `rust-analyzer` or `pyright` can take more than a second to release file locks, flush caches, and exit.
  - Spawning a new instance after a fixed 600ms delay causes port collisions, lock contention, or failed server startup. On fast systems, 600ms is an unnecessary lag.
- **Improvement Suggestions**:
  - Eliminate hardcoded delay timers for process lifecycles.
  - Bind restart triggers directly to the client's process exit callback (`on_exit`). Once the old server has exited cleanly, immediately spawn the replacement instance.

---

### 13. Fragile Windows `.cmd` Shim Resolution for npm Language Servers
- **Location**: `app/assets/nvim/mindforge/lsp/install.lua` (function `command`)
- **Root Cause**: On Windows, `install.lua` reads the text of `.cmd` files generated by npm and uses a regular expression to extract the target `.js` entry point script.
- **Systemic Failure**:
  - If npm, pnpm, yarn, or Node changes the format of the batch shim, or if an executable launcher binary (`.exe`) is used instead, the regular expression fails.
  - Libuv cannot spawn Windows `.cmd` files directly without the system command processor. When regex matching fails, the language server fails to launch entirely.
- **Improvement Suggestions**:
  - Resolve the server entry point by inspecting package manifests (`package.json` bin entries) within the managed directory.
  - If invoking a batch wrapper on Windows, execute via `cmd.exe /d /s /c` with proper argument escaping rather than hand-parsing batch script internals.

---

### 14. Brittle TypeScript Language Server Library Path Resolution
- **Location**: `app/assets/nvim/mindforge/lsp/install.lua` (function `typescript_lib`)
- **Root Cause**: `typescript-language-server` requires an explicit path to `tsserver.js`. MindForge searches only three hardcoded relative directory paths under its managed npm directory.
- **Systemic Failure**:
  - If TypeScript is hoisted, installed globally, or bundled within a different folder hierarchy, `tsserver.js` is not located, causing TypeScript and TSX language support to fail on initialization.
- **Improvement Suggestions**:
  - Use Node's built-in module resolution (`node -e "console.log(require.resolve('typescript/lib/tsserver.js'))"`) or walk the directory hierarchy dynamically to reliably find `tsserver.js`.

---

### 15. Inverted Vertical Grid Scrolling Coordinates
- **Location**: `app/src/vim/backend.rs` (method `apply_grid_scroll`, lines 511-524)
- **Root Cause**: In `grid_scroll`, vertical row movement is calculated using `row as isize + row_delta - top`, whereas horizontal column movement subtracts the delta.
- **Systemic Failure**:
  - According to Neovim's UI grid protocol specification, row scrolling semantics dictate content movement in the opposite direction.
  - When Neovim performs partial viewport scrolling optimizations, lines are shifted in the wrong direction, producing corrupted text blocks and duplicate lines on screen until a full redraw occurs.
- **Improvement Suggestions**:
  - Correct the row delta sign convention in `apply_grid_scroll` to strictly follow the Neovim UI specification for both dimensions.

---

### 16. Caret Drift on Multi-Byte and Tab-Indented Characters
- **Location**: `app/src/vim/backend.rs` (lines 1510-1544)
- **Root Cause**: MindForge paints its caret by multiplying `cursor_column` by a static monospace `cell_width`.
- **Systemic Failure**:
  - In Neovim, `cursor_column` represents the grid cell position. For lines containing tab stops, full-width CJK characters, emojis, or zero-width joiners, character byte counts, Unicode scalar counts, and display cell counts differ significantly.
  - Mixing byte offsets, character indices, and screen cell coordinates causes the rendered caret to drift several characters away from the actual insertion point.
- **Improvement Suggestions**:
  - Maintain a strict boundary: use Neovim's `grid_cursor_goto` cell coordinates exclusively for visual caret placement, and use Neovim's `nvim_win_get_cursor` byte offsets exclusively for text manipulation.
  - Account for multi-cell character widths by querying the cell width from Neovim's grid state rather than assuming all characters occupy exactly one unit width.

---

### 17. Duplicate Snippet Server Attachments and Memory Leaks
- **Location**: `app/assets/nvim/mindforge/lsp/snippets.lua` (function `attach`)
- **Root Cause**: `attach` is invoked whenever a buffer's filetype is set or changed. It calls `vim.lsp.start` without verifying whether a snippet server is already running on that buffer.
- **Systemic Failure**:
  - Switching between files or repeatedly resetting buffer filetypes spawns duplicate in-process language server instances.
  - This leaks Lua memory and results in duplicate snippet suggestions appearing in the completion popup menu.
- **Improvement Suggestions**:
  - Prior to calling `vim.lsp.start`, inspect active buffer clients. If a `mindforge-snippets` client is already attached to the buffer, skip starting a new instance.
  - Cleanly detach or shut down snippet servers when buffers are unloaded.

---

### 18. Unbounded RPC Event Queue in Neovim Client
- **Location**: `app/src/vim/client.rs` (lines 292-305)
- **Root Cause**: Background event reader threads push all Neovim RPC messages into an unbounded Rust `mpsc::channel`.
- **Systemic Failure**:
  - During rapid LSP diagnostic publishing, indexing progress updates, or extensive screen redrawing, thousands of events are queued in fractions of a second.
  - If the UI thread is executing a complex layout pass, this queue expands indefinitely, causing memory pressure, delayed responsiveness, and visual lag.
- **Improvement Suggestions**:
  - Bound the event channel or introduce a coalescing mechanism for redraw batches.
  - When multiple redraw packets arrive before the UI thread renders a frame, combine intermediate grid updates so that only the final frame state is rendered.

---

### 19. Duplicated LSP Panel Geometry and Inefficient Re-filtering
- **Location**: `app/src/vim/lsp_panel.rs` (lines 127-134, 201-209)
- **Root Cause**: The bounding rectangle formula for the language server browser panel is duplicated between `paint` and `contains`. In addition, `matches()` re-filters, re-allocates, and re-sorts all server rows on every render frame.
- **Systemic Failure**:
  - Changes to panel margins, row heights, or widths in one function can desynchronize hit-testing from drawing, making buttons unclickable.
  - Re-filtering all rows on every frame causes unnecessary allocations and garbage collection in the rendering pipeline.
- **Improvement Suggestions**:
  - Extract panel geometry computation into a single authoritative helper method shared by both `paint` and `contains`.
  - Cache filtered row lists, updating them only when the user's filter text changes or when server statuses update.

---

### 20. Workspace Root Detection and Monorepo Support
- **Location**: `app/assets/nvim/mindforge/lsp/registry.lua`
- **Root Cause**: Server definitions rely on rudimentary root marker arrays like `[ '.git' ]`.
- **Systemic Failure**:
  - When opening files in standalone folders, vaults without git repositories, or deeply nested monorepos (e.g. `packages/backend`), servers fail to determine the workspace root and fall back to `nil`.
  - Servers like `pyright`, `ts_ls`, and `rust-analyzer` refuse to initialize or fail to resolve relative import paths without a valid workspace root.
- **Improvement Suggestions**:
  - Explicitly pass MindForge's active project or vault root folder to Neovim upon application startup.
  - Configure root directory detection fallbacks to default to MindForge's workspace directory when no repository marker is discovered.

---

### 21. Language Intelligence Feature Gap (Definition, Hover, References, Rename, Actions)
- **Location**: `app/assets/nvim/mindforge/lsp/completion.lua` and `app/src/vim/backend.rs`
- **Root Cause**: The only mapped LSP command is `gd` (`vim.lsp.buf.definition`). All other standard language server capabilities are absent from both keyboard shortcuts and UI chrome.
- **Systemic Failure**:
  - Users have no access to hover documentation, finding references, symbol renaming, code actions, or document formatting, despite having language servers running in the background.
- **Improvement Suggestions**:
  - Map `K` to trigger interactive hover documentation (`vim.lsp.buf.hover`) rendered in a clean MindForge floating overlay.
  - Map `gr` to find references with a searchable modal list.
  - Map `<leader>rn` to symbol renaming with an egui input modal.
  - Map `<leader>ca` to trigger code action quick-fixes.
  - Provide a document format command (`:Format` or shortcut) connected to `vim.lsp.buf.format`.

---

### 22. Cross-Mode Integrity: Unifying Vim Mode, Hybrid Mode, and Markdown Viewers
- **Location**: Workspace architecture boundary across `app/src/editor/`, `app/src/vim/`, and `app/src/view_editor/`
- **Root Cause**: MindForge supports Vim mode, Hybrid mode, and inline/preview Markdown modes, but LSP language intelligence is currently isolated strictly within Neovim.
- **Systemic Failure**:
  - Switching between Hybrid mode and Vim mode on code files creates disjointed user experiences where diagnostics and completions vanish in Hybrid mode.
  - When switching modes, text buffers are copied across different representations (`Vec<char>` vs Neovim string buffers) without synchronizing undo histories or cursor positions.
- **Improvement Suggestions**:
  - Ensure mode switching cleanly transfers document text, cursor row and column, and scroll offsets without data loss or desynchronization.
  - Architect language intelligence services so that diagnostic overlays and completion can eventually serve both editing surfaces through a unified MindForge presentation layer.

---

### 23. Observability, Structured Diagnostics Logging, and Automated Verification Strategy
- **Location**: System-wide integration across Rust and Lua boundaries
- **Root Cause**: Debugging LSP and Neovim synchronization issues currently relies on inspecting ephemeral status strings or reading log files after crashes occur.
- **Systemic Failure**:
  - Race conditions, dropped events, and IPC communication errors fail silently in release builds.
- **Improvement Suggestions**:
  - Implement a configurable diagnostic trace mode that logs IPC message timestamps, event names, buffer revisions, and cursor coordinates without spamming production logs.
  - Build automated integration test harnesses that simulate document switching, rapid typing with completions, backspace deletion at boundaries, and language server crashes to ensure no regressions occur.
