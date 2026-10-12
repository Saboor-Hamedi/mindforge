# MindForge Architectural & Performance Optimization Suggestions

This document presents a deep, production-grade architectural and performance audit for MindForge. It analyzes core runtime flows, memory models, threading and IPC barriers, and latency characteristics, providing concrete, zero-regression suggestions for the Language Server Protocol (LSP) pipeline, embedded Neovim engine, filesystem sync, and UI rendering layer.

---

## 1. LSP & Language Intelligence Pipeline

### 1.1 Multi-Buffer Architecture vs. Buffer 0 Multiplexing
- **Current Flow**:
  MindForge routes all open tabs through a single Neovim buffer (buffer `0`). Switching tabs renames buffer 0, clears content, and invokes `reset_buffer()` in Lua, detaching running LSP clients.
- **Root Cause & Latency Profile**:
  Every tab switch triggers `LspDetach`, followed by filetype detection and `LspAttach`. Heavy language servers (`rust-analyzer`, `tsserver`, `clangd`) treat this as a document lifecycle reset, discarding local syntax trees and forcing re-computation of semantic tokens, diagnostics, and import graphs.
- **Deep Suggestion**:
  - Implement a **1-to-1 buffer mapping**: allocate a distinct Neovim buffer (`nvim_create_buf(true, false)`) for each open editor tab.
  - Store `nvim_buf_id` inside MindForge's `OpenNote` tab struct.
  - On tab switch, execute `nvim_set_current_buf(tab.nvim_buf_id)` ($O(1)$ pointer swap inside Neovim).
  - Let language servers maintain persistent `textDocument/didOpen` connections per buffer, eliminating tab-switch re-indexing latency and preventing spurious `textDocument/didClose` cycles.

### 1.2 Incremental Document Synchronization (`textDocument/didChange`)
- **Current Flow**:
  Document text changes synchronize via full buffer line dumps (`client.set_buffer_text_async`), and updates sent back to egui coalesce whole text blocks using `text.lines().collect()`.
- **Root Cause & Latency Profile**:
  For large source files (5,000+ lines), full buffer replacement serializes entire string payloads across stdio MessagePack-RPC, causing heap churn and forcing language servers into full-file AST re-parses.
- **Deep Suggestion**:
  - Implement full support for LSP `TextDocumentSyncKind::Incremental`.
  - Listen to incremental `nvim_buf_lines_event(firstline, lastline, new_lines)` notifications.
  - Translate line delta events into precise LSP range-formatted text edits (`Range { start, end }, new_text`).
  - Send compact delta packets to external language servers and indexing routines, slashing IPC payload size by 99% during active typing.

### 1.3 Asynchronous Non-Blocking Completion & Adaptive Debouncing
- **Current Flow**:
  `InsertCharPre` sets a static 60ms timer to request completions via `vim.lsp.completion.get`.
- **Root Cause & Latency Profile**:
  Fixed debounce timers fail during fast burst typing (e.g., 80+ WPM) or when language servers take >100ms to resolve semantic candidates. Outstanding RPC requests stack up in the pipe, causing delayed popup dismissals and keystroke stutter.
- **Deep Suggestion**:
  - Implement **adaptive predictive debouncing**: 20ms for local identifier/snippet completion, scaled up to 80–120ms for remote semantic servers.
  - Cancel prior in-flight `textDocument/completion` requests by sending `$/cancelRequest` with the corresponding request ID whenever newer keystrokes arrive before response completion.
  - Maintain a local prefix cache in Rust: if the user types `foo` and receives 50 items, filtering on typing `b` to form `foob` should happen instantly in Rust memory without re-querying the language server.

### 1.4 Native Floating Tooltips for Hover Documentation (`textDocument/hover`)
- **Current Flow**:
  `K` invokes `vim.lsp.buf.hover`, which renders inside Neovim's character grid. In the completion popup, docstrings are parsed via `clean_documentation` and laid out in `render.rs`.
- **Root Cause & Latency Profile**:
  Neovim character grid floats are constrained by cell coordinates and monospaced font geometry. Rich markdown documentation (fenced code blocks, type signature tables, clickable links) gets stripped or awkwardly wrapped.
- **Deep Suggestion**:
  - Intercept `textDocument/hover` via a dedicated MindForge RPC callback (`mindforge_hover`).
  - Render documentation in an egui floating popover using proportional typography, syntax-highlighted code fences, and smooth screen-edge clamping.
  - Cache rendered doc galleys per symbol to prevent repetitive re-parsing during mouse hovering.

