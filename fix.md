# MindForge — Complete LSP Integration Audit & Hardening

## Mission

Perform a **complete production-grade audit, debugging, and hardening pass of the entire LSP integration in MindForge**.

Do not wait for me to report bugs one at a time.

I have already discovered multiple LSP-related problems involving:

- cursor positioning
- Neovim redraw synchronization
- diagnostics rendering
- autocomplete selection
- Enter handling
- Backspace/caret synchronization
- keyboard event routing
- LSP activation changing editor behavior
- completion popup behavior
- virtual text/extmarks
- asynchronous redraw events

There may be more bugs that I have not discovered yet.

Your job is to **find them proactively and fix them all**.

---

# CORE ARCHITECTURE

The architecture must remain:

```text
                 MINDFORGE
                     │
        ┌────────────┼────────────┐
        │            │            │
      Hybrid      Neovim         LSP
        │            │            │
        │       Vim semantics   Language
        │       + buffer        intelligence
        │       + cursor
        │            │
        └────────────┼────────────┘
                     │
              MindForge UI
```

### Neovim is authoritative for:

- buffer contents
- cursor position
- Vim modes
- motions
- operators
- registers
- undo/redo
- insert/delete
- search
- Vim commands

### LSP is authoritative for:

- completion
- diagnostics
- hover
- definitions
- references
- symbols
- language intelligence

### MindForge is authoritative for:

- application UI
- editor surface
- rendering
- tabs
- workspace
- files
- status bar
- completion presentation
- diagnostics presentation
- knowledge system

Do not create competing sources of truth.

---

# 1. FIRST: AUDIT THE ENTIRE SYSTEM

Before changing code, inspect the complete LSP pipeline.

Trace:

```text
File opened
   ↓
Language detection
   ↓
Workspace/root detection
   ↓
LSP server selection
   ↓
Server startup
   ↓
initialize
   ↓
initialized
   ↓
buffer attach
   ↓
text synchronization
   ↓
Neovim redraw
   ↓
completion
   ↓
diagnostics
   ↓
hover
   ↓
definition
   ↓
shutdown
```

Also inspect every boundary between:

```text
egui
↕
MindForge
↕
Neovim
↕
LSP
```

Do not assume existing code is correct merely because some features work.

---

# 2. FIND ALL INPUT CONFLICTS

Audit the complete keyboard pipeline.

Look for:

- duplicate key handling
- `Event::Key`
- `Event::Text`
- global shortcuts
- editor shortcuts
- completion shortcuts
- Vim mappings
- command-line input
- fuzzy-search input
- terminal input
- sidebar input
- Enter
- Escape
- Tab
- Backspace
- Ctrl+J
- Ctrl+K
- arrows
- printable characters

Establish one clear ownership model.

For example:

```text
Completion active?
    ↓
Completion/Neovim owns relevant keys

Normal Neovim editing?
    ↓
Neovim owns editor keys

MindForge global shortcut?
    ↓
MindForge handles only when the editor does not own it
```

No key should accidentally be processed twice.

---

# 3. CURSOR SYNCHRONIZATION

Audit cursor synchronization completely.

The invariant must always be:

```text
Neovim cursor
      =
MindForge grid cursor
      =
Rendered caret
      =
Actual insertion position
```

Verify:

- row
- column
- screen column
- buffer column
- UTF-8 byte offsets
- Unicode characters
- display-cell widths
- horizontal scrolling
- vertical scrolling
- line wrapping if supported

Never treat a screen column as a UTF-8 byte offset.

Never maintain an independently advancing cursor that competes with Neovim.

---

# 4. NEOVIM REDRAW PROTOCOL

Audit every relevant redraw event.

Especially:

```text
grid_line
grid_cursor_goto
grid_scroll
grid_clear
grid_resize
flush
mode_change
hl_attr_define
default_colors_set
```

If using extended UI features, audit those too.

The redraw event stream must be processed in order.

No cursor event should be silently discarded.

No grid update should accidentally overwrite newer cursor state.

No asynchronous LSP event should corrupt the grid state.

---

# 5. LSP DIAGNOSTICS

Diagnostics must be treated as metadata/overlay information.

They must NEVER become document text.

They must NEVER change:

- cursor position
- insertion position
- line numbering
- editor row
- editor column
- document layout

Implement a clean visual model:

```text
source line
   ↓
diagnostic metadata
   ↓
MindForge overlay
```

Prefer:

- gutter markers
- subtle underlines
- hover information
- status-bar counts
- optional diagnostic panel

Do NOT dump complete diagnostic messages inline across the editor.

---

# 6. COMPLETION

Audit completion from beginning to end.

