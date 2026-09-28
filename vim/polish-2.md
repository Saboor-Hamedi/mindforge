# CRITICAL — THE CURRENT APP IS FREEZING

Stop working on tests and stop adding features.

The current application is now **completely frozen** when I launch it.

The app becomes:

> Not Responding

I cannot:

* type
* delete
* move the cursor
* interact normally with the editor
* use the application reliably

This is a **critical runtime problem**.

Fix the actual application first.

---

# 1. FIRST PRIORITY: FIND WHY THE APP FREEZES

Do NOT assume the problem.

Inspect the current implementation and find the exact blocking operation.

Pay particular attention to:

* Neovim startup
* embedded Neovim process
* RPC connection
* msgpack communication
* Tokio/runtime integration
* synchronous requests
* blocking reads
* blocking waits
* mutex locks
* channel waits
* event loops
* redraw handling
* `nvim_ui_attach`
* egui rendering callbacks
* calls from the egui UI thread into Neovim
* infinite redraw loops
* recursive event handling
* waiting for Neovim from the UI thread

### CRITICAL RULE

**The egui UI/render thread must NEVER block waiting for Neovim.**

Bad:

```text
egui
  ↓
wait for Neovim
  ↓
Neovim response
  ↓
continue rendering
```

Good:

```text
EGUI THREAD
    │
    │ non-blocking commands/events
    ▼
CHANNEL
    │
    ▼
NEOVIM RUNTIME
    │
    ▼
NEOVIM
    │
    │ async events
    ▼
CHANNEL
    │
    ▼
EGUI
```

Find the exact place where the application can block and fix the architecture.

---

# 2. DO NOT HIDE THE FREEZE

Do not add:

```rust
thread::sleep(...)
```

timeouts that merely hide the problem, polling loops, arbitrary delays, or hacks.

Do not make the UI "appear responsive" while the Neovim communication is still fundamentally blocking.

Fix the ownership and communication model properly.

---

# 3. CLEAN SOURCE DIRECTORY ARCHITECTURE

The application currently has:

```text
src/
```

Inside `src`, I want the editor implementations clearly separated.

Create/clean:

```text
src/
├── vim/
│   ├── ...
│
├── hybrid/
│   ├── ...
│
└── other application modules
```

## `src/vim/`

**EVERYTHING specifically related to Vim/Neovim belongs here.**

For example:

```text
src/vim/
├── mod.rs
├── backend.rs
├── client.rs
├── input.rs
├── events.rs
├── state.rs
├── renderer.rs
├── cursor.rs
├── highlights.rs
├── lifecycle.rs
├── protocol.rs
├── resize.rs
└── ...
```

Adapt the exact structure to the real project.

Do not create unnecessary files.

## `src/hybrid/`

**EVERYTHING specifically related to the Hybrid editor belongs here.**

For example:

```text
src/hybrid/
├── mod.rs
├── backend.rs
├── engine.rs
├── state.rs
├── input.rs
├── cursor.rs
├── selection.rs
├── rendering.rs
├── commands.rs
└── ...
```

Again, adapt to the real codebase.

---

# 4. SEARCH THE ENTIRE CODEBASE

Do not only create these folders.

Actually inspect the existing source tree.

Find every file containing:

* Vim logic
* Vim key handlers
* Vim mode state
* Vim motions
* Vim commands
* Hybrid editor logic
* Hybrid-specific state
* editor-specific rendering
* editor-specific input

Then move/refactor them into the correct ownership boundary.

Do not leave random Vim code scattered throughout unrelated files.

Do not leave Hybrid code inside Vim modules.

Do not leave old duplicate Vim implementations.

---

# 5. REMOVE THE OLD HARD-CODED VIM ENGINE

This application previously had manually implemented Vim behavior.

Remove it.

Search the entire project for hard-coded implementations of things such as:

```text
h
j
k
l
w
b
e
0
$
gg
G
f
F
t
T

dd
dw
de
d$
ciw
diw
yy
p

visual mode
operator pending
counts
registers
undo
redo
```

Do not keep these implementations alongside Neovim.

The final architecture must have:

```text
Vim semantics
      ↓
    Neovim
```

Rust should NOT recreate Vim.

---

# 6. VIM IS THE DEFAULT MODE

At application startup:

```text
EditorMode::Vim
```

must be the default.

Do not create an ambiguous state where neither editor owns the document.

The default experience must immediately be the Neovim-powered editor.

---

# 7. NEOVIM MUST BE A REAL NEOVIM EDITOR

When Vim mode is active, I expect behavior equivalent to using Neovim itself.

For example:

```text
i
a
o
Esc

h j k l

w
b
e

dd
dw
ciw
yy
p

u
Ctrl-r

v
V
Ctrl-v

/
?

:
```

and normal Neovim behavior.

Do NOT manually implement these.

Forward input correctly to Neovim.

---

# 8. RENDER NEOVIM INSIDE OUR EXISTING EDITOR

Neovim must NOT appear as:

* a terminal
* a separate window
* a floating editor
* an overlay positioned independently
* a second application UI

It must render inside:

```text
existing application
        ↓
existing editor content rectangle
        ↓
Neovim-rendered editor
```

Our existing:

* titlebar
* sidebar
* tabs
* command line
* terminal
* backlinks
* wikilinks
* preview
* documentation
* navigation
* other UI

must remain untouched.

---

# 9. FIX THE EDITOR COORDINATE SYSTEM

The current implementation has a serious visual bug where the caret can appear on the titlebar and the scrollbar appears outside the editor.

Find the actual editor content rectangle.

Use that rectangle as the origin for:

* Neovim grid
* cursor
* text
* mouse
* scrolling
* viewport
* clipping

