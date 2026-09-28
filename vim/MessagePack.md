# CODEX MASTER TASK — CLEAN EDITOR ARCHITECTURE + NATIVE NEOVIM VIM MODE

You are working on an existing, mature Rust + egui desktop editor application.

This is NOT a prototype.

The application already has a large, polished UI and many working features. Your job is to improve the **editor architecture and Vim integration** without destroying, replacing, or unnecessarily modifying the existing application.

## 1. ABSOLUTE RULE

DO NOT redesign or replace the existing application UI.

Keep all existing application features working:

* titlebar
* sidebar
* tabs
* backlinks
* wikilinks
* command line
* terminal
* multiple terminal sessions
* Markdown editing
* Markdown live preview
* documentation
* file management
* navigation
* search
* knowledge features
* existing themes
* existing keyboard shortcuts that belong to the application
* existing layout
* existing performance optimizations
* all other existing functionality

The task is specifically to create a clean editor architecture where:

```text
                    APPLICATION
                         │
                ┌────────┴────────┐
                │                 │
             HYBRID              VIM
                │                 │
        Existing editor       Neovim backend
            engine                 │
                                   │
                                Neovim
```

The surrounding application remains ours.

Only the editor engine changes depending on the selected editing mode.

---

# 2. TWO COMPLETELY SEPARATE EDITOR SYSTEMS

The current Vim and Hybrid implementations are not clean enough.

Refactor them.

They must become clearly separated modules.

The target structure should be approximately:

```text
src/
├── editor/
│   ├── mod.rs
│   ├── backend.rs
│   ├── controller.rs
│   ├── events.rs
│   ├── types.rs
│   │
│   ├── hybrid/
│   │   ├── mod.rs
│   │   ├── backend.rs
│   │   ├── engine.rs
│   │   ├── state.rs
│   │   ├── input.rs
│   │   ├── cursor.rs
│   │   ├── selection.rs
│   │   ├── rendering.rs
│   │   ├── commands.rs
│   │   ├── navigation.rs
│   │   └── tests.rs
│   │
│   └── vim/
│       ├── mod.rs
│       ├── backend.rs
│       ├── client.rs
│       ├── input.rs
│       ├── events.rs
│       ├── state.rs
│       ├── renderer.rs
│       ├── cursor.rs
│       ├── highlights.rs
│       ├── lifecycle.rs
│       ├── protocol.rs
│       ├── resize.rs
│       └── tests.rs
│
├── document/
│   ├── ...
│
├── ui/
│   ├── ...
│
└── ...
```

Adapt this structure to the actual project rather than blindly copying it.

The important architectural rule is:

```text
editor/
    hybrid/
    vim/
```

Hybrid-specific implementation must stay inside `hybrid/`.

Vim/Neovim-specific implementation must stay inside `vim/`.

Do NOT mix them.

---

# 3. CLEAN UP THE EXISTING CODE FIRST

Before writing code:

1. Inspect the entire existing editor implementation.
2. Identify:

   * current editor files
   * current Vim files
   * current Hybrid files
   * enums
   * keyboard handlers
   * cursor logic
   * selection logic
   * command handling
   * rendering
   * document synchronization
   * mode switching
   * save logic
   * undo/redo
   * existing Vim key handling
3. Determine which code belongs to:

   * Hybrid
   * Vim
   * shared infrastructure
4. Move code into the correct architecture.
5. Remove duplicated logic.
6. Remove obsolete Vim implementation once the Neovim backend replaces it.
7. Do NOT leave dead files behind.

Do not simply create a second implementation beside the old one.

The final codebase should have ONE clear implementation path for each mode.

---

# 4. EDITOR MODE ARCHITECTURE

Create a clean editor abstraction.

For example:

```rust
pub enum EditorMode {
    Hybrid,
    Vim,
}
```

If the existing application already has a mode representation, refactor it instead of creating a competing enum.

The enum must be clean and centralized.

Do NOT scatter strings such as:

```rust
"vim"
"hybrid"
```

throughout the application.

Use strongly typed Rust enums.

---

# 5. BACKEND ABSTRACTION

Create a clean abstraction between the application and the editor engines.

For example:

