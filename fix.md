# MindForge — Focused Fixes

Fix these issues only. Do not perform a large architectural rewrite.

### 1. Ctrl+Shift+N — New Project Window

`Ctrl+Shift+N` must:

- open a completely new MindForge window;
- start with **no project/workspace loaded**;
- show the Welcome page so I can choose/drag a new project;
- keep the existing project/window completely untouched;
- NOT appear directly on top of the existing window.

Position the new window slightly offset/down from the existing window, like a professional multi-window IDE.

---

### 2. Smart Context Menu

The Explorer context menu must intelligently position itself.

If I right-click a file near the bottom of the Explorer, the menu must NOT open underneath the sidebar footer or outside the visible area.

The menu must calculate available space and automatically reposition:

```text
normal → open below/right

near bottom → open above

near right edge → shift left

near corner → adjust both
```

The menu must always remain fully visible.

Keep the menu corners at approximately **2px radius** — sharp, professional, not rounded-card style.

---

### 3. Markdown Highlighting

Neovim Markdown highlighting currently does not look as good as the Hybrid/editor styling.

Improve Neovim Markdown highlighting so Markdown has the same polished MindForge visual language as Hybrid.

Keep the existing Markdown semantics.

---

### 4. New Files Must Stay Inside the Current Project

I dragged a project into MindForge successfully.

When I create:

```text
testpython.py
```

it must be created inside the **currently opened project/workspace**, not somewhere else.

Example:

```text
B:\Projects\MyProject\
    testpython.py
```

The active workspace root must always be the default parent for root-level new files.

Never silently save a newly created file to another directory.

---

### 5. Hybrid Ctrl+Z

`Ctrl+Z` currently does not undo correctly in Hybrid.

Fix undo so:

```text
type text
Ctrl+Z
```

actually restores the previous editor state.

Make sure undo is handled by the correct Hybrid editor state and is not intercepted by global shortcuts.

---

### 6. Ctrl+Enter in Hybrid

`Ctrl+Enter` should insert/move the caret to the next line correctly.

Expected:

```text
hello|
```

Press:

```text
Ctrl+Enter
```

Result:

```text
hello
|
```

Do not trigger unrelated application commands.

Make sure the shortcut reaches the active editor when Hybrid has focus.

---

### Final requirement

After fixing these, run the actual application and manually verify all six behaviors.

Do not just run `cargo check`.

Do not break:

- filesystem Explorer;
- tabs;
- workspace state;
- Neovim;
- Hybrid;
- LSP;
- Markdown;
- drag/drop;
- context menus.

Keep the implementation clean and minimal.
