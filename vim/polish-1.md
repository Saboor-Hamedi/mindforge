# 20 Robust Architectural & Performance Suggestions for MindForge

---

### 1. Cross-Platform Neovim Paths & Runtime Environment
- **Problem**: In `app/src/vim/client.rs` (lines 43–64), user Neovim configuration loading relies solely on `std::env::var("APPDATA")`, which resolves to `None` on Linux and macOS. Consequently, user `init.lua`, custom LSP configurations, and keymaps are never loaded on non-Windows platforms.
- **Root Cause**: Windows-specific environment variables are hardcoded instead of using standard cross-platform XDG directory specifications.
- **Robust Suggestion**: Utilize standard cross-platform directory resolution (via the `etcetera` or `directories` crate, or an internal helper):
  - Linux / macOS: `$XDG_CONFIG_HOME/mindforge/nvim` falling back to `~/.config/mindforge/nvim`.
  - Windows: `%APPDATA%\mindforge\mindforge\nvim`.
  - Export `g:mindforge_home` and prepend the resolved directory to Neovim's `&runtimepath` uniformly across all platforms during process initialization in `NeovimClient::spawn`.

---

### 2. Bounded Event Channel & Flow Control for Neovim RPC
- **Problem**: In `app/src/vim/client.rs` (lines 280–325), the background MessagePack-RPC reader thread sends incoming events to the egui main thread via an unbounded `mpsc::channel`. During heavy buffer streaming, large file loading, or rapid cursor drags, the channel backlog can grow indefinitely, causing memory spikes and delayed rendering.
- **Root Cause**: Absence of backpressure between the high-frequency Neovim stdio stream and the egui 60/120 Hz render loop.
- **Robust Suggestion**:
  - Replace the unbounded channel with a bounded `sync_channel(256)` or a ring-buffer queue with coalescing logic.
  - When consecutive `redraw` grid events arrive in the queue, coalesce redundant intermediate viewport redraws so only the latest visual grid snapshot is processed per egui frame tick.
  - Cap `drain_events()` dynamically based on frame deadline budget (e.g., max 4ms processing time per tick) to prevent frame drops.

---

### 3. Incremental Grid Line Diffing & Row Layout Caching
- **Problem**: In `app/src/vim/backend.rs` (lines 400–550) and `app/src/vim/render.rs` (lines 100–300), every redraw event invalidates row layouts across the grid. When Neovim scrolls or edits a single character, multiple egui `Galley` text layouts are recalculated from scratch.
- **Root Cause**: Row layouts are invalidated globally or keyed only by a coarse revision counter without cell-level or row-hash dirty checking.
- **Robust Suggestion**:
  - Implement a row-level content hash (`u64` per row) storing the hash of cell characters, foreground colors, background colors, and syntax highlight flags.
  - In `VimBackend`, only recreate egui `Galley` text layouts for rows whose hash changed since the previous frame.
  - For vertical scrolling events (`grid_scroll`), perform a zero-allocation circular buffer shift of existing row galleys, painting only the newly exposed rows.

---

### 4. Safe Process Lifecycle & Graceful Child Reaping
- **Problem**: In `app/src/vim/client.rs` (lines 530–550), `NeovimClient::shutdown` and `Drop` spawn an unmonitored thread (`nvim-reaper`) to execute a blocking `child.wait()`. If Neovim hangs due to a blocked child process, plugin, or socket lock, the spawned thread lingers indefinitely as a zombie thread.
- **Root Cause**: `child.kill()` does not guarantee immediate termination on Windows if the process is suspended or waiting on an uncooperative driver/pipe, and threads spawned without cancellation handles cannot be timed out.
- **Robust Suggestion**:
  - First send RPC command `:qa!` with a 150ms graceful shutdown timeout.
  - If the process fails to exit within the deadline, call `child.kill()`.
  - On Windows, assign the child process to an OS Job Object (`JobObjectAssociateCompletionPort`) with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, ensuring all child and grandchild processes (e.g. node, python, language servers) are killed atomically when MindForge exits.

---