### 1.5 Diagnostic Virtualization & Sub-Pixel Squiggly Waves
- **Current Flow**:
  Diagnostics currently disable virtual text and signs to avoid grid row line wrapping in Neovim, relying on plain cell underlines.
- **Root Cause & Latency Profile**:
  Neovim cell underlines cannot render anti-aliased squiggly error waves or severity-colored gutter indicators with fine sub-pixel precision.
- **Deep Suggestion**:
  - Intercept `textDocument/publishDiagnostics` in the Rust runtime via an event forwarder.
  - Map diagnostic ranges (`Range { start, end }`, `Severity`) to document byte offsets.
  - In egui's paint pass, compute exact bounding boxes and paint high-DPI sinusoidal wave curves (`Color32::from_rgb(239, 68, 68)`) and gutter dot badges directly onto the viewport canvas, entirely independent of Neovim character grid restrictions.

### 1.6 Bidirectional Go-To-Definition & Workspace Sync
- **Current Flow**:
  `gd` calls `vim.lsp.buf.definition()`, which executes inside Neovim. If the target file is not open in MindForge's tab bar, the tab system and Neovim buffer state can desynchronize.
- **Deep Suggestion**:
  - Hook Neovim's `BufEnter` / `BufAdd` autocmds with an RPC event: `mindforge_file_opened(path)`.
  - When Neovim navigates to a new file via `gd` or `:edit`, MindForge automatically creates or switches to the corresponding tab, reveals the target in the sidebar tree, and scrolls into view.

### 1.7 Dedicated LSP Process Supervisor & Sandboxed Package Isolation
- **Current Flow**:
  LSP binaries installed via `:LspInstall` run through `install.lua`, executing `npm` with Node directly or resolving through system `PATH`.
- **Root Cause & Bottleneck**:
  If a language server crashes repeatedly (OOM on large repos) or hangs at 100% CPU, it can degrade system performance or lock Neovim's event loop.
- **Deep Suggestion**:
  - Implement a **supervisor watchdog** for spawned language servers:
    - Monitor child PID memory and CPU usage.
    - If a server crashes 3 times in 30 seconds, mark it as disabled with a clear status bar notification rather than looping continuously.
    - Provide an explicit memory budget per language server (e.g. 2 GB for `rust-analyzer` / `tsserver`).

---

## 2. Embedded Neovim Runtime & IPC Performance

### 2.1 Multi-Threaded MessagePack-RPC Prioritization
- **Current Flow**:
  `NeovimClient` processes all incoming RPC packets sequentially over a single reader thread. Gated repaint wakeups run every 16ms.
- **Root Cause & Latency Profile**:
  Bulk redraw batches (`grid_line`, `hl_attr_define`) share the same channel as high-priority user input replies. A massive syntax-highlighting burst from opening a 10,000-line file can delay user keystroke acknowledgments.
- **Deep Suggestion**:
  - Establish a **dual-priority channel architecture**:
    - **High Priority**: Keyboard inputs, mouse events, mode queries, and cancellation signals.
    - **Bulk Stream**: Grid redraw updates, line delta events, and diagnostic payloads.
  - Redraw frame collapsing: if the reader receives multiple `redraw` batches before the UI renders the next frame, merge intermediate grid states in memory and emit a single collapsed repaint tick.

### 2.2 Shared Memory / Zero-Copy Stdio Pipe Buffering
- **Current Flow**:
  MessagePack encoding and decoding allocate intermediate `rmpv::Value` tree nodes for every cell in every redraw row (`GridCell { text: String, highlight: u64 }`).
- **Root Cause & Latency Profile**:
  At 60 FPS on a 160x50 character terminal grid, allocating thousands of small strings per frame causes severe L1/L2 cache pollution and allocator fragmentation.
- **Deep Suggestion**:
  - Implement zero-allocation streaming decoding using `rmp-serde` directly into fixed-size grid row arrays (`[Cell; MAX_COLS]`).
  - Store character strings as inline small-string buffers (`CompactStr` / `ArrayString<4>`) to eliminate heap allocation for individual characters and glyphs.

### 2.3 Resilient Process Supervision & Zero-Data-Loss Auto-Recovery
- **Current Flow**:
  If the child Neovim process terminates unexpectedly, `poll_exit` logs an error and the editor becomes unresponsive.
