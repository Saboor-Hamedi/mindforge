# MindForge — Input Routing Fix Plan

## Critical Bugs (directly cause the reported problem)

### Bug 1: Sidebar focus catch-all blocks command line
**File:** `app/src/input/global.rs:864-922`

When `sidebar.open && sidebar.focused`, line 921 returns `Some(false)` for **any key**, consuming all input. This is the **only** focus-consuming check that lacks a `!app.command_bar.in_command` guard. The doc sidebar (line 924) and scan view checks (lines 1031, 1039) already have this guard.

**Impact:** When the sidebar is focused and the user activates the command line (`:`), all subsequent keystrokes are consumed by the sidebar check. The command line never receives typing, Backspace, arrows, Home, End, Enter, or Escape.

**Fix:** Add `&& !app.command_bar.in_command` to the condition at line 864.

---

### Bug 2: Unconditional Tab consumption
**File:** `app/src/input/global.rs:1079-1082`

```rust
ctx.input_mut(|i| {
    i.consume_key(egui::Modifiers::NONE, egui::Key::Tab);
    i.consume_key(egui::Modifiers::SHIFT, egui::Key::Tab);
});
```

This runs **every frame**, consuming Tab and Shift+Tab regardless of context. This prevents Tab from reaching the command bar's suggestion completion and interferes with egui's built-in focus cycling.

**Impact:** Tab never reaches the command bar for autocomplete completion. Tab never reaches egui widgets for focus traversal.

**Fix:** Remove the unconditional `consume_key` calls. The Tab key is already handled explicitly at lines 388-445 with proper mode-specific logic.

---

## Medium Issues (could cause problems in specific scenarios)

### Issue 3: AI input priority absorbs all input
**File:** `app/src/input/global.rs:191-207`

When the AI panel is open and focused, `is_ai_focused` returns true and all input is absorbed. This could interfere with the command line if the AI panel is focused when the user types `:`.

**Fix:** Add `!app.command_bar.in_command` guard, or better yet, check `in_command` before the AI focus check.

---

### Issue 4: Terminal `prev_mode_before_term` never set
**File:** `app/src/state/terminal.rs:22`

The field exists but is never assigned when opening the terminal. It always defaults to `Mode::Normal`.

**Fix:** Set `prev_mode_before_term` when opening the terminal in `global.rs:104-113`.

---

### Issue 5: `:quit` calls `std::process::exit(0)` without cleanup
**File:** `app/src/command/dispatch.rs:1313`

No save prompt, no DB flush, no session save.

**Fix:** Call `sync_save_session()` before exiting, or use `ctx.send_viewport_cmd(ViewportCommand::Close)`.

---

## Architectural Improvements

### Improvement 1: State duplication between `App` and `EditorState`
`App` duplicates 15+ fields from `EditorState`: `scroll_y`, `doc_scroll_y`, `preview_scroll_y`, `preview_open`, `inline_mode`, `split_ratio`, `is_dragging_splitter`, `show_line_numbers`, `is_dirty`, `font_dirty`, `last_saved_time`, `last_editor_rect`, `last_ed_origin`, `last_ed_font_size`.

**Risk:** Two sources of truth can get out of sync, causing subtle bugs.

**Fix:** Remove duplicate fields from `App` and use `app.editor.xxx` instead.

---

### Improvement 2: Duplicate `open_notes` and `doc_sidebar_focused`
Both `App` and `TabsState` have `pub open_notes: Vec<OpenNote>` and `doc_sidebar_focused: bool`.

**Risk:** Critical data duplication — they can get out of sync.

**Fix:** Remove from `App` and use `app.tabs.xxx` instead.

---

### Improvement 3: No centralized focus management
Focus is tracked via boolean flags (`sidebar.focused`, `terminal.focused`, `doc_sidebar_focused`) but egui's internal focus system is only used for the AI agent. Two parallel focus systems can conflict.

**Fix:** Use egui's `request_focus`/`surrender_focus` for all focusable surfaces.

---

### Improvement 4: No command history navigation
Command history is stored but ArrowUp/ArrowDown are consumed by autocomplete navigation. No way to browse history.

**Fix:** Add Ctrl+R for history search, or use ArrowUp/Down when no suggestions are shown.

---

### Improvement 5: Inconsistent Vim mode pass-through
Some keys pass through to Neovim (Ctrl+A, Ctrl+C, Ctrl+X, Ctrl+Z, Tab, Escape), others are handled globally (Ctrl+S, Ctrl+N, Ctrl+R, Ctrl+P). Vim mode behavior depends on which key is pressed.

**Fix:** Create a consistent rule — in Vim normal mode, all non-global shortcuts pass through to Neovim.

---

### Improvement 6: No input event logging/debugging
No way to trace which handler consumed a given key event.

**Fix:** Add a debug mode that logs which handler consumed each event.

---

### Improvement 7: `window_shortcuts` called after `handle_input`
F11, Alt+drag, and resize grips are processed after all other input.

**Fix:** Move `window_shortcuts` before `handle_input`, or leave as-is (these are window-level shortcuts that should always work).

---

### Improvement 8: No keybinding customization in input routing
The `VimKeymap` exists in `ModalState` but is not used by `global.rs`. All keybindings are hardcoded.

**Fix:** Use the `VimKeymap` for input routing.

---

## Implementation Phases

### Phase 1: Critical Fixes (fixes the reported bugs)
1. Add `!app.command_bar.in_command` guard to sidebar focus check (`input/global.rs:864`)
2. Remove unconditional Tab consumption (`input/global.rs:1079-1082`)
3. Add `!app.command_bar.in_command` guard to AI input priority (`input/global.rs:193`)

### Phase 2: Important Improvements (prevents future issues)
4. Set `prev_mode_before_term` when opening terminal (`input/global.rs:104`)
5. Fix `:quit` to use proper cleanup (`command/dispatch.rs:1313`)
6. Deduplicate `open_notes` and `doc_sidebar_focused` (remove from `App`, use `app.tabs.xxx`)

### Phase 3: Architectural Improvements (long-term)
7. Deduplicate `App` and `EditorState` fields
8. Centralize focus management using egui's focus system
9. Add command history navigation
10. Create consistent Vim mode pass-through rules
11. Add input event logging/debugging