### 5. Single Source of Truth for Buffer Content
- **Problem**: The application maintains two separate text buffers: `app.editor.ed` (Rust string buffer) and `VimBackend.lines` (Neovim internal buffer). Synchronization between them relies on periodic polling, `force_sync_text`, or save triggers in `shell.rs` and `notes.rs`, creating subtle desynchronizations during multi-tab operations or external disk updates.
- **Root Cause**: Dual-buffer design where both egui and Neovim attempt to act as primary storage for the active note.
- **Robust Suggestion**:
  - Establish a strict single-source architecture where Neovim is the sole owner of buffer text in Vim mode.
  - Subscribed Neovim buffer events (`nvim_buf_lines_event`) should stream diffs directly into `VimBackend.lines`.
  - Eliminate redundant string allocations in `app.editor.ed` while Vim mode is active; read-only operations (such as word count, reading time, wikilink parsing) should read directly from `VimBackend` slices without intermediate full-buffer copies.

---

### 6. Unified Undo Tree Bridging Between Rust Actions & Neovim
- **Problem**: Edits triggered through GUI actions (e.g. format button, markdown table navigation in `app/src/editor/editing.rs`, checklist toggle, or AI snippet insertion) push snapshots to `app.editor.ed.undo_stack`, bypassing Neovim's undo history. Pressing `u` in Normal mode fails to undo GUI edits, or reverts an unrelated earlier keystroke.
- **Root Cause**: Two completely independent undo/redo stacks that have no mutual notification or synchronization.
- **Robust Suggestion**:
  - Route all text modifications through Neovim RPC methods: use `nvim_buf_set_text` or `nvim_exec_lua` with `vim.api.nvim_buf_set_lines`.
  - Wrap multi-step GUI edits in Neovim undojoin blocks (`vim.cmd("undojoin")` or `undolevels` boundary markers) so that actions like table formatting or checklist toggling are recognized as single native undo steps by Neovim.
  - Disable the legacy Rust `undo_stack` in Vim mode to prevent desynchronized state rollbacks.

---

### 7. Atomic Safe-Saving with Write-Rename to Prevent Data Loss
- **Problem**: In `app/src/notes.rs` (lines 30–35), document saving writes directly to the destination path using `std::fs::write(&path, content)`. If an unexpected process crash, power disruption, or antivirus file lock occurs mid-write, the user's file is left truncated to 0 bytes or corrupted.
- **Root Cause**: In-place file truncation without temporary staging or atomic swapping.
- **Robust Suggestion**:
  - Implement atomic write-and-replace:
    1. Write content to a sibling temporary file in the same directory: `path.with_extension("tmp.mindforge~")`.
    2. Flush and sync file buffers to disk (`file.sync_all()`).
    3. Atomically rename/replace the temporary file over the target destination using `std::fs::rename` (or `MoveFileExW` with `MOVEFILE_REPLACE_EXISTING` on Windows).
  - Update `file_versions` fingerprint cache only after the atomic rename successfully completes.

---

### 8. Zero-Copy Document Metrics Memoization
- **Problem**: In `app/src/app/shell.rs` (line 335) and `app/src/vim/backend.rs`, document metrics (total words, reading time, total characters, line counts) are recalculated across the entire text on every frame or on every cursor movement, calling `lines.join("\n")` and traversing hundreds of thousands of characters repeatedly.
- **Root Cause**: Metrics computation is bound to the render pass rather than being invalidation-driven.
- **Robust Suggestion**:
  - Store memoized `DocumentMetrics { words: usize, chars: usize, lines: usize, reading_mins: usize }` on `VimBackend`.
  - Invalidate and recompute metrics only when `lines_revision` increments (i.e. when buffer lines are inserted, deleted, or replaced).
  - Calculate words and characters via row iterator without allocating an intermediate joined `String`.

---

