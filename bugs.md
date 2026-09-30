# MindForge — Fix Command Line Input Across All Surfaces

The core Hybrid editor and embedded Neovim editor are now working correctly. **Do not break or redesign that architecture.**

There is now a specific input-routing problem.

## CURRENT PROBLEM

The MindForge command line / command plate works in some parts of the application, but does not accept keyboard input in certain surfaces.

The affected areas include:

1. **Web Scan**

   * Web Scan is opened through a command such as:

     ```text
     :scan
     ```
   * It scans a URL and displays the result.
   * When I am inside the Web Scan surface, I cannot properly type into the command line.

2. **Documentation**

   * The documentation surface is implemented in `docs.rs`.
   * The command line also does not accept typing there.
   * `:` / command-line input is not behaving correctly.

3. **Vim behavior**

   * Vim input is also not working correctly in these affected surfaces.
   * Determine whether this is intentional or caused by the same input-routing/focus problem.

There may be another surface with the same problem. **Do not assume the two listed surfaces are the only ones. Search the entire application for the same input-routing pattern.**

---

# PRIMARY GOAL

Make the MindForge command line/input system behave consistently throughout the application.

The command line should not suddenly stop receiving keyboard input simply because the user navigated to:

* Web Scan
* Documentation
* another application panel
* another tab
* another non-editor surface

The existing working editor behavior must remain intact.

---

# 1. TRACE THE INPUT ROUTING FIRST

Before changing code, trace the complete keyboard path:

```text
Physical keyboard
      ↓
egui input
      ↓
global input router
      ↓
focused surface
      ↓
command line / editor / panel
```

Determine exactly where keyboard events are being consumed or discarded.

Look for:

* focus checks
* early returns
* `Event::Key`
* `Event::Text`
* command-line state
* Vim state
* active pane/tab
* modal state
* focused widget
* `consume_key`
* `request_focus`
* `has_focus`
* global keyboard handlers

Do not blindly add another keyboard handler.

---

# 2. COMMAND LINE MUST HAVE CLEAR OWNERSHIP

There should be one clear rule:

```text
If the MindForge command line is active/focused,
keyboard input belongs to the command line.
```

For example:

```text
:
```

should activate the MindForge command line.

Then:

```text
:scan
```

should be typed into the MindForge command line regardless of which normal application surface is currently visible.

Likewise:

```text
:some-command
```

must continue receiving:

* letters
* numbers
* spaces
* backspace
* delete
* arrows
* Home
* End
* Enter
* Escape

---

# 3. DO NOT CONFUSE COMMAND-LINE INPUT WITH EDITOR INPUT

There are two different concepts:

```text
MindForge command line
        ↓
Application commands

Neovim editor
        ↓
Vim editing semantics
```

Do not route every keyboard event to Neovim.

Likewise, do not allow a focused Web Scan or documentation panel to swallow keyboard input that belongs to the MindForge command line.

The routing should be explicit.

Conceptually:

```text
                 Keyboard
                    │
                    ▼
              Input Router
                    │
          ┌─────────┼─────────┐
          │         │         │
          ▼         ▼         ▼
      Command     Editor    Panel
       Line      Backend    Input
          │         │
          │         ├── Hybrid
          │         └── Neovim
          │
       Commands
```

---

# 4. WEB SCAN

Inspect the Web Scan implementation.

When Web Scan is displayed:

```text
:scan
```

the command line must still be able to receive keyboard input.

Test:

```text
:
web scan
```

and verify that the entire command can be entered naturally.

Also test:

```text
Backspace
Delete
Left
Right
Home
End
Enter
Escape
```

Do not break Web Scan's existing URL scanning and result display.

---

# 5. DOCUMENTATION / `docs.rs`

Inspect `docs.rs`.

The command line must remain usable while Documentation is active.

Test:

```text
:
```

then type a command.

Also verify that normal application navigation continues to work.

Do not assume the documentation surface should behave like a text editor.

If the documentation is not an editable document, **Vim editing mode should not hijack its keyboard input**.

The important distinction is:

```text
Documentation surface
    ↓
MindForge global/application input

Markdown editor
    ↓
Hybrid or Neovim editor input
```

---

# 6. VIM INPUT

Determine exactly where Vim mode is supposed to be active.