```rust
pub trait EditorBackend {
    fn handle_key(&mut self, event: EditorKeyEvent);

    fn handle_mouse(&mut self, event: EditorMouseEvent);

    fn render(&mut self, ui: &mut egui::Ui);

    fn tick(&mut self);

    fn mode(&self) -> EditorMode;

    fn is_dirty(&self) -> bool;

    fn save(&mut self) -> EditorResult<()>;

    fn shutdown(&mut self);
}
```

Adapt the exact API to the existing application.

The application should NOT need to know whether the editor is Hybrid or Neovim.

Instead:

```text
Application
     │
     ▼
EditorController
     │
     ├── HybridBackend
     │
     └── VimBackend
              │
              ▼
           Neovim
```

---

# 6. HYBRID MODE

Hybrid mode should remain the application's existing custom editor.

Move all Hybrid-specific functionality into:

```text
editor/hybrid/
```

Hybrid should own:

* text editing
* cursor
* selection
* navigation
* rendering
* keyboard behavior
* commands
* existing editing features
* existing Markdown editing behavior
* existing optimizations

Do not introduce Neovim dependencies into Hybrid.

Hybrid must continue working independently.

If Hybrid currently works correctly, preserve its behavior.

Do NOT rewrite functioning Hybrid functionality unnecessarily.

---

# 7. VIM MODE — NEOVIM MUST BE THE REAL EDITOR

This is the most important requirement.

DO NOT implement Vim mode by manually recreating Vim behavior.

Do NOT continue adding hard-coded handlers such as:

```rust
if key == Key::H { ... }

if key == Key::J { ... }

if key == Key::K { ... }

if key == Key::L { ... }
```

Do NOT create another homemade Vim state machine.

Instead:

```text
egui keyboard event
        ↓
Rust Vim backend
        ↓
Neovim
        ↓
Neovim processes Vim semantics
        ↓
Neovim UI events
        ↓
Rust
        ↓
egui rendering
```

Neovim becomes the source of truth for Vim-mode editing.

---

# 8. EMBED NEOVIM

Use embedded/headless Neovim rather than opening a terminal window.

Conceptually:

```text
nvim --embed
```

Communicate using Neovim's RPC protocol.

Use an appropriate Rust Neovim/msgpack-RPC implementation.

Do not make the existing terminal UI the Vim interface.

The existing egui editor surface remains the visual frontend.

---

# 9. NEOVIM UI PROTOCOL

Use Neovim's UI API.

The preferred architecture should use:

```text
nvim_ui_attach()
```

with the appropriate UI extensions.

At minimum investigate and use where appropriate:

```text
ext_linegrid
ext_hlstate
ext_popupmenu
ext_cmdline
ext_messages
```

Consider:

```text
ext_multigrid
```

only where it provides real value for our editor surface.

Do not blindly enable features without understanding their rendering implications.

The goal is to receive Neovim's state and render it efficiently through egui.

---

# 10. NEOVIM FEATURES MUST WORK NATIVELY

Once keyboard input reaches Neovim, Neovim must handle Vim semantics.

That includes, where applicable:

### Motions

```text
h j k l
w
b
e
0
^
$
gg
G
%
f
F
t
T
;
,
```

### Operators

```text
d
c
y
>
<
g~
gu
gU
```

### Operator combinations

```text
dd
dw
de
d$
ciw
ci"
ci(
diw
yiw
yy
cc
```

### Modes

```text
Normal
Insert
Visual
Visual Line
Visual Block
Replace
Command-line
Operator-pending
```

### Registers

```text
"
0
1
2
+
*
_
```

where supported by the Neovim environment.

### Other Vim functionality

```text
macros
marks
counts
search
search navigation
undo
redo
text objects
jump lists
registers
folding
command mode
colon commands
visual selections
operator-pending state
insert mode behavior
repeat .
```

Do NOT manually implement these.

Neovim implements them.

---

# 11. KEYBOARD INPUT

The input layer must be extremely robust.

egui receives the physical/user input.

The Vim backend converts that event into the appropriate Neovim input representation.

Then sends it to Neovim.

The Rust layer should NOT interpret Vim semantics.

Bad:

```rust
if key == Key::D {
    delete_word();
}
```

Good:

```text
egui KeyEvent
      ↓
VimInputTranslator
      ↓
Neovim input
      ↓
Neovim decides what "d" means
```