### 9. Native Gutter Diagnostic Signs & Inline Virtual Text
- **Problem**: LSP diagnostics (warnings, errors, hints) are currently communicated via statusline summaries (`mindforge_lsp_status`) or floating modals, but lines with compiler errors or lint warnings have no visual indicators in the editor gutter or at the end of the problematic line.
- **Root Cause**: Gutter painting in `app/src/vim/render.rs` only handles line numbers and does not query or render Neovim diagnostic namespaces.
- **Robust Suggestion**:
  - Subscribe to Neovim diagnostic events via MessagePack-RPC:
    `vim.diagnostic.get(0)` returning `{ lnum, col, severity, message }`.
  - In `render.rs` gutter pass:
    - Reserve a 16px sign column beside line numbers for Error (`●` in `theme.accent`), Warning (`▲` in amber), and Info (`ℹ`).
    - Render subtle virtual text at the end of the line on hover or when the caret is on that row, styled with dimmed severity colors (`Color32::from_rgba_unmultiplied(..., 140)`).

---

### 10. Quickfix Navigation Command Suite & Auto-Dismiss HUD
- **Problem**: When running `:references`, `:codeaction`, or pressing `gr`, Neovim opens its built-in quickfix window (`buftype=qf`). Users must manually close it, and there are no quick keybindings or HUD controls to cycle between occurrences.
- **Root Cause**: Quickfix management is treated as a plain Neovim buffer without native MindForge GUI surface bindings.
- **Robust Suggestion**:
  - Register canonical quickfix traversal commands in `app/src/command/dispatch/`:
    `:cnext` / `:cn`, `:cprev` / `:cp`, `:clist`, and `:cclose` / `:ccl`.
  - Bind global quickfix navigation shortcuts: `]q` (next reference/error) and `[q` (previous reference/error).
  - Surface a discreet, floating pill badge in the editor when the quickfix window is populated (e.g. `1 of 14 references`), with clickable `‹` and `›` arrows and an `✕` dismiss button.

---

### 11. Conflict-Free Autocomplete Priority (Wikilinks vs Neovim PUM)
- **Problem**: In `app/src/app/shell.rs` (lines 810–885), the wikilink autocomplete popup (`[[...]]`) and Neovim's native popupmenu (`ext_popupmenu` / `pumvisible`) can compete for arrow key navigation, `Enter`, and `Escape` when typing inside markdown wikilinks or tags.
- **Root Cause**: Keyboard routing does not maintain a strict priority arbiter between the egui wikilink state machine and Neovim's completion engine.
- **Robust Suggestion**:
  - Enforce clear priority:
    - When `app.services.wikilink_autocomplete.is_active` is true, suppress Neovim completion (`pcall(vim.fn.pum_setvisible, 0)` or ignore PUM events) and route keyboard navigation exclusively to wikilink items.
    - Conversely, when Neovim PUM is active for code files (Rust, Python, Lua), suppress wikilink trigger checks on single bracket input.
    - When an autocomplete item is accepted, insert the text via a single `nvim_input` call rather than a separate file edit.

---

### 12. Cross-Platform Language Server Binary Resolver & Sandbox Isolation
- **Problem**: In `app/assets/nvim/mindforge/lsp/install.lua` and `registry.lua`, language server installation uses system package managers (`npm`, `cargo`, `pip`) without verifying that node/npm or python are installed on the user's system PATH, failing with cryptic spawn errors when dependencies are missing.
- **Root Cause**: LSP installation scripts lack preflight environment verification and isolated directory containment.
- **Robust Suggestion**:
  - Implement a preflight health check in `install.lua` that verifies the runtime binary (`node -v`, `npm -v`, `cargo --version`, `python --version`) before initiating an install, displaying clear error prompts if a prerequisite runtime is missing.
  - Store all installed LSP binaries under `mindforge_home/servers/<server_name>` with platform-specific executable extensions (`.cmd`/`.exe` on Windows, standard executable on Unix).
  - Add PATH prepend logic so Neovim spawns language servers directly from the local MindForge server directory without global system pollution.

---