- **Deep Suggestion**:
  - Implement an **auto-recovery supervisor**:
    - Keep in-memory snapshots of all dirty editor buffers in Rust.
    - If Neovim terminates or becomes unresponsive (detected via a 2-second RPC heartbeat ping), transparently spawn a fresh `nvim --embed` instance.
    - Re-populate buffers, restore cursor coordinates, re-attach LSP servers, and continue without data loss.

### 2.4 Decoupled Cursor Interpolation & Render Pass Optimization
- **Current Flow**:
  When cursor blinking or cursor gliding animations run at 60–120 FPS, `render_in_rect_impl` checks `row_revision` across all grid rows.
- **Deep Suggestion**:
  - Isolate the cursor animation layer: cache the rendered text grid as a retained list of egui `Shape` primitives or a cached texture.
  - During animation ticks where text has not changed, paint the cached text layer directly and draw only the moving caret quad on top, saving up to 80% of per-frame render CPU time.

---

## 3. Editor Core Data Structures & Memory Hierarchy

### 3.1 B-Tree Rope Buffer vs. Flat `Vec<char>`
- **Current Flow**:
  The native fallback editor (`Editor` in `app/src/editor/mod.rs`) stores document content as `pub buf: Vec<char>`.
- **Root Cause & Latency Profile**:
  Inserting a character at the beginning of a 2 MB file requires moving 2,000,000 characters in memory ($O(N)$ copies).
- **Deep Suggestion**:
  - Transition to a **B-Tree Rope** data structure (e.g., `crop` or `ropey`).
  - Guarantees $O(\log N)$ insertions, deletions, line-to-char queries, and byte-offset conversions.
  - Allows chunked UTF-8 string slices without allocating continuous linear arrays.

### 3.2 Transactional Delta-Compressed Undo/Redo Engine
- **Current Flow**:
  `save_undo_snapshot` in `app/src/editor/undo.rs` clones the entire `buf: Vec<char>` on every snapshot:
  ```rust
  let snap = EditorSnapshot { buf: self.buf.clone(), cur: self.cur };
  ```
- **Root Cause & Latency Profile**:
  Maintaining 300 full snapshots of a 500 KB document consumes 150 MB of memory for a single file tab.
- **Deep Suggestion**:
  - Replace full snapshots with **reversible operational deltas**:
    ```rust
    pub enum EditDelta {
        Insert { offset: usize, text: CompactString },
        Delete { offset: usize, text: CompactString },
    }
    ```
  - Group continuous typing bursts into single undo transactions.
  - Reduces undo memory footprint by over 98% while supporting unlimited undo depth.

### 3.3 Large-File Virtualization & Piece Table Buffer Model
- **Current Flow**:
  Opening minified JavaScript or multi-gigabyte log files attempts to read the entire file into memory and pass it to Neovim / `Editor`.
- **Root Cause & Latency Profile**:
  Files larger than 50 MB cause severe allocation lag and can trigger out-of-memory errors on constrained hardware.
- **Deep Suggestion**:
  - Implement a **read-only large file viewer mode** for files exceeding 10 MB:
    - Use memory-mapped I/O (`memmap2`) to read file slices on demand without loading the full file into the heap.
    - Virtualize line breaks by indexing byte offsets of newline characters in a background thread.

---

## 4. Filesystem, Workspace & Explorer Scalability

### 4.1 Asynchronous OS-Native File Event Streaming (`notify`)
- **Current Flow**:
  Workspace changes are polled periodically and re-scanned via filesystem walks.
- **Root Cause & Latency Profile**:
  Scanning directory trees with 50,000+ files on startup or disk wake-up causes high disk I/O and freezes Explorer interactions.
- **Deep Suggestion**:
  - Integrate OS-native file monitoring (`notify` crate using Windows `ReadDirectoryChangesW`, macOS `FSEvents`, Linux `inotify`) on a background worker thread.
  - Buffer events with a 150ms debounce window to collapse atomic toolchain writes (e.g., `git checkout` or compiler build outputs).
  - Update only the affected subtrees in `workspace.entries` instead of regenerating the entire tree structure.

### 4.2 Virtualized Row Rendering for the File Explorer
- **Current Flow**:
  `render_sidebar_body` loops through all workspace entries sequentially inside an `egui::ScrollArea`.
