# MindForge — Performance, Architecture & Explorer Polish

This is now a **performance + architecture + filesystem/editor polish pass**.

MindForge was designed from the beginning to be a **high-speed editor/IDE**.

The goal is not merely to make it functional.

The goal is:

> **Fast, lightweight, responsive, maintainable, visually coherent, and capable of handling large projects and large files without becoming slow or heavy.**

Do not optimize blindly.

Inspect the actual codebase, profile the expensive paths, identify the causes, fix them, and verify the improvements in the running application.

---

# 1. READ THE PERFORMANCE DOCUMENT FIRST

There is an existing performance document:

```text
performance.md
```

Inspect it first.

Use it as an important source for the existing performance architecture, measurements, known problems, and intended direction.

Also inspect the relevant performance-related files, services, editor code, filesystem code, Explorer code, Neovim/LSP code, indexing/search code, and rendering code.

Do not create a second competing performance system if one already exists.

---

# 2. MIND FORGE PERFORMANCE IS A CORE REQUIREMENT

The application was intentionally designed to be high-performance.

Treat performance as a first-class architectural requirement.

The following must remain responsive even under heavy workloads:

- typing;
- cursor movement;
- scrolling;
- opening files;
- switching tabs;
- switching between Hybrid and Neovim;
- Explorer interaction;
- folder expansion;
- file/folder movement;
- fuzzy search;
- LSP completion;
- diagnostics;
- Markdown editing;
- workspace navigation.

Never solve performance problems by hiding functionality.

Find the actual bottleneck.

---

# 3. FIND WHAT MAKES THE APPLICATION SLOW OR HEAVY

Perform a codebase-wide performance audit.

Look for:

- unnecessary allocations;
- excessive `.clone()`;
- repeated `String` creation;
- repeated filesystem scans;
- repeated directory traversal;
- rebuilding the entire Explorer tree;
- rebuilding the entire editor;
- whole-document copies;
- full-document parsing on every keystroke;
- full syntax re-highlighting;
- repeated Markdown parsing;
- repeated Wikilink indexing;
- repeated SQLite queries;
- synchronous filesystem I/O;
- blocking Neovim RPC;
- blocking LSP requests;
- unbounded queues;
- redundant redraws;
- unnecessary egui repaint requests;
- duplicate state;
- stale caches;
- unnecessary serialization/deserialization;
- work repeated every frame;
- large computations running on the UI thread.

Fix the root causes.

---

# 4. FIND "GOD FILES" / "GOD FUNCTIONS"

Search the project for extremely large or overly-responsible files/functions.

If a file is doing too many unrelated jobs, split it.

For example, avoid a structure like:

```text
backend.rs
    ├── Neovim communication
    ├── rendering
    ├── input
    ├── cursor calculation
    ├── completion
    ├── diagnostics
    ├── filesystem
    ├── tabs
    └── document management
```

Instead, separate responsibilities into clearly named modules.

Possible structure:

```text
vim/
├── backend.rs
├── client.rs
├── input.rs
├── renderer.rs
├── grid.rs
├── cursor.rs
├── completion.rs
├── diagnostics.rs
├── document.rs
└── lifecycle.rs
```

Use the existing architecture where appropriate rather than blindly creating dozens of files.

### Every extracted module must have a clear responsibility.

Do not split files simply to make them shorter.

---

# 5. COMMENTS AND MODULE DOCUMENTATION

When splitting large/god files, add useful comments.

At the top of important modules explain:

```text
What does this module do?
Why does it exist?
What owns the state?
What must NOT be done here?
Which subsystem communicates with it?
```

For example:

```rust
//! Neovim grid renderer.
//!
//! Converts Neovim UI/grid state into MindForge's egui editor surface.
//! This module owns rendering only.
//!
//! It must not:
//! - mutate the authoritative Neovim buffer;
//! - perform blocking RPC;
//! - implement Vim semantics.
```

Comments should explain **architecture and reasoning**, not obvious syntax.

---

# 6. SIDEBAR / EXPLORER VISUAL PROBLEM

The attached screenshot shows a clear layout problem.

The Explorer currently looks roughly like:

```text
┌─────────────────────────────┐
│ Stats                       │
│ WORDS             buttons   │
│                             │
│       hello.py              │
│       README.md             │
│       typing.json           │
└─────────────────────────────┘
```

