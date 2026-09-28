# CRITICAL: FIX THE NEOVIM EDITOR FREEZE AND MAKE NEOVIM THE REAL EDITOR

The current problem is now clear:

**When an editor is open, especially an editor containing substantial existing Markdown/text, the entire Lumina application freezes.**

When no editor is open, Lumina remains responsive:

* Ctrl+N works
* opening another editor works
* typing works
* fuzzy finder works

Therefore, do NOT treat this as a general application performance problem.

The failure is specifically in the **Neovim-backed editor lifecycle/render/input/synchronization path**.

## PRIMARY OBJECTIVE

Make the embedded Neovim editor work as a real, comfortable Neovim editor inside Lumina.

Do not fake Vim behavior.

Do not maintain a second Vim engine.

Do not continuously synchronize the entire document between Neovim and the old editor.

Do not allow the old editor renderer and Neovim renderer to fight over the same editor surface.

Neovim must be the authoritative editor engine while Vim mode is active.

---

# 1. FIX THE FREEZE FIRST

Before visual polish, find exactly what blocks the application when an editor is open.

Trace the complete path:

```text
egui frame
    ↓
editor update
    ↓
Neovim runtime
    ↓
RPC
    ↓
Neovim redraw/events
    ↓
Lumina state update
    ↓
editor rendering
```

Find every place where this path can:

* block
* wait synchronously
* acquire a lock indefinitely
* perform synchronous RPC
* wait for a channel
* perform a full-document clone
* rebuild the entire document every frame
* rebuild layout every frame
* repeatedly send the same Neovim state back and forth
* cause a redraw → state update → redraw loop

The egui/UI thread MUST NEVER wait for Neovim.

There must be no blocking Neovim RPC on the UI thread.

There must be no:

```rust
recv()
recv_timeout(...)
send(...)
```

that can block the UI thread.

There must be no synchronous startup/request path that can freeze the application while the editor is rendering.

---

# 2. ASSUME EXISTING DOCUMENT SIZE IS EXPOSING THE BUG

An empty/new editor may appear to work differently from an existing document.

Test specifically with:

```text
empty document
small document
medium document
large document
very large document
```

Do not solve this by limiting the document or deleting old text.

The editor must comfortably open existing Markdown files.

Look especially for code that does this every frame:

```rust
clone entire buffer
rebuild all lines
recalculate entire layout
compute inline Markdown layout
rebuild cursor position
rebuild visual lines
```

That is unacceptable for the Neovim path.

---

# 3. NEOVIM MUST OWN EDITING STATE

When Vim mode is active:

```text
Neovim buffer = source of truth
Neovim cursor = source of truth
Neovim mode = source of truth
Neovim undo history = source of truth
Neovim registers = source of truth
Neovim motions = source of truth
Neovim commands = source of truth
```

Lumina should observe Neovim state when needed for application features.

Do NOT have:

```text
Neovim buffer
      ↕
Lumina editor buffer
      ↕
old Vim engine
```

That creates competing authorities and synchronization loops.

Instead:

```text
              ┌───────────────┐
              │    Neovim     │
              │ source of     │
              │ truth         │
              └───────┬───────┘
                      │
             redraw/events
                      │
                      ▼
              ┌───────────────┐
              │ Lumina editor │
              │ presentation  │
              └───────────────┘
```

---

# 4. NEOVIM RENDERING MUST BE PROPERLY EMBEDDED

Neovim should NOT be treated as a second editor window placed over Lumina.

It should render its editor state INSIDE the existing Lumina editor rectangle.

The existing editor surface is the container.

Conceptually:

```text
┌──────────────────────────────────────────┐
│ Lumina editor surface                    │
│                                          │
│ # My Document                            │
│                                          │
│ This is some Markdown text.              │
│                                          │
│ [[Another Note]]                         │
│                                          │
│                                          │
└──────────────────────────────────────────┘
```

Neovim's UI protocol should provide the information needed to render the editor surface.

Use the proper Neovim UI mechanism rather than trying to reproduce Neovim's terminal UI manually.

The intended architecture is:

```text
nvim --embed
      │
      │ msgpack-RPC
      ▼
nvim_ui_attach(...)
      │
      ├── ext_linegrid
      ├── highlights
      ├── cursor
      ├── mode
      ├── scroll/redraw events
      └── other required UI events
```

Render those events into the exact Lumina editor rectangle.

---

# 5. DO NOT CONFUSE SCREEN COLUMNS WITH TEXT OFFSETS

This is critical.

Neovim's UI grid coordinates are NOT automatically equivalent to:

```rust
String byte index
```

Do not do things like:

```rust
self.grid.cursor.column
```

and treat that directly as a Rust string/byte offset.

The rendering coordinate system should remain:

```text
Neovim screen row
Neovim screen column
```

while document text indexing should remain its own representation.

Only convert between coordinate systems when explicitly necessary.

---

# 6. REMOVE OLD VIM IMPLEMENTATION FROM THE ACTIVE PATH

The old homemade Vim implementation must not process keyboard input while Neovim is active.

Do not allow:

```text
egui key
   ├── old Vim engine
   └── Neovim
```

That can create conflicts and duplicate work.

Instead:

```text
egui input
     ↓
Vim backend
     ↓
Neovim input
```

Neovim decides:

* normal mode
* insert mode
* visual mode
* command-line mode
* motions
* operators
* text objects
* registers
* macros
* undo
* redo
* search
* commands
* mappings

Do not recreate these in Rust.

---

# 7. NORMAL TEXT INPUT MUST REACH NEOVIM

