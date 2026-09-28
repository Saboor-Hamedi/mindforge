# CODEX — STOP TESTING, FIX THE ACTUAL VIM EDITOR FIRST

The previous implementation work is largely done, but the current Vim/Neovim editor integration is visibly broken.

**Do NOT spend this task writing tests first. Do NOT add new features. Do NOT continue architectural experimentation.**

Your priority is to make the actual application work correctly and feel native inside the existing editor.

## 1. CURRENT PROBLEMS

I am running the application in **Vim mode** and currently see these problems:

* The Neovim caret appears on/in the **titlebar** instead of inside the editor.
* The Neovim scrollbar appears **above/outside the actual editor content area**.
* I cannot type into the editor.
* The Neovim editor surface does not appear naturally inside our existing editor.
* Every line currently has an ugly visible gap/incorrect row spacing.
* The Neovim rendering looks like an editor placed on top of our application rather than being integrated into our editor surface.
* Focus and coordinate mapping are clearly wrong.

These are the highest-priority problems.

---

# 2. FIX THE ACTUAL EGUI INTEGRATION

The Neovim editor must render **inside the exact existing editor content rectangle**.

It must NOT:

* create another window
* create another floating editor
* render above the editor
* render over the titlebar
* render over the sidebar
* create an independent scrollbar outside the editor
* create a second application layout
* manipulate the global egui layout incorrectly

The architecture must be:

```text
Existing Application
│
├── Titlebar
├── Sidebar
├── Tabs
├── Other UI
│
└── Existing Editor Content Rectangle
        │
        └── Vim Backend
              │
              └── Neovim
```

Neovim is the **editing engine**.

Our existing egui application remains the **visual shell**.

---

# 3. EDITOR RECTANGLE IS THE SOURCE OF TRUTH FOR RENDERING

Find the existing code that determines the actual editor content rectangle.

Use that rectangle for:

* Neovim rendering
* cursor position
* keyboard focus
* mouse input
* scrolling
* viewport size
* resize calculations

Do NOT calculate editor coordinates from:

* the entire window
* the titlebar
* the root egui context
* arbitrary screen coordinates
* a separate Neovim window rectangle

Everything must be relative to the existing editor content area.

---

# 4. FIX THE CARET

The caret currently appears on the titlebar.

This is unacceptable.

Trace the complete coordinate pipeline:

```text
Neovim cursor
    ↓
Neovim grid coordinates
    ↓
Rust Vim state
    ↓
egui editor rectangle
    ↓
screen position
```

Correct the conversion.

The cursor must appear exactly where the corresponding character is rendered.

For example:

```text
┌──────────────────────────────────────┐
│ TITLEBAR                             │
├──────────────────────────────────────┤
│                                      │
│ # Heading                            │
│ ^                                    │
│                                      │
│ Some text                            │
│                                      │
└──────────────────────────────────────┘
```

The cursor must NEVER escape the editor rectangle.

---

# 5. FIX KEYBOARD FOCUS

Currently I cannot type.

Investigate why.

When Vim mode is active and the editor has focus:

```text
Keyboard
   ↓
egui
   ↓
Vim backend
   ↓
Neovim
```

Keyboard input must actually reach Neovim.

Do not implement Vim behavior in Rust.

Do not manually handle:

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
g
f
t
etc.
```

Those keys must be forwarded to Neovim.

---

# 6. REMOVE OLD HARD-CODED VIM LOGIC

This is extremely important.

Previously, this project manually implemented various Vim motions and keys.

The new architecture uses Neovim.

Therefore:

**SEARCH THE ENTIRE CODEBASE**, not only the new `vim/` folder.

Find and remove obsolete manually implemented Vim behavior.

Look for code implementing things such as:

```text
hjkl
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
u
Ctrl-r
visual mode
operator-pending
counts
Vim mode transitions
etc.
```

Do not keep two Vim engines.

The final architecture must be:

```text
Vim semantics
      ↓
    Neovim
```

Rust should primarily handle:

```text
input translation
communication
Neovim lifecycle
state/event handling
rendering
application integration
```

---

# 7. FIX THE SCROLLBAR

The scrollbar is currently appearing above/outside the editor.

Find where it is being generated.

The scrollbar/scroll behavior must belong to the actual editor content rectangle.

If Neovim's grid is being rendered into egui, make sure the viewport and scrolling model are correctly mapped.

Do not create a duplicate scrollbar accidentally.

Do not let Neovim's UI surface escape its parent editor area.

---

# 8. FIX LINE HEIGHT AND GAPS

Every line currently has an ugly gap.

Inspect the rendering code.

The Neovim grid row height must correspond correctly to the font metrics used by the egui editor.

Avoid:

```text
Neovim row
+
extra egui spacing
+
extra widget spacing
+
extra line height
```

The final result should look like a normal text editor:

```text
# Heading
This is text.
Another line.
Another line.
```

not:

```text
# Heading


This is text.


Another line.