This allows:

```text
d
dd
dw
ciw
3dw
2dd
gqap
```

and all other combinations to work naturally.

---

# 12. INPUT OWNERSHIP

This is critical.

When Vim mode is active and the editor has focus:

```text
editor key
     ↓
Neovim
```

The application must not intercept normal Vim keys.

Do not let egui consume:

```text
h
j
k
l
w
b
d
c
y
i
a
o
v
:
/
?
%
g
f
t
etc.
```

before Neovim receives them.

Application-global shortcuts can still exist, but define a clear policy for them.

For example:

```text
Application-global shortcut
        ↓
Application

Editor key
        ↓
Neovim
```

Avoid accidental conflicts.

---

# 13. TEXT INPUT

Do not only support individual key presses.

Handle proper text input.

Support:

* Unicode
* multiline input
* punctuation
* symbols
* modifier combinations
* Ctrl
* Alt
* Shift
* special keys
* Backspace
* Delete
* Enter
* Tab
* Escape
* arrows
* Home
* End
* PageUp
* PageDown
* function keys where appropriate

Be careful with IME/text composition if the application supports it.

Do not corrupt Unicode input.

---

# 14. NEOVIM STATE

Create a dedicated state representation for the Vim backend.

For example:

```rust
pub struct NeovimState {
    mode: VimMode,
    cursor: CursorPosition,
    grids: GridState,
    highlights: HighlightState,
    popup_menu: Option<PopupMenuState>,
    command_line: Option<CommandLineState>,
    messages: Vec<Message>,
    dirty: bool,
}
```

Adapt this to the actual UI protocol.

The important thing is:

Neovim owns editing state.

Rust stores the UI state required to render it.

---

# 15. RENDERING

Do not render Neovim through a terminal emulator.

Render the editor surface directly in egui.

The flow should be:

```text
Neovim redraw event
        ↓
Neovim event parser
        ↓
Vim UI state
        ↓
egui renderer
```

The renderer should efficiently handle:

* text
* cursor
* highlights
* selections
* syntax colors
* line/grid updates
* popup menus
* command-line UI where required
* messages where required

Avoid rebuilding the entire editor UI when one line changes.

---

# 16. PERFORMANCE

Performance is a first-class requirement.

Do NOT create an architecture that technically works but causes:

* excessive allocations
* unnecessary cloning
* full-buffer copies every frame
* full-document synchronization every keystroke
* blocking calls on the egui UI thread
* unnecessary RPC calls
* repeated layout calculations
* excessive redraws
* excessive mutex contention

Avoid:

```rust
buffer.clone()
```

on every frame.

Avoid:

```text
Neovim → entire document
       → egui
       → entire document
       → every frame
```

Instead use incremental updates wherever possible.

---

# 17. THREADING

Do not block the egui rendering thread waiting for Neovim.

Use a dedicated communication/runtime layer.

Conceptually:

```text
                EGUI THREAD
                    │
                    │ channels
                    ▼
             NEOVIM RUNTIME
                    │
                    ▼
                 Neovim
```

For example:

```rust
enum NeovimCommand {
    Input(String),
    Resize {
        width: u64,
        height: u64,
    },
    Command(String),
    Save,
    OpenBuffer(PathBuf),
    Shutdown,
}
```

And:

```rust
enum NeovimEvent {
    Redraw(...),
    ModeChanged(...),
    CursorMoved(...),
    BufferModified(...),
    Resize(...),
    HighlightDefined(...),
    PopupMenu(...),
    CommandLine(...),
    Message(...),
    Exit(...),
    Error(...),
}
```

Use the project's existing async/runtime architecture if one already exists.

Do not introduce unnecessary runtimes.

---

# 18. RESIZE PERFORMANCE

egui window resizing can generate a huge number of resize events.

Do NOT send a Neovim resize RPC for every tiny intermediate resize.

Coalesce resize requests.

For example:

```text
100 resize events
       ↓
coalesce
       ↓
latest dimensions
       ↓
one Neovim resize
```

This must remain responsive.

---

# 19. DOCUMENT SYNCHRONIZATION

This is extremely important.

There must NOT be two competing sources of truth while Vim mode is active.

### Hybrid