The files are incorrectly centered.

This is NOT acceptable for a filesystem Explorer.

The file/folder tree must be **left aligned**.

Use a consistent Explorer content origin:

```text
┌─────────────────────────────┐
│ STATS                       │
│                             │
│ WORKSPACE            ...    │
│                             │
│ ▾ src                       │
│   ▾ components              │
│     App.tsx                 │
│   main.py                   │
│   utils.py                  │
│                             │
│ README.md                   │
│ package.json                │
└─────────────────────────────┘
```

Every filename should begin from a predictable left edge.

Folders should indent relative to their parent.

Files should NEVER be horizontally centered in the available Explorer width.

---

# 7. CLEAR SEPARATION BETWEEN STATS AND EXPLORER

The screenshot also shows the Stats area and its buttons visually colliding with the file area.

Create a clear vertical separation.

Conceptually:

```text
STATS
────────────────────────

WORKSPACE
MyProject                 ...

▾ src
  main.py
  utils.py

README.md
package.json
```

There should be an obvious visual gap between:

```text
Stats
```

and:

```text
Workspace / Files
```

The Stats controls must not look like they belong to the file rows.

Do not simply add arbitrary empty pixels.

Use proper layout sections, spacing tokens, and container boundaries.

---

# 8. SIDEBAR BUTTONS

The current buttons near Stats look visually attached and cluttered.

Polish them.

They should:

- have consistent size;
- align properly;
- have appropriate hit areas;
- use MindForge's iconography;
- have subtle hover states;
- avoid oversized backgrounds;
- not compete visually with the file tree.

Do not make every button look like a large rectangular button.

Prefer compact IDE-style controls.

---

# 9. FILE/FOLDER TYPOGRAPHY

The Explorer filenames must be readable.

Do not make the filenames tiny.

Use a comfortable font size and row height.

Maintain:

```text
icon → filename
```

alignment.

Long names should truncate gracefully:

```text
Building Your Word Collec...
```

but the actual filesystem filename must remain unchanged.

Show the full name through a tooltip when useful.

---

# 10. FILE TREE PERFORMANCE

The Explorer must remain fast with large workspaces.

Do NOT render thousands of nested files unnecessarily.

Use lazy folder expansion.

For example:

```text
workspace/
├── src/
├── node_modules/
├── target/
├── .git/
└── docs/
```

Opening the workspace should not require constructing the complete visual tree of every descendant.

Load/expand what is needed.

Do not scan the entire workspace repeatedly.

---

# 11. FILESYSTEM IS THE SOURCE OF TRUTH

Maintain the filesystem-first architecture.

The actual filesystem owns:

- files;
- folders;
- contents;
- paths;
- hierarchy;
- existence.

SQLite must NOT become the canonical storage for source files or Markdown documents.

SQLite may remain as a secondary:

- cache;
- index;
- search index;
- knowledge graph store;
- metadata store;

only where it genuinely provides value.

If the SQLite cache is deleted, MindForge should be capable of rebuilding it from the filesystem.

---

# 12. LARGE FILE PERFORMANCE

Test:

```text
1 MB
10 MB
50 MB
100 MB
250 MB
500 MB
```

especially:

```text
large.json
```

The application must remain usable.

Do not duplicate huge documents in memory.

Avoid:

```text
filesystem → String
         → clone
         → Neovim
         → clone
         → syntax highlighting
         → clone
         → indexer
```

Audit every large-document ownership path.

---

# 13. EDITOR VIRTUALIZATION

Do not create/render one UI element per line for enormous files.

For a million-line file:

```text
1,000,000 lines
```

render only the visible region plus sensible overscan.

The editor must remain responsive while scrolling.

---

# 14. SYNTAX HIGHLIGHTING

Syntax highlighting must be incremental.

Do not re-highlight an entire 100 MB file because one character changed.

Prefer:

```text
changed region
     ↓
invalidate affected tokens
     ↓
incrementally update
```

Test:

- Markdown;
- Python;
- HTML;
- CSS;
- JavaScript;
- TypeScript;
- TSX;
- Rust;
- JSON.

---

# 15. LSP PERFORMANCE

LSP must never block the UI.

Large files and diagnostic storms must not freeze MindForge.

