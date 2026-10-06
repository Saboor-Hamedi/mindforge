# MindForge — Workspace, Explorer, Editor & LSP Stabilization

The filesystem architecture is now in place. Do NOT redesign it again.

This pass is about making the existing MindForge experience **correct, persistent, seamless, and polished**.

Do not add unrelated features.

First inspect the existing architecture and fix the actual causes rather than adding patches on top of patches.

---

## 1. Workspace lifecycle

### Open workspace → remain open

When a workspace/project is opened, MindForge must remember it.

There must be a clear way to:

- open a workspace;
- close the current workspace;
- remove it from the recent-workspace list;
- return to the welcome page.

Do NOT make the user delete the actual project from disk just to remove it from MindForge.

Separate:

```text
Remove from recent/workspace list
```

from:

```text
Delete files from disk
```

Never delete a user's filesystem project when they simply want to close/remove it from MindForge.

---

# 2. Welcome page + Recent files

When no workspace/editor is active, the welcome page should be useful.

Show approximately 3–4 recent files/projects where appropriate.

For example:

```text
Welcome to MindForge

Recent

README.md
main.py
MyProject
index.html
```

Clicking a recent item should reopen the real filesystem-backed document/workspace.

Do not create copies.

Do not store the document contents in SQLite.

---

# 3. Ctrl+Shift+N — New Window

Implement:

```text
Ctrl + Shift + N
```

as **New Window**.

It must create a completely new MindForge application window/instance.

The current window must remain open.

Expected behavior:

```text
Window A
    ↓
Ctrl+Shift+N
    ↓
Window A remains open
Window B opens
```

Each window must have independent:

- workspace state;
- tabs;
- editor state;
- sidebar state;
- Explorer state.

Do not accidentally close or replace the original instance.

Use the correct native window/process architecture already present in MindForge.

---

# 4. Status-bar language/extension dropdown

The language selector in the status bar currently changes the language correctly, but its input/dropdown UI is poor.

Fix the UI.

It should:

- open cleanly;
- have enough width;
- be readable;
- have proper spacing;
- have a search/filter input if appropriate;
- show the current language clearly;
- allow selecting a language;
- close correctly;
- not interfere with the editor;
- not look like a tiny default egui control.

Keep the existing status-bar design language.

---

# 5. Editor scrolling is broken

I currently cannot reliably scroll upward in the editor.

Investigate the complete scroll pipeline:

```text
mouse wheel
    ↓
egui input
    ↓
editor
    ↓
Hybrid / Neovim
```

Determine where upward scroll is being consumed, inverted, clamped, or lost.

Fix scrolling properly.

Test:

- scroll up;
- scroll down;
- large documents;
- Markdown;
- Python;
- HTML;
- Neovim;
- Hybrid.

Do not patch only one editor mode.

---

# 6. Sidebar state persistence

If the sidebar is open when MindForge closes:

```text
Sidebar = open
    ↓
close MindForge
    ↓
reopen MindForge
    ↓
Sidebar = open
```

Persist the sidebar state.

Likewise preserve the appropriate Explorer/sidebar state such as:

- open/closed;
- expanded folders;
- selected item;
- scroll position where practical.

Do not reset the UI to defaults every launch.

---

# 7. Welcome tab bug

Currently the welcome page can leave a tab open even though there is no editor/document behind it.

Fix this.

The welcome page must not create a fake document tab.

Correct states:

```text
Welcome page
    → no document tab
```

or:

```text
Real file
    → real editor tab
```

Never:

```text
Welcome page
    → empty/fake editor tab
```

---

# 8. Markdown highlighting must be consistent

Neovim currently provides good Markdown highlighting.

Hybrid currently looks almost completely plain.

This must be unified.

Markdown should visually behave consistently in:

```text
Hybrid
Neovim
```

Do not necessarily make both render identically internally.

Instead, establish a shared MindForge language/theme definition so the visual language is consistent.

Headings, emphasis, links, code, lists, quotes, etc. should have appropriate highlighting.

---

# 9. Wikilinks are broken

This is critical to MindForge.

Test:

```text
[[My Note]]
```

in both:

```text
Hybrid
Neovim
```