```text
Application document
       ↓
Hybrid editor
```

### Vim

```text
Neovim buffer
       ↓
Vim editor
```

When Vim mode is active:

```text
Neovim = authoritative editor state
```

The application can observe changes for:

* Markdown preview
* backlinks
* wikilinks
* indexing
* dirty state
* search
* metadata
* document analysis

But it must not independently modify the same buffer without coordinating through Neovim.

---

# 20. MODE SWITCHING

Implement clean transitions.

### Hybrid → Vim

```text
Current Hybrid document
        ↓
Create/update Neovim buffer
        ↓
Set cursor position
        ↓
Set file path/buffer metadata
        ↓
Activate Vim backend
```

### Vim → Hybrid

```text
Neovim buffer
        ↓
Commit/synchronize document
        ↓
Hybrid receives latest content
        ↓
Restore cursor/selection where possible
        ↓
Activate Hybrid backend
```

Do this only at mode boundaries.

Do not continuously synchronize both engines.

---

# 21. FILE SAVING

Inspect the existing application's save architecture before implementing this.

Do not create two competing save systems.

Choose one authoritative strategy.

A reasonable Vim-mode approach is:

```text
Neovim buffer
      ↓
Neovim :write / buffer write
      ↓
filesystem
      ↓
application observes saved state
```

OR integrate the application's existing persistence layer with Neovim buffer state if that is architecturally safer.

Make the decision based on the existing code.

Document the decision.

---

# 22. LIFECYCLE

Neovim must have a robust lifecycle.

Handle:

```text
startup
initialization
UI attach
buffer creation
buffer loading
resize
input
redraw
save
mode switching
application shutdown
Neovim crash
Neovim exit
restart/recovery
```

Do not leave orphaned Neovim processes.

Do not panic if Neovim exits unexpectedly.

Return proper errors.

---

# 23. ERROR HANDLING

Do not use:

```rust
unwrap()
```

or:

```rust
expect(...)
```

for recoverable runtime operations.

Use proper error types.

For example:

```rust
pub enum EditorError {
    NeovimStartFailed(...),
    RpcError(...),
    UiAttachFailed(...),
    BufferError(...),
    InputError(...),
    ShutdownError(...),
}
```

Adapt to the project's existing error architecture.

Errors should be diagnosable.

---

# 24. COMMENTS

Write comments for architecture and non-obvious logic.

Good:

```rust
// Neovim owns Vim-mode editing semantics.
// The egui layer only translates input and renders
// the state emitted by the Neovim UI protocol.
```

Bad:

```rust
// increment i
i += 1;
```

Comments should explain:

* WHY
* ownership
* synchronization rules
* threading decisions
* protocol behavior
* performance decisions
* lifecycle requirements

Do not litter every line with useless comments.

---

# 25. CODE QUALITY

This must be production-quality Rust.

Priorities:

1. correctness
2. architecture
3. performance
4. maintainability
5. readability
6. testability

Do not rush.

Do not create giant files.

Do not create one 2,000-line Vim module.

Split functionality by responsibility.

Use:

* structs
* enums
* traits
* modules
* clear ownership
* explicit state transitions
* proper error handling

Avoid unnecessary abstractions.

---

# 26. TESTING

Create tests for the new architecture.

At minimum test:

### Architecture

* Hybrid backend initializes.
* Vim backend initializes.
* Switching Hybrid → Vim.
* Switching Vim → Hybrid.

### Input

* normal mode input
* insert mode
* escape
* motions
* operators
* counts
* visual mode
* command mode
* special keys

### Synchronization

* document loading
* edits
* dirty state
* save
* switching modes

### Lifecycle

* startup
* shutdown
* Neovim exit
* resize
* errors

### Performance

Ensure no obvious:

* per-frame full document cloning
* blocking RPC
* unbounded event queue
* unnecessary redraw loop

---

# 27. REMOVE THE OLD HARD-CODED VIM IMPLEMENTATION

Once the Neovim backend is working:

Find and remove obsolete Vim implementations.

Especially remove code that manually implements:

```text
hjkl
dd
dw
ciw
yy
p
visual mode
operator pending
registers
macros
counts
undo
redo
etc.
```

Do not keep two Vim engines.

There must be ONE Vim engine:

```text
Neovim
```

---

# 28. DO NOT BREAK THE APPLICATION

Before modifying anything:

Understand the existing architecture.

Then refactor incrementally.

Do not:

* replace the UI
* rewrite unrelated features
* replace egui
* replace the document system unnecessarily
* replace the terminal
* replace the Markdown renderer
* replace the sidebar
* replace backlinks
* replace wikilinks
* remove existing features
* introduce unnecessary dependencies

This is an editor-engine refactor, not an application rewrite.

---

# 29. DEPENDENCIES

Inspect the existing `Cargo.toml`.

Use existing dependencies where possible.

For Neovim integration, choose a mature Rust library appropriate for:

* msgpack-RPC
* Neovim embedding
* asynchronous communication
* notifications
* requests
* UI events

Do not add multiple overlapping Neovim/RPC libraries.

Before adding a dependency, verify that it is actually needed.

---

# 30. IMPLEMENTATION ORDER

Follow this order.

### PHASE 1 — AUDIT

Inspect:

```text
Cargo.toml
src/
editor code
Vim code
Hybrid code
input handling
document model
save system
rendering
mode switching
```

Produce a concise architecture map before making major changes.

### PHASE 2 — CLEAN STRUCTURE

Create:

```text
editor/
    hybrid/
    vim/
```

Move existing code into the correct modules.

Make the project compile.

Do not change behavior unnecessarily.

### PHASE 3 — BACKEND ABSTRACTION

Implement:

```text
EditorMode
EditorBackend
EditorController
```

Integrate with existing application state.

### PHASE 4 — NEOVIM

Implement:

```text
NeovimClient
NeovimBackend
Neovim lifecycle
RPC
UI attach
redraw handling
input
```

### PHASE 5 — RENDERING

Render Neovim state through the existing egui editor surface.

### PHASE 6 — DOCUMENT INTEGRATION

Integrate:

```text
buffer
dirty state
save
Markdown preview
backlinks
wikilinks
indexing
```

without creating competing sources of truth.

### PHASE 7 — MODE SWITCHING

Implement reliable:

```text
Hybrid ↔ Vim
```

transitions.

### PHASE 8 — TESTING

Test all critical paths.

### PHASE 9 — CLEANUP

Remove:

* obsolete Vim handlers
* dead files
* duplicated logic
* unused imports
* unused dependencies
* obsolete enums
* temporary code
* debug code

Run:

```bash
cargo check
cargo test
cargo clippy
```

and format with:

```bash
cargo fmt
```

Fix all relevant warnings/errors.

---

# 31. IMPORTANT: INSPECT BEFORE CODING

Do NOT immediately start writing hundreds of lines.

First inspect the repository.

Understand what already exists.

Then make a concrete migration plan based on the real codebase.

If the current architecture differs from this specification, adapt intelligently.

Do not force the project into an artificial structure merely because this prompt shows an example.

---

# 32. FINAL ARCHITECTURAL GOAL

The final architecture should feel like this:

```text
                         APPLICATION
                              │
                 ┌────────────┴────────────┐
                 │                         │
             EditorController          Other UI
                 │
        ┌────────┴─────────┐
        │                  │
     HYBRID               VIM
        │                  │
 HybridBackend       NeovimBackend
        │                  │
 HybridEngine         NeovimClient
                           │
                        Neovim
```

Hybrid and Vim are independent.

Neovim owns Vim-mode editing semantics.

egui owns the application's visual interface.

The application owns application-level features.

There is no homemade Vim engine.

There is no duplicated source of truth.

There is no unnecessary full-buffer synchronization every frame.

There is no blocking Neovim communication on the UI thread.

There is no rushed implementation.

The result must be clean, maintainable, production-quality Rust with excellent performance.

## MOST IMPORTANT PRINCIPLE

Do not think:

> "How do I implement every Vim key?"

Think:

> "How do I make every Vim key reach Neovim correctly?"

Neovim already knows what every Vim key means.

Our job is to build an excellent bridge between:

```text
egui
  ↕
Rust Vim Backend
  ↕
Neovim
```

while leaving the rest of the application completely intact.
----


we already implemented the core bridge between egui, the Rust Vim backend, and Neovim.

we need polish-1.md