- **Deep Suggestion**:
  - Pre-flatten visible expanded tree nodes into an indexed array `Vec<VisibleNode>`.
  - From `scroll_offset` and viewport height, compute visible row range:
    $$\text{start} = \lfloor \text{scroll\_y} / \text{row\_h} \rfloor, \quad \text{end} = \lceil (\text{scroll\_y} + \text{height}) / \text{row\_h} \rceil$$
  - Render strictly the 30–50 rows visible in the viewport, achieving stable 120 FPS performance even with 100,000 files in the workspace.

### 4.3 Background Worker Pool for File I/O & Atomic Writes
- **Current Flow**:
  File operations (saving, deleting, creating, moving) execute synchronously on the main thread (e.g. `std::fs::write(&target, &content)`).
- **Deep Suggestion**:
  - Dispatch file operations to a background I/O task pool.
  - Use atomic write protocols: write to temporary sibling file `.filename.tmp` and execute an atomic rename (`std::fs::rename`) to guarantee file integrity even during sudden application power loss.

### 4.4 Git Status & VCS Decoration Virtualization
- **Current Flow**:
  Git status is evaluated synchronously during file scanning or disabled to prevent blocking.
- **Deep Suggestion**:
  - Run git status checks in a dedicated background worker (`git status --porcelain=v1 -z`).
  - Store a lock-free lookup map `HashMap<PathBuf, GitStatus>` (`Modified`, `Untracked`, `Ignored`, `Added`).
  - Paint color accents (e.g. amber for modified, emerald for new) directly in the sidebar tree rows without touching disk during UI render loops.

---

## 5. UI Rendering, Modals & Command Palette

### 5.1 Pre-Computed Modal Layouts & Zero-Allocation Dialogs
- **Current Flow**:
  Modal components re-calculate text layout jobs (`LayoutJob`) and bounding dimensions on every animation frame.
- **Deep Suggestion**:
  - Compute text layout and modal geometry once during `just_opened` or when message text changes.
  - Store the pre-compiled `egui::Galley` in `ModalState` and paint it directly on subsequent frames with zero heap allocations.

### 5.2 Asynchronous Trigram Fuzzy Search Index
- **Current Flow**:
  `search_workspace` iterates synchronously over all workspace paths on every keystroke in the Command Palette (`Ctrl+P`).
- **Deep Suggestion**:
  - Build and maintain an in-memory trigram / inverted path index in a background worker.
  - As the user types, stream top-ranked matches via a lock-free channel, ensuring search response times remain under 2ms regardless of workspace size.

### 5.3 Tab Bar Pinning & Virtualized Overflow Handling
- **Current Flow**:
  Tabs are laid out linearly in a single row without horizontal pan clipping.
- **Deep Suggestion**:
  - Support horizontal mouse wheel scrolling across the tab bar.
  - Implement a dedicated tab overflow dropdown (`Ctrl+Tab` quick picker) when open tab count exceeds visible bar width.

---

## 6. Execution Roadmap & Impact Matrix

| Subsystem | Optimization | Complexity | Impact | Latency Gain |
| :--- | :--- | :--- | :--- | :--- |
| **LSP** | Multi-Buffer 1-to-1 Mapping | Medium | Critical | Eliminates 200–800ms tab switch delay |
| **LSP** | Incremental `textDocument/didChange` | High | Critical | Reduces typing payload by 99% |
| **LSP** | Native Floating Popovers & Squigglies | Medium | High | High-DPI UI; no grid distortion |
| **LSP** | LSP Watchdog & Memory Budgeting | Low | High | Prevents frozen editor on server crash |
| **Engine** | Multi-Threaded MessagePack Priority Channel | Medium | High | Zero keystroke drops under heavy redraw |
| **Engine** | Neovim Process Crash Auto-Recovery | Medium | Critical | Zero data loss on engine crash |
| **Editor** | B-Tree Rope Buffer & Delta Undo | High | High | $O(\log N)$ editing; 98% RAM reduction |
| **Editor** | Memory-Mapped Large File View Mode | Medium | High | Instant load of 100MB+ logs/files |
| **Explorer**| Virtualized Tree Row Rendering | Low | High | 120 FPS on 50,000+ file trees |
| **Explorer**| OS File Watcher (`notify`) | Medium | High | Instant external file update detection |
| **VCS** | Background Git Status Badging | Low | Medium | Non-blocking git indicators in tree |
| **Search** | Background Trigram Search Index | Medium | Medium | Sub-2ms Command Palette query latency |