Wikilinks must be recognized by the MindForge application layer.

They must not depend entirely on Neovim's Markdown implementation.

Expected architecture:

```text
Editor
   ↓
document content
   ↓
MindForge Markdown/Wikilink layer
   ↓
[[My Note]]
   ↓
open/navigate/index/backlink behavior
```

Preserve normal Vim/Neovim editing semantics.

Do not hard-code Wikilink behavior into Vim motions.

---

# 10. Hybrid ↔ Neovim transition

The transition currently visibly jumps.

When switching:

```text
Hybrid → Neovim
```

or:

```text
Neovim → Hybrid
```

there should be no obvious visual jump.

Preserve:

- document content;
- cursor position;
- scroll position;
- line height;
- font;
- font size;
- syntax colors;
- gutter width;
- selection;
- editor rectangle;
- horizontal position.

The user should feel like they are switching **editing engines**, not switching to another completely different editor.

Avoid:

```text
Hybrid disappears
→ blank area
→ Neovim appears
→ content drops into place
```

Make the transition synchronized.

If the editor surface is already rendered by the application, reuse the same geometry and visual metrics.

---

# 11. Folder rename must close correctly

Current bug:

```text
Right-click folder
→ Rename
→ rename input appears
→ click somewhere else
→ rename input remains open
```

Fix the interaction lifecycle.

Clicking outside the rename input should commit or cancel according to the existing rename semantics.

At minimum:

```text
Escape → cancel
Enter → commit
click outside → finish/cancel safely
switch selection → finish/cancel safely
open another item → finish/cancel safely
```

Never leave a stale rename field floating in the Explorer.

---

# 12. Root-level file creation

There must be a clear way to create a file in the workspace root.

Currently there is no useful empty/root area to click.

Provide an obvious root-level creation target.

For example:

```text
WORKSPACE
MyProject

▾ src
  main.py

README.md

+ New File
```

or an appropriate empty area/action.

The user must be able to create:

```text
MyProject/
    new_file.py
```

without first creating a folder.

Do not require a fake folder or modal.

---

# 13. Sidebar empty-state / create-file bug

There is currently an issue where the sidebar says something equivalent to:

```text
Nothing available
```

when trying to create a file.

Fix the underlying state/data flow.

The empty state should be visually centered within the sidebar body.

For example:

```text
┌─────────────────────────┐
│                         │
│                         │
│      No files yet       │
│                         │
│       + New File        │
│                         │
└─────────────────────────┘
```

Not:

```text
No files available
```

stuck awkwardly near the top.

---

# 14. New file creation must replace the creation state

When the user clicks New File:

```text
New File
    ↓
inline filename input
```

After entering:

```text
hello.py
```

the creation row must be replaced by the actual file:

```text
hello.py
```

and the file must immediately become available/open in the editor.

It must NOT remain:

```text
New File
```

while the real file exists somewhere else.

The Explorer state must update atomically:

```text
filesystem creation
        ↓
Explorer refresh
        ↓
new file appears
        ↓
file opens
        ↓
tab becomes active
```

---

# 15. File and folder icons

The current icons are ugly and visually weak.

Improve the icon system.

Use a coherent MindForge icon language for:

- folder;
- open folder;
- Markdown;
- Python;
- HTML;
- CSS;
- JavaScript;
- TypeScript;
- TSX;
- Rust;
- JSON;
- generic file.

Icons should have enough visual distinction to be useful.

They should still remain professional and not become a rainbow of distracting colors.

Create an extensible mapping rather than hard-coding icons throughout the Explorer.

---

# 16. Explorer filename typography

The filenames are currently too small.

Increase them to a comfortable readable size.

Also improve:

- row height;
- indentation;
- spacing;
- icon alignment;
- hover state;
- selected state;
- active state.

The Explorer should be easy to scan visually.

---

# 17. Folder visualization

Folder hierarchy currently lacks good visual structure.

Improve:

```text
▾ src
  ▾ components
    App.tsx
    Header.tsx

  ▾ utils
    format.ts

README.md
```

Use:

- consistent indentation;
- subtle hierarchy guides where appropriate;
- clean expand/collapse indicators;
- consistent vertical rhythm.