Everything must be relative to the editor rectangle.

Never use the full application window as the Neovim editor coordinate space.

---

# 10. KEYBOARD INPUT MUST WORK

When the editor has focus:

```text
Keyboard
   ↓
egui
   ↓
Vim input translator
   ↓
Neovim
```

I must be able to:

* type
* delete
* insert
* escape
* move
* select
* search
* execute commands

The application must not swallow normal Vim keys before Neovim receives them.

---

# 11. DO NOT BLOCK EGUI

This is extremely important.

Never do synchronous/blocking Neovim operations directly inside:

```text
egui update()
render()
paint()
keyboard handler
```

if those operations can wait on Neovim.

Use the existing async/runtime architecture or introduce a clean dedicated Neovim worker.

The UI must remain responsive even if Neovim:

* starts slowly
* sends many redraw events
* encounters an error
* exits
* crashes

---

# 12. NEOVIM LIFECYCLE

Implement a robust lifecycle:

```text
Application starts
      ↓
Start Neovim
      ↓
Connect RPC
      ↓
Attach UI
      ↓
Create/load buffer
      ↓
Render
```

Shutdown:

```text
Application closes
      ↓
Tell Neovim to shut down
      ↓
Close RPC
      ↓
Terminate worker cleanly
```

If Neovim crashes or exits unexpectedly:

```text
Neovim exits
      ↓
Application receives event
      ↓
No UI freeze
      ↓
Display/record an error
```

Do not leave the application permanently blocked.

---

# 13. RAW MARKDOWN EDITOR

For the editor itself, use **raw Markdown**.

For example:

```text
# Heading

This is **bold**.

[[WikiLink]]

- Item
```

Do NOT try to reproduce the inline live Markdown transformation inside Neovim right now.

The existing beautiful Markdown preview remains separate.

Keep:

* wikilinks
* backlinks
* graph
* indexing
* search
* preview
* navigation
* metadata

working as application-level features.

---

# 14. HYBRID MODE

Hybrid remains a completely separate editor implementation.

```text
src/hybrid/
```

contains Hybrid-specific behavior.

It must not depend on Neovim.

```text
src/vim/
```

contains Vim/Neovim-specific behavior.

It must not contain Hybrid editing logic.

Shared application infrastructure may remain outside both folders.

---

# 15. DOCUMENT OWNERSHIP

When Vim mode is active:

```text
Neovim buffer = authoritative editor state
```

Do not continuously maintain a second competing editable buffer.

The application can observe changes for:

* preview
* wikilinks
* backlinks
* indexing
* dirty state

but should not fight Neovim for ownership.

When switching from Vim to Hybrid, synchronize the document once.

When switching from Hybrid to Vim, initialize the Neovim buffer once.

---

# 16. PERFORMANCE

The final implementation must be fast.

Avoid:

* full-buffer copies every frame
* blocking RPC
* unnecessary cloning
* excessive locks
* redraw storms
* infinite event loops
* unnecessary allocations
* rebuilding the entire editor for one cursor movement

Input must feel immediate.

Scrolling must feel immediate.

Cursor movement must feel immediate.

---

# 17. DO NOT TOUCH UNRELATED FEATURES

Do not rewrite:

* terminal
* sidebar
* titlebar
* backlinks
* wikilinks
* preview
* graph
* documentation
* file management
* unrelated UI

This task is about:

```text
Vim/Neovim editor
+
Hybrid separation
+
runtime correctness
+
rendering/input integration
```

---

# 18. IMPLEMENTATION ORDER

Follow this exact priority:

### Step 1

Find why the application freezes.

### Step 2

Fix the blocking/deadlock/event-loop problem.

### Step 3

Verify the application launches and remains responsive.

### Step 4

Clean:

```text
src/vim/
src/hybrid/
```

### Step 5

Remove old hard-coded Vim handlers.

### Step 6

Fix Neovim input.

### Step 7

Fix Neovim rendering inside the existing editor rectangle.

### Step 8

Fix cursor coordinates.

### Step 9

Fix scrolling.

### Step 10

Fix line height/spacing.

### Step 11

Verify normal Neovim editing behavior.

### Step 12

Only AFTER all of the above is working:

```text
cargo fmt
cargo check
cargo clippy
cargo test
```

Tests are the final validation step, not the primary task right now.

---

# 19. FINAL ACCEPTANCE TEST

I should be able to launch the application.

It should NOT freeze.

Vim mode should appear by default.

I should be able to click the editor and immediately type:

```text
# Hello World
```

Then:

```text
i
```

insert text.

Press:

```text
Esc
```

Then:

```text
dd
```

delete the line.

Then:

```text
u
```

undo.

Then:

```text
Ctrl-r
```

redo.

Then:

```text
ciw
```

change a word.

Then:

```text
yy
p
```

copy/paste through Neovim's registers.

All of this must be handled by **actual Neovim**, not a Rust implementation pretending to be Vim.

The editor must visually remain inside our existing editor area.

The caret must be inside the editor.

The scrollbar must belong to the editor.

Typing must work.

Deleting must work.

The application must remain responsive.

---

# FINAL PRINCIPLE

We are NOT building another Vim implementation.

We are embedding **real Neovim** into our existing application.

```text
              OUR APPLICATION
                    │
        ┌───────────┴───────────┐
        │                       │
      HYBRID                   VIM
        │                       │
 src/hybrid/              src/vim/
                                │
                                ▼
                             Neovim
```

**Neovim is the brain.**

**Our existing application is the body and UI.**

Fix the runtime freeze first. Then fix the editor integration. Then clean the architecture. Then validate.

Do not rush.