Verify:

```text
trigger
 ↓
request
 ↓
response
 ↓
popup
 ↓
selection
 ↓
navigation
 ↓
acceptance
 ↓
text edit
 ↓
popup close
```

The selected completion item must have a real authoritative selection state.

Do not visually select an item in egui while Neovim considers another item selected.

Test:

- typing
- Ctrl+Space
- automatic completion
- first-item selection
- arrows
- Ctrl+J
- Ctrl+K
- Enter
- Tab
- Escape
- Backspace
- rapid typing
- documentation
- no results
- large results

---

# 7. ENTER EVENT

When completion is active:

```text
Enter
 ↓
accept completion
 ↓
consume event
```

It must NOT propagate to:

- Wikilink opening
- note navigation
- tabs
- global commands
- other application handlers

When completion is not active, normal Enter behavior must remain unchanged.

---

# 8. BACKSPACE

Test Backspace in all states:

```text
normal editing
completion visible
completion dismissed
after completion accepted
after LSP diagnostic update
Unicode text
empty line
beginning of line
middle of line
end of line
```

The visible caret must always follow the actual Neovim cursor.

---

# 9. PRINTABLE CHARACTERS

Every ordinary character must behave identically whether LSP is:

```text
not installed
installed but inactive
starting
connected
publishing diagnostics
serving completion
```

For example:

```text
a
b
c
j
k
x
y
z
```

must never suddenly acquire different caret behavior.

---

# 10. LSP MUST NEVER BLOCK THE UI

Audit for:

- synchronous RPC
- blocking receives
- blocking process startup
- blocking installation
- locks held during rendering
- synchronous LSP requests on egui thread
- waiting for server initialization

The UI must remain responsive while:

```text
LSP starts
LSP crashes
LSP restarts
LSP responds slowly
LSP publishes diagnostics
LSP sends completion
```

---

# 11. SERVER LIFECYCLE

Test:

```text
start
initialize
ready
shutdown
restart
crash
reconnect
missing executable
invalid configuration
```

A broken language server must never freeze or crash MindForge.

---

# 12. MULTIPLE FILES

Open multiple Python files.

Then:

```text
file A
 ↓
file B
 ↓
file C
 ↓
file A
```

Verify:

- correct diagnostics
- correct cursor
- correct completion
- correct workspace
- no stale diagnostics
- no stale completion popup
- no stale cursor
- no duplicated server
- no cross-document state corruption

---

# 13. MULTIPLE LANGUAGES

Test at minimum:

```text
Python
HTML
CSS
JavaScript
TypeScript
TSX
Rust
JSON
```

A Python LSP must never affect an HTML buffer.

A TypeScript server must never corrupt a Python editor.

---

# 14. WORKSPACE ROOT

Verify proper workspace detection.

Test projects containing:

```text
package.json
tsconfig.json
pyproject.toml
Cargo.toml
.git
```

The language server should receive the correct workspace/root.

Do not assume:

```text
workspace = current file directory
```

---

# 15. LSP + TABS

Test:

```text
open A
open B
open C
switch A
switch B
close B
switch C
```

Completion, diagnostics and cursor state must follow the active document.

No stale popup should remain attached to a closed document.

---

# 16. LSP + FUZZY SEARCH

Test:

```text
Ctrl+P
```

while:

- editor is active
- completion is active
- diagnostics are visible

Ensure there is no keyboard/event conflict.

The existing fuzzy finder must remain functional.

---

# 17. LSP + TERMINAL

Test terminal shortcuts while an LSP-enabled editor is open.

Opening the terminal must not:

- alter editor cursor
- alter completion state
- freeze the application
- steal events permanently

---

# 18. LSP + HYBRID

Switch:

```text
Hybrid
 ↕
Neovim
```

on LSP-enabled files.

Verify:

- document contents
- cursor
- scroll
- language
- diagnostics
- completion
- syntax highlighting

remain coherent.

Do not create a second LSP system unnecessarily.

---

# 19. DIAGNOSTIC STORM

Create a file with many deliberate errors.

Verify:

```text
100+ diagnostics
```

does not:

- freeze UI
- reorder the document
- move cursor
- change line heights unexpectedly
- corrupt rendering
- create huge inline text

The editor should remain usable.

---

# 20. LARGE FILES

Test reasonably large files.

Verify that:

- typing remains responsive
- cursor remains correct
- diagnostics don't block UI
- completion remains responsive
- redraw processing remains stable

Do not perform whole-document work every frame.

---

# 21. LSP SERVER FAILURE

Kill the language server while editing.

MindForge must:

```text
remain usable
```

and show an appropriate unavailable/error state.