### 13. Context-Aware Key Interception Hierarchy
- **Problem**: In `app/src/input/mod.rs` and `app/src/input/global/`, global hotkeys (e.g. `Ctrl+b` for sidebar, `Ctrl+h` for navigation, `Ctrl+p` for quick search) unconditionally intercept keystrokes before they reach the Neovim editor backend. Power users with Neovim keymaps (like `<C-b>` for page up or `<C-p>` for keyword completion) cannot use standard Vim mappings.
- **Root Cause**: Global egui shortcuts have higher routing precedence than Neovim editor surface focus.
- **Robust Suggestion**:
  - Implement a structured input routing tree:
    1. **Modal Overlays**: Search modal, rename modal, delete confirm modal (highest priority).
    2. **Command Bar**: Active `:` or `/` input bar.
    3. **Vim Editor Focus**: If Neovim editor is focused:
       - In **Insert / Visual mode**: Forward all non-system shortcuts directly to Neovim.
       - In **Normal mode**: Allow user-configured pass-through for Vim navigation keys while reserving primary app shortcuts (`Ctrl+Shift+P`, `Ctrl+S`).
  - Provide a setting toggle (`settings.json`: `"vim_owns_all_ctrl_keys": false/true`) for full terminal-grade key loyalty.

---

### 14. Unicode-Safe Keystroke Translation & Bracket Escaping
- **Problem**: In `app/src/vim/client.rs` (lines 325–345) and `app/src/vim/input.rs`, text inputs with special characters or multi-byte UTF-8 sequences (such as non-English alphabets, emojis, accented letters, or literal `<` characters) can be misinterpreted by Neovim's `nvim_input` parser, leading to missing characters or unintended keymap trigger sequences.
- **Root Cause**: `nvim_input` parses angle-bracket notation (`<Esc>`, `<CR>`, `<Tab>`, `<C-x>`) by default; unescaped `<` characters cause parse failures.
- **Robust Suggestion**:
  - For text input events in Insert mode: always escape `<` to `<LT>`.
  - For pasted content: use `nvim_paste` with `phase = -1` (or `0`) rather than splitting into individual `nvim_input` keystrokes.
  - For direct keyboard character typing: send characters directly via `nvim_put` or batch text events to eliminate bracket parsing ambiguities and optimize IME (Input Method Editor) support for CJK/accented input.

---

### 15. Unified Keymap Overrides System with Conflict Detection
- **Problem**: Keybindings are currently defined in multiple separate locations: `app/src/setting/shortcuts.rs`, `app/src/input/global/*.rs`, `app/src/vim/input.rs`, and Lua keymaps in `bootstrap.rs` and `completion.lua`. Modifying a shortcut in settings does not automatically synchronize or detect conflicts with Neovim's internal keymaps.
- **Root Cause**: Decentralized keymapping logic spread across Rust event handlers and Lua configurations.
- **Robust Suggestion**:
  - Create a central `KeymapRegistry` in `app/src/setting/` that reads from `shortcuts.json`.
  - Provide bidirectional conflict detection: when a user assigns a shortcut in the MindForge UI that overlaps with a critical Neovim key (e.g. `<C-r>`, `<C-o>`, `<C-i>`), display an inline warning indicator in the Settings panel.
  - Generate a synchronized `mindforge_keymaps.lua` file during startup that declares user-configured leader keys and global shortcuts consistently inside Neovim.

---

### 16. Decompose the `shell.rs` God Function into Focused Render Passes
- **Problem**: `app/src/app/shell.rs` contains a monolithic `draw` method exceeding 700 lines that mixes titlebar assembly, split layout computations, tab switching, editor routing, LunaLine statusline rendering, terminal docking, and modal overlay drawing in a single function.
- **Root Cause**: Centralized application loop handling all major UI domains without separation of concerns.
- **Robust Suggestion**:
  - Refactor `shell.rs` into distinct pipeline passes:
    1. `shell::layout::compute_workspace_layout(&mut self, ui)` $\rightarrow$ Returns layout rects (`titlebar_rect`, `sidebar_rect`, `editor_rect`, `dock_rect`).
    2. `shell::views::render_sidebar(&mut self, ui, sidebar_rect)`
    3. `shell::views::render_center_content(&mut self, ui, editor_rect)` (routes to Neovim editor, markdown preview, or doc viewer).
    4. `shell::views::render_bottom_dock(&mut self, ui, dock_rect)` (LunaLine, terminal drawer).
    5. `shell::overlays::render_modal_layers(&mut self, ui)` (search, command suggestions, language selector).
  - This architecture eliminates repetitive coordinate calculations and simplifies debugging.