Do not assume every typed character arrives as:

```rust
egui::Event::Key
```

Normal text commonly arrives through text input events.

Make sure the Vim input layer handles both appropriately:

```text
keyboard special keys
        ↓
Neovim input

text characters
        ↓
Neovim input
```

For example, typing:

```text
# Hello World
```

must actually insert those characters through Neovim.

Then:

```text
Esc
h
j
k
l
dd
u
i
```

must be interpreted by Neovim itself.

---

# 8. RAW MARKDOWN ONLY

For Vim mode, the editor contents are raw Markdown.

Example:

```markdown
# Hello

This is **bold**.

[[WikiLink]]
```

Do NOT run the old inline Markdown renderer/layout engine over the Neovim editor.

The Markdown preview remains a separate Lumina feature.

Therefore, while Vim mode is active:

* no inline Markdown transformation
* no old inline layout engine
* no old editor line-layout system
* no duplicate cursor renderer
* no duplicate text renderer

Neovim's grid is the editor presentation model.

---

# 9. EDITOR RECTANGLE MUST BE THE ONLY RENDERING AREA

All Neovim drawing must be clipped to:

```rust
actual_editor_rect
```

Nothing from the Neovim renderer should appear in:

* titlebar
* sidebar
* tabs
* command line
* surrounding UI
* outside the editor

The cursor must never appear in the titlebar.

The scrollbar must never appear above/outside the editor.

Do not use the global window origin as the editor origin.

Use:

```text
actual_editor_rect.min
```

as the rendering origin.

---

# 10. SCROLLING

Do not create a second independent scrolling model unless absolutely necessary.

Neovim should control its viewport/grid state.

Lumina renders that viewport.

If Neovim reports:

```text
grid scroll
grid line updates
cursor movement
window resize
```

apply those changes efficiently.

Do NOT rebuild the entire document for every scroll event.

---

# 11. RESIZE MUST BE COALESCED

When the Lumina editor rectangle changes size:

```text
egui
  ↓
resize event
  ↓
Neovim nvim_ui_try_resize(...)
```

Do not send dozens of resize requests during one resize gesture.

Coalesce them.

Only send the latest dimensions.

---

# 12. REDRAW EVENTS MUST NOT CAUSE A FEEDBACK LOOP

Be very careful about:

```text
Neovim redraw
    ↓
Lumina state update
    ↓
Lumina requests Neovim update
    ↓
Neovim redraw
    ↓
repeat forever
```

Only send requests to Neovim when something actually changed.

Likewise, only update Lumina presentation state when the Neovim event represents a real change.

---

# 13. KEEP NEOVIM RUNTIME OFF THE UI THREAD

Preferred architecture:

```text
                 EGUI THREAD
                     │
                     │ commands
                     ▼
              ┌──────────────┐
              │ Vim Backend  │
              └──────┬───────┘
                     │
                 channel
                     │
                     ▼
              ┌──────────────┐
              │ Nvim Runtime │
              │ worker       │
              └──────┬───────┘
                     │
                 msgpack RPC
                     │
                     ▼
                 NEOVIM
```

The worker communicates with Neovim.

The egui thread consumes already-available events without waiting for Neovim.

---

# 14. DO NOT USE FULL DOCUMENT SYNCHRONIZATION EVERY FRAME

This is one of the most important requirements.

NEVER do:

```rust
every_frame:
    get entire Neovim buffer
    clone entire buffer
    rebuild entire layout
```

Instead:

```text
Neovim redraw/change
       ↓
incremental update
       ↓
render only required state
```

For application features such as:

* backlinks
* wikilinks
* graph
* search
* indexing
* Markdown preview

use appropriate change notifications/debounced synchronization.

Those features do NOT justify rebuilding the editor every frame.

---

# 15. FILE OPENING

When opening an existing Markdown file:

```text
File
 ↓
Neovim buffer initialization
 ↓
set buffer contents once
 ↓
attach UI
 ↓
render
```

Do not repeatedly load the same file into Neovim.

Do not initialize the same buffer multiple times.

Do not trigger a full buffer synchronization during every frame.

Verify that opening an existing document with thousands of lines remains responsive.

---

# 16. COMMANDS MUST ACTUALLY BE NEOVIM COMMANDS

If the user enters a Vim command such as:

```text
:w
:q
:e
:%s/foo/bar/g
```

or uses normal-mode commands/mappings, they should be interpreted by the embedded Neovim instance.

Do not create a parallel Rust command interpreter for Vim semantics.

Lumina-specific commands may remain Lumina commands.

Keep the boundary clear:

```text
Neovim:
Vim editing semantics and Vim commands

Lumina:
Application commands and application features
```

---

# 17. PERFORMANCE REQUIREMENT

The editor must remain responsive with:

```text
100 lines
1,000 lines
10,000 lines
large Markdown files
```

Do not optimize only the empty-editor case.

The important test is:

> Open an existing large document and immediately type, move, delete, scroll, search, and switch modes without Lumina freezing.

---

# 18. TEMPORARILY REMOVE NONESSENTIAL WORK

While diagnosing this issue, disable anything that is not required for basic Neovim editing:

* inline Markdown layout
* expensive syntax processing
* unnecessary indexing on every keystroke
* full-document preview regeneration
* unnecessary graph updates
* unnecessary backlinks recomputation
* expensive diagnostics
* debug rendering
* performance overlays if they interfere

First make this work:

```text
open file
↓
render file
↓
type
↓
Esc
↓
move
↓
delete
↓
un
```