It must not:

```text
freeze
crash
lose the buffer
corrupt the cursor
corrupt the editor
```

---

# 22. STALE EVENTS

This is particularly important.

Asynchronous LSP/Neovim events may arrive after:

- switching tabs
- closing a file
- changing mode
- restarting a server

Make sure stale events cannot modify the wrong active document.

Every asynchronous event must have appropriate document/session ownership.

---

# 23. RENDERING CONTRACT

Establish a strict rule:

```text
Document content
    ↓
Neovim authoritative buffer

Cursor
    ↓
Neovim authoritative cursor

LSP information
    ↓
metadata/overlay

MindForge
    ↓
rendering
```

Diagnostics, completion, hover and other LSP information must never mutate the authoritative text renderer directly.

---

# 24. DO NOT PATCH SYMPTOMS

Do NOT solve bugs using:

```text
sleep()
delay
setTimeout-like hacks
arbitrary repaint loops
cursor += 1
cursor -= 1
hard-coded row offsets
forced cursor resets
duplicate event forwarding
```

If the cursor is wrong, find out why.

If an event is duplicated, find out why.

If redraw state is stale, fix event ordering/state ownership.

---

# 25. OBSERVABILITY

Add lightweight diagnostic logging during development.

For difficult bugs, be able to inspect:

```text
Input event
Neovim event
Document/session ID
Cursor position
Grid cursor
Completion state
Diagnostic update
```

Do not permanently spam production logs.

Make debugging easy to enable and disable.

---

# 26. REGRESSION MATRIX

Build a manual/automated checklist covering:

### Input

- [ ] normal typing
- [ ] Backspace
- [ ] Delete
- [ ] Enter
- [ ] Escape
- [ ] arrows
- [ ] Ctrl+J
- [ ] Ctrl+K
- [ ] Tab

### Cursor

- [ ] beginning of line
- [ ] middle
- [ ] end
- [ ] empty line
- [ ] multiline
- [ ] Unicode

### Completion

- [ ] automatic
- [ ] manual trigger
- [ ] selection
- [ ] navigation
- [ ] acceptance
- [ ] cancellation
- [ ] documentation

### Diagnostics

- [ ] one error
- [ ] many errors
- [ ] warning
- [ ] error
- [ ] clearing diagnostics

### Lifecycle

- [ ] server start
- [ ] server ready
- [ ] server crash
- [ ] server restart
- [ ] tab switch
- [ ] file close
- [ ] application close

### Languages

- [ ] Python
- [ ] HTML
- [ ] CSS
- [ ] JavaScript
- [ ] TypeScript
- [ ] TSX
- [ ] Rust
- [ ] JSON

---

# 27. IMPORTANT: DO NOT STOP AT THE FIRST FIX

The objective is NOT:

> "Fix the cursor bug."

The objective is:

> **Audit the entire LSP integration and leave it in a stable production-ready state.**

If fixing one issue exposes another, continue investigating.

Do not stop merely because the original reproduction case works.

---

# 28. PROTECT EXISTING MIND FORGE

Do not regress:

- Markdown
- Wikilinks
- Backlinks
- Graph
- SQLite
- Tabs
- Explorer
- Fuzzy finder
- Terminal
- Hybrid editor
- Neovim
- Vim motions
- command line
- status bar
- file switching

LSP must integrate into MindForge.

MindForge must not become an LSP application with the rest of the product bolted around it.

---

# 29. FINAL DEFINITION OF DONE

Do not report "LSP fixed" simply because:

```text
completion works
```

The integration is complete only when:

```text
Typing
    ✓

Cursor
    ✓

Backspace
    ✓

Enter
    ✓

Completion
    ✓

Diagnostics
    ✓

Hover
    ✓

Redraw
    ✓

Neovim synchronization
    ✓

Multiple files
    ✓

Multiple languages
    ✓

Server lifecycle
    ✓

Failure recovery
    ✓

Performance
    ✓

UI responsiveness
    ✓
```

and there are no known synchronization, input-routing, rendering, or lifecycle regressions.

---

# FINAL INSTRUCTION

**Take ownership of the entire LSP integration.**

Do not wait for me to discover the next bug.

Inspect the implementation, reproduce the known failures, identify their root causes, and systematically test the surrounding systems for related failures.

Fix the architecture rather than patching individual symptoms.

Keep Neovim as the authoritative editor engine.

Keep LSP as the language-intelligence layer.

Keep MindForge as the UI and knowledge environment.

The final result should feel like:

> **A stable, native MindForge editor with real Neovim semantics and professional language intelligence — not a collection of loosely connected features.**

Work carefully, incrementally, and verify every change.