For an actual Markdown editor:

```text
Vim mode
   ↓
Neovim receives editor input
```

For Web Scan or Documentation:

```text
Vim mode should NOT steal input
unless that surface explicitly supports Vim editing.
```

Do not send:

```text
h
j
k
l
:
/
```

to Neovim merely because Vim is the application's default mode.

Only the focused Vim editor should route editing keys to Neovim.

---

# 7. FOCUS MANAGEMENT

Inspect how focus is handled when switching between:

* Markdown editor
* Web Scan
* Documentation
* tabs
* fuzzy finder
* command line
* terminal
* other panels

A common failure pattern is:

```text
Panel receives focus
      ↓
Panel consumes keyboard event
      ↓
Command line never sees it
```

Fix the focus hierarchy rather than adding hacks for individual panels.

---

# 8. COMMAND LINE SHOULD BE GLOBAL

The command line should not depend on the Markdown editor being mounted.

This must work:

```text
Open MindForge
    ↓
Documentation
    ↓
:
    ↓
type command
```

And:

```text
Open MindForge
    ↓
Web Scan
    ↓
:
    ↓
type command
```

And:

```text
Open MindForge
    ↓
Markdown editor
    ↓
:
    ↓
type command
```

The behavior should be consistent.

---

# 9. DO NOT BREAK NEOVIM

The existing Neovim integration is now working.

Do not modify:

* Neovim initialization
* Neovim RPC architecture
* Neovim buffer ownership
* Neovim rendering
* Vim motions
* Vim commands
* Neovim Lua configuration

unless the input-routing investigation proves that a specific change is necessary.

Do not reintroduce the previous freeze.

---

# 10. SEARCH FOR ALL AFFECTED SURFACES

Do not fix only Web Scan and `docs.rs`.

Search the codebase for every major surface that can receive focus.

Look for:

```text
show_*
render_*
ui.*
egui::TextEdit
egui::Area
request_focus
has_focus
Event::Key
Event::Text
```

and identify panels that intercept keyboard input.

Create a clear input-routing model rather than accumulating one-off exceptions.

---

# 11. COMMAND LINE PRIORITY

When the command line is active:

```text
Command line gets priority
```

Example:

```text
:
```

Once command-line mode is active:

```text
typing
Backspace
arrows
Home
End
Enter
Escape
```

must belong to the command line.

Neovim should not receive those events unless the command line explicitly delegates them.

---

# 12. REGRESSION TEST

After fixing the issue, manually verify:

### Markdown + Hybrid

```text
Open Markdown
Ctrl+E
type
:
type command
```

### Markdown + Neovim

```text
Open Markdown
Ctrl+E
switch to Vim
type
Esc
:
type command
```

### Web Scan

```text
Open Web Scan
:
type command
```

### Documentation

```text
Open Documentation
:
type command
```

### Fuzzy Finder

```text
Ctrl+P
type search text
```

### Terminal

Verify that terminal input remains terminal input and is not accidentally routed to the global command line.

---

# 13. IMPORTANT — DO NOT CREATE DUPLICATE INPUT SYSTEMS

The goal is not:

```text
Web Scan input handler
Documentation input handler
Editor input handler
Command line input handler
Vim input handler
```

all independently competing for keyboard events.

Instead establish a predictable hierarchy:

```text
                 Input Router
                      │
          ┌───────────┴───────────┐
          │                       │
    Global command line      Focused surface
                                  │
                    ┌─────────────┼─────────────┐
                    │             │             │
                  Hybrid       Neovim       Panel
```

The focused surface should only receive input when the command line is not active.

---

# FINAL REQUIREMENT

Fix the underlying **focus/input-routing architecture**, not just the two currently visible bugs.

The final behavior should be:

> Wherever I am in MindForge, if I activate the MindForge command line, I can type normally.

And:

> If I am editing a Markdown document in Vim mode, Neovim receives the editing keys.

And:

> If I am in Web Scan, Documentation, Fuzzy Finder, Terminal, or another application surface, input goes to that surface according to its intended behavior.

Do not break the working Hybrid editor, Neovim editor, fuzzy finder, terminal, tabs, or existing commands.

**Inspect first. Identify the actual input-routing/focus conflict. Then fix it cleanly at the correct architectural level.**