Do not overdecorate the tree.

---

# 18. Explorer settings

The Explorer should eventually be configurable through MindForge settings.

Prepare the architecture for settings such as:

- Explorer font size;
- row height;
- icon visibility;
- indentation;
- compact/comfortable density;
- folder guides;
- file icon style.

Do not necessarily implement every setting now.

But do not hard-code the Explorer architecture so these options become difficult to add later.

---

# 19. LSP — CURRENTLY BROKEN

LSP currently does not work reliably.

Do not assume the previous LSP fixes solved everything.

Trace the complete lifecycle:

```text
file opened
    ↓
language detected
    ↓
LSP configuration
    ↓
server startup
    ↓
buffer attachment
    ↓
didOpen
    ↓
diagnostics
    ↓
completion
    ↓
hover
    ↓
definition
```

Test at minimum:

```text
Python → pyright
HTML → HTML language server
JavaScript/TypeScript → appropriate server
Rust → rust-analyzer
```

Verify:

- server starts;
- buffer attaches;
- diagnostics appear;
- completion works;
- completion popup works;
- navigation works;
- server survives tab switching;
- server does not restart unnecessarily.

Do not block the UI while starting or communicating with LSP.

---

# 20. LSP and filesystem identity

Make sure the LSP receives the **real filesystem URI/path**.

Do not give LSP:

- database IDs;
- temporary IDs;
- duplicated virtual paths;
- stale document paths.

The document identity must be:

```text
real filesystem path
        ↓
document
        ↓
Neovim buffer
        ↓
LSP
```

---

# 21. Preserve the filesystem-first architecture

Do not reintroduce SQLite.

The filesystem remains authoritative for:

- files;
- folders;
- Markdown;
- source code;
- project structure.

Application metadata can remain under:

```text
.mindforge/
```

as appropriate.

Do not create another hidden document store.

---

# 22. DO NOT FIX ONE BUG BY BREAKING ANOTHER

Before changing code, identify ownership.

For every state ask:

```text
Who owns this?
Who updates it?
Who renders it?
Who persists it?
Who is allowed to mutate it?
```

Especially for:

- Explorer selection;
- filesystem state;
- tabs;
- editor documents;
- Hybrid state;
- Neovim state;
- LSP state;
- sidebar state;
- welcome state.

Avoid duplicate sources of truth.

---

# 23. REAL APPLICATION TESTING

Do not stop at:

```text
cargo check
cargo test
```

Those are necessary but insufficient.

Run MindForge and manually test:

### Workspace

- open workspace;
- close workspace;
- reopen workspace;
- remove workspace from recent;
- reopen application.

### Windows

- Ctrl+Shift+N;
- verify second window;
- verify first window remains open;
- switch between windows.

### Explorer

- create root file;
- create folder;
- create nested folder;
- rename;
- delete;
- move;
- drag/drop;
- open file;
- double-click;
- switch tabs.

### Editor

- Markdown;
- Python;
- HTML;
- JavaScript;
- TypeScript;
- JSON.

### Hybrid / Neovim

- switch both directions;
- type;
- cursor;
- scroll;
- selection;
- Markdown highlighting;
- Wikilinks;
- preserve cursor/scroll.

### LSP

- Python;
- HTML;
- TypeScript/JavaScript;
- Rust.

### Persistence

Close MindForge with:

```text
sidebar open
workspace open
folders expanded
multiple tabs open
```

Reopen it.

Verify the expected state is restored.

---

# Definition of Done

Do not report completion because the project compiles.

The pass is complete only when the running MindForge application behaves coherently:

```text
Workspace
   ↓
Explorer
   ↓
Filesystem
   ↓
Tabs
   ↓
Hybrid / Neovim
   ↓
LSP
```

Everything must point to the same real document.

No fake welcome tabs.

No broken empty-state creation.

No stale rename inputs.

No broken scrolling.

No disappearing files.

No broken LSP.

No broken Wikilinks.

No visual jump between editors.

No accidental state resets.

And:

> **A user should be able to open a project, work in it, create/move/rename files, switch editors, use LSP, close MindForge, reopen it, and continue exactly where they left off.**