Audit:

- RPC;
- redraw events;
- diagnostics;
- completion;
- hover;
- server startup;
- server shutdown;
- event queues.

Coalesce redundant updates where safe.

Never drop important semantic state simply to improve a benchmark.

---

# 16. NEOVIM PERFORMANCE

Audit the complete:

```text
egui
 ↓
MindForge input
 ↓
Neovim RPC
 ↓
Neovim
 ↓
redraw events
 ↓
MindForge grid
 ↓
egui
```

Look for:

- blocking RPC;
- redundant serialization;
- duplicate events;
- unnecessary full-buffer synchronization;
- redraw storms;
- event queue growth;
- repeated allocations.

Typing one character should not cause the entire document to be transferred and rebuilt.

---

# 17. TAB PERFORMANCE

Switching:

```text
A → B → C → A
```

must be fast.

Do not:

- reload unnecessarily;
- duplicate document content;
- restart LSP;
- rebuild syntax highlighting;
- rebuild the Explorer;
- rescan the workspace.

Preserve:

- cursor;
- scroll;
- unsaved changes;
- appropriate editor state.

---

# 18. MARKDOWN PERFORMANCE

Markdown is a core MindForge feature.

Do not sacrifice it.

Large Markdown documents must remain responsive.

Wikilinks, backlinks, graph indexing and preview generation should not run as expensive synchronous work on every keystroke.

Use incremental updates/debouncing/background work where appropriate.

---

# 19. FUZZY SEARCH

The fuzzy finder must remain fast with:

```text
10,000 files
50,000 files
100,000 files
```

Do not scan the entire filesystem for every keystroke.

Use a reusable index.

Cancel obsolete queries.

Keep search off the UI thread when necessary.

---

# 20. FILESYSTEM WATCHER

Do not respond to every filesystem event by rescanning the entire workspace.

Instead:

```text
file changed
 ↓
identify affected path
 ↓
update affected Explorer/document/index state
```

Coalesce bursts from:

- git;
- builds;
- package managers;
- bulk file operations.

---

# 21. MEMORY AUDIT

Measure memory with:

- many tabs;
- large files;
- large workspace;
- LSP active;
- fuzzy index;
- Markdown indexing;
- Explorer expanded.

Look for:

- duplicated documents;
- leaked buffers;
- stale completion data;
- stale diagnostics;
- stale tab state;
- unbounded event queues;
- unnecessary cached file contents.

Closing a file should release resources that are no longer needed.

---

# 22. PERFORMANCE HUD

Use the existing performance instrumentation if present.

Measure:

```text
FPS
frame time
editor render time
input processing
Neovim event processing
RPC queue depth
LSP processing
filesystem operations
fuzzy-search latency
workspace indexing
memory
```

Do not create duplicate monitoring systems.

---

# 23. DO NOT OPTIMIZE BY GUESSING

For every major change:

```text
measure
 ↓
identify bottleneck
 ↓
fix
 ↓
measure again
```

Keep changes that produce meaningful improvements.

Avoid speculative abstractions.

Avoid unnecessary complexity.

---

# 24. DO NOT BREAK FUNCTIONALITY

During this pass, preserve:

- Markdown;
- Wikilinks;
- backlinks;
- graph;
- filesystem Explorer;
- folder creation;
- nested folders;
- drag/drop;
- bulk moves;
- tabs;
- Neovim;
- Hybrid;
- LSP;
- autocomplete;
- diagnostics;
- fuzzy search;
- terminal;
- status bar.

Performance work must not regress functionality.

---

# 25. FINAL QUALITY REQUIREMENT

When you finish, MindForge should feel like:

> **A fast native editor that happens to have a knowledge system — not a database application trying to become an editor.**

The filesystem should remain simple.

The Explorer should be fast.

The editor should remain responsive.

Large files should not bring down the application.

LSP should work in the background.

SQLite should not be involved in unnecessary document operations.

And the codebase itself should remain understandable.

If you find a large "God file" or "God function", split it into meaningful modules with clear names and architectural comments rather than leaving a huge file simply because it currently works.

**Do not stop at compilation or unit tests. Run the actual application and exercise the heavy workloads.**

The final acceptance criterion is simple:

> **MindForge must remain fast when the workload becomes large.**