Another line.
```

Do not arbitrarily remove spacing until you understand where it comes from.

Trace the actual layout calculations.

---

# 9. RAW MARKDOWN EDITOR

Important architectural direction:

For now, the main editor should be a **clean raw Markdown editor**.

Do not try to reproduce the existing inline/live Markdown transformation inside Neovim.

Typing:

```text
# Heading
```

should remain:

```text
# Heading
```

inside the editor.

The existing beautiful Markdown preview remains responsible for rendered presentation.

Keep all existing application intelligence:

* wikilinks
* backlinks
* graph
* search
* document indexing
* Markdown preview
* navigation
* metadata
* other existing features

Do not remove these.

The separation should be:

```text
RAW EDITOR
    ↓
Markdown document
    ↓
Existing application systems
    ├── Preview
    ├── Wikilinks
    ├── Backlinks
    ├── Graph
    └── Search
```

This makes Neovim integration much cleaner.

---

# 10. HYBRID MODE MUST REMAIN SEPARATE

Do not break Hybrid mode.

The architecture should remain:

```text
editor/
├── hybrid/
│   └── Hybrid editor implementation
│
└── vim/
    └── Neovim implementation
```

Hybrid must not depend on Neovim.

Vim must not contain Hybrid-specific editing logic.

Use the existing editor backend abstraction.

---

# 11. DO NOT CHANGE THE APPLICATION UI

Do NOT redesign:

* titlebar
* sidebar
* tabs
* backlinks
* wikilinks
* terminal
* command line
* documentation
* layout
* themes
* existing preview
* existing navigation

The existing application UI is already good.

Only fix the integration of the Vim editor inside the existing editor surface.

---

# 12. PERFORMANCE

Do not solve the problem by introducing a constantly rebuilding UI.

Avoid:

* full-buffer cloning every frame
* full document synchronization every frame
* blocking Neovim RPC on the egui thread
* unnecessary allocations
* unnecessary redraws
* duplicate layout calculations
* excessive locking

The editor must remain responsive.

Keyboard input should feel immediate.

Cursor movement should feel immediate.

Scrolling should feel immediate.

---

# 13. DEBUG THE REAL PROBLEM

Do not guess.

Trace:

### Rendering

```text
editor rectangle
→ viewport
→ Neovim grid
→ egui coordinates
→ cursor
```

### Input

```text
egui input
→ focus detection
→ Vim backend
→ Neovim input
```

### Scrolling

```text
editor viewport
→ Neovim viewport/grid
→ egui clipping
```

### Layout

```text
font metrics
→ row height
→ column width
→ cursor position
```

Identify the actual source of each bug and fix it properly.

---

# 14. USE THE EXISTING CODE

Before modifying anything:

Inspect the current implementation.

Do not blindly rewrite it.

Find:

* current Vim backend
* Neovim client
* renderer
* input translator
* editor view
* editor rectangle calculation
* focus handling
* scroll handling
* existing Hybrid editor
* old Vim key handlers

Then make the smallest clean changes necessary.

---

# 15. DO NOT WORK ON TESTS YET

This is important.

**Do not spend the majority of this task creating tests.**

First make the actual application work.

Priority:

```text
1. Editor appears in correct location
2. Keyboard input works
3. Cursor works
4. Scrolling works
5. Line spacing works
6. Neovim rendering is visually native
7. Old hard-coded Vim logic is removed
8. Hybrid remains functional
9. Performance is good
10. THEN tests and final cleanup
```

Tests are important, but they come after the visible integration problems are fixed.

---

# 16. ACCEPTANCE CRITERIA

Do not consider this task complete until I can launch the application and:

### Vim mode

* click inside the editor
* type text
* press `Esc`
* use `h j k l`
* use `w`, `b`, `e`
* use `dd`
* use `dw`
* use `ciw`
* use `yy`
* use `p`
* use visual mode
* use `/`
* use `:`
* use undo/redo
* use normal Vim behavior

and these operations are handled by **Neovim**, not hard-coded Rust logic.

### Visual

The editor must:

* stay inside the existing editor rectangle
* never overlap the titlebar
* never place the caret on the titlebar
* never place the scrollbar above the editor
* have correct line spacing
* have correct cursor placement
* have correct focus
* look like one integrated editor
* not look like a separate Neovim window pasted over the application

### Application

Existing:

* sidebar
* tabs
* backlinks
* wikilinks
* preview
* terminal
* command line
* documentation
* navigation
* other application features

must continue working.

---

# FINAL INSTRUCTION

**Stop polishing tests. Fix the actual running application first.**

This is a visual/integration problem.

Run the application, inspect the actual behavior, trace the rendering/input/layout pipeline, fix the underlying implementation, and verify the result.

Do not hide the symptoms with hacks.

Do not add another layer on top of the broken rendering.

Make the Neovim backend a clean, native editing engine **inside our existing editor surface**.

The goal is:

> **Neovim provides the brain. Our application provides the body and UI.**