---

### 17. Subpixel Glyph Grid Alignment & Zoom-Resilient Font Metrics
- **Problem**: In `app/src/vim/render.rs` (lines 150–350), when zooming the editor or changing font size, row heights and column widths calculated as floating point values (`f32`) can produce subpixel rounding artifacts (e.g. 1px line seams between cells, uneven cursor widths, or slight vertical jitter during scrolling).
- **Root Cause**: Character cell geometry does not round to integer device pixel boundaries at varying DPI scale factors.
- **Robust Suggestion**:
  - Calculate row height and character advance as pixel-grid-aligned values:
    `row_height = (font_size * line_height_multiplier * ppi).round() / ppi`.
  - Align the editor canvas starting origin (`editor_rect.min`) to exact device pixel boundaries using `ui.ctx().pixels_per_point()`.
  - Snap cursor rectangles and highlight background rectangles to integer pixel boundaries (`rect.round_ui()`), eliminating visual blurring and selection gaps.

---

### 18. Layer Ordering & Focus Trapping for Floating Popups
- **Problem**: When popups such as the Language Detection dropdown, Command Suggestions palette, or Search Modal open over the editor, mouse clicks or wheel scrolls can bleed through to the underlying Neovim editor grid, triggering unwanted caret moves or viewport scrolls.
- **Root Cause**: Floating egui areas did not consistently allocate blocking interaction barriers across their full boundary rects.
- **Robust Suggestion**:
  - Wrap all floating overlay components (`language_detection.rs`, `command_suggestion.rs`, `search.rs`) in modal interaction barriers:
    - Allocate an overlay area with `egui::Order::Foreground`.
    - Consume primary and secondary click events that occur outside the popup to dismiss the popup cleanly without passing the click to the background editor canvas.
    - Intercept and consume scroll wheel deltas within popup boundaries so background Neovim buffers never scroll while navigating popup lists.

---

### 19. OS-Native Event-Driven File Watching with Symlink Loop Guards
- **Problem**: In `app/src/workspace.rs` and `app/src/workspace_import/scanner.rs`, workspace file discovery and external change detection rely on periodic metadata polling, which is CPU-intensive on large projects (thousands of files) and vulnerable to infinite loops if a circular symlink directory exists.
- **Root Cause**: Polling-based directory traversal without cycle detection or OS-native filesystem notification hooks.
- **Robust Suggestion**:
  - Integrate the `notify` crate to receive OS-native event streams (`ReadDirectoryChangesW` on Windows, `inotify` on Linux, `FSEvents` on macOS).
  - Debounce file system events with a 100ms trailing window to coalesce multi-step writes (such as git branch checkouts or compilers writing output files).
  - In recursive directory walkers: maintain a visited `HashSet<PathBuf>` tracking canonical paths (`std::fs::canonicalize`) to immediately abort symlink loops and protect against stack overflow.

---

### 20. Multi-Threaded Background Workspace Search with Memoization
- **Problem**: In `app/src/services/fuzzy.rs` and `app/src/modals/search.rs`, workspace-wide full-text and filename search scans every file synchronously on the background worker or UI thread upon every keystroke, causing latency when searching through large codebases.
- **Root Cause**: Linear unindexed scanning without query prefix memoization or worker thread pools.
- **Robust Suggestion**:
  - Utilize a worker thread pool (e.g. `rayon` or an internal background channel) with cancelable query tokens: typing a new character cancels the in-flight query immediately.
  - Implement query result memoization: if query `"mod"` is typed after `"mo"`, filter the existing matched candidate subset rather than rescanning the entire filesystem from scratch.
  - Pre-index file paths on project load into an in-memory trigram index or sorted vector, allowing sub-5ms fuzzy filename lookups across 50,000+ files.
