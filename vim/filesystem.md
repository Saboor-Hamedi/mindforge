# MindForge — Filesystem-First Workspace & Folder System

We are changing MindForge's document architecture.

## Core decision

MindForge should become **filesystem-first**, like a real IDE.

The user's actual files and folders on disk are the **single source of truth**.

Do NOT duplicate source files into SQLite.

Do NOT store the canonical contents of any user-created files inside SQLite, regardless of file extension or format. This includes, but is not limited to, `.md`, `.py`, `.html`, `.js`, `.ts`, `.tsx`, `.rs`, configuration files, data files, images, assets, project files, and any current or future file types supported by MindForge.

Example:

```text
B:\Projects\MyProject\
├── README.md
├── src\
│   ├── main.py
│   ├── utils.py
│   └── components\
│       └── App.tsx
└── docs\
    └── Architecture.md
```

MindForge opens and edits those actual files.

If MindForge closes and reopens, the filesystem must contain everything necessary to restore the project.

SQLite should NOT be the source of truth for files.

Before removing SQLite, audit exactly what currently uses it. Migrate those responsibilities safely rather than deleting it blindly. SQLite may remain temporarily or permanently for **cache/index/search/metadata** if useful, but the user's files must never depend on it.

---

# 1. WORKSPACE / FOLDER SYSTEM

MindForge currently lacks a proper folder/workspace system.

Implement a real filesystem hierarchy.

The Explorer must understand:

```text
Workspace
├── Folder
│   ├── Folder
│   │   └── File
│   └── File
└── File
```

Support unlimited nested folders.

The Explorer must reflect the actual filesystem.

---

# 2. CREATE FOLDER

Allow:

- New Folder
- New File
- nested folder creation

Example:

```text
src/
  components/
    ui/
```

Creating a folder must create the **real directory on disk**.

Do not create a fake database folder.

Validate names appropriately for the operating system.

---

# 3. FOLDER EXPAND / COLLAPSE

Folders should behave like a normal IDE Explorer:

```text
▾ src
   ▾ components
      ▸ ui
   main.py

▸ docs
```

Expansion state is UI state, not file-content state.

Opening/closing folders must be fast.

Do not reload or duplicate the entire document model unnecessarily.

---

# 4. MOVE FILES

Allow dragging a file into another folder.

Example:

```text
Before:

project/
├── main.py
└── utils.py

After dragging utils.py into src/:

project/
├── main.py
└── src/
    └── utils.py
```

This must perform a real filesystem move.

Use safe filesystem operations.

After moving:

- update Explorer immediately;
- update the open tab/path;
- update the editor's document identity;
- update LSP document/workspace state if required;
- update backlinks/indexes if applicable;
- do not duplicate the file.

---

# 5. MOVE FOLDERS

Allow dragging folders.

Example:

```text
Before:

project/
├── src/
│   └── main.py
└── docs/

After moving src into docs:

project/
└── docs/
    └── src/
        └── main.py
```

The entire real directory tree must move.

Nested contents must remain intact.

---

# 6. BULK FILE MOVE

Support selecting multiple files and moving them into a folder.

Example:

```text
☑ main.py
☑ utils.py
☑ config.py
```

Drag them into:

```text
src/
```

Result:

```text
src/
├── main.py
├── utils.py
└── config.py
```

The operation must be atomic/safe as far as practical.

Handle conflicts clearly.

Do not silently overwrite existing files.

---

# 7. BULK FOLDER / NESTED DRAG

Support moving multiple selected items, including nested folders.

Example:

```text
☑ components/
☑ services/
☑ utils.py
```

Move them into another directory while preserving their internal structure.

Prevent invalid operations such as:

```text
folder A
  └── folder B

drag folder A into folder B
```

Do not allow a directory to become its own descendant.

---

# 8. RENAME

Support:

- rename file
- rename folder

Renaming must happen on the filesystem.

Update all relevant MindForge state afterward.

Open tabs must follow the renamed file.

LSP state must remain coherent.

Do not create a second copy.

---

# 9. DELETE

Support safe deletion of:

- files
- folders
- nested folders

For folders, clearly confirm destructive operations when appropriate.

Never silently delete a directory tree.

---

# 10. DRAG/DROP SAFETY

Before moving anything:

Validate:

- source exists;
- destination exists;
- destination is a directory;
- source != destination;
- destination is not inside source;
- no illegal filesystem operation;
- no conflicting destination path.

If a conflict exists, present a clear MindForge UI decision instead of silently overwriting.

---

# 11. FILESYSTEM WATCHING

MindForge must detect external filesystem changes.

For example:

```text
MindForge open
       +
another editor changes file
       ↓
MindForge detects change
```

Then update the Explorer appropriately.

Likewise:

- external file created
- external file deleted
- external file renamed
- external folder created
- external folder deleted
- external folder renamed
- external file modified

Do not blindly reload an actively edited buffer and destroy unsaved changes.

Handle conflicts safely.

---

# 12. WORKSPACE OPENING

A workspace should simply be a real directory.

For example:

```text
B:\Projects\MyProject
```

MindForge opens that directory.

It should NOT copy the project into an internal MindForge database.

When reopened later:

```text
MindForge
   ↓
Open Workspace
   ↓
B:\Projects\MyProject
```

the filesystem is read again.

---

# 13. RECENT WORKSPACES

MindForge may store:

- recent workspace paths;
- UI state;
- expanded folders;
- open tabs;
- settings.

This is application metadata.

It must NOT become a second copy of the user's files.

---

# 14. SQLITE MIGRATION

Perform an audit of every current SQLite responsibility.

Classify each usage:

```text
FILE CONTENT
FILE PATH
FOLDER
DOCUMENT STORAGE
INDEX
SEARCH CACHE
GRAPH RELATIONSHIP
METADATA
SETTINGS
HISTORY
```

The following should migrate away from SQLite as the canonical source:

- file contents
- file existence
- folder hierarchy
- file paths
- folder paths

The filesystem becomes authoritative.

SQLite may remain temporarily for useful secondary data such as:

- search index
- cached metadata
- knowledge relationships
- symbol index
- performance cache

But MindForge must still function correctly if that cache is deleted and rebuilt.

Do not perform a blind SQLite deletion.

---

# 15. CRITICAL BUG: DOCUMENT DUPLICATION

There is currently a serious editor bug:

> I write a single line, switch to another tab, then return to the original tab, and that line can appear duplicated three times.

This must be investigated as part of this migration.

Do NOT simply deduplicate the visible text.

Find the actual cause.

Investigate:

```text
filesystem
   ↓
document loading
   ↓
Neovim buffer
   ↓
MindForge editor state
   ↓
tab switching
   ↓
document restoration
```

There must be exactly **one authoritative document state**.

Do not do:

```text
filesystem → buffer
filesystem → SQLite
SQLite → buffer
buffer → SQLite
buffer → buffer
```

in multiple competing paths.

For Neovim mode:

```text
Neovim buffer = authoritative editing state
filesystem = persisted file
MindForge = UI/state around them
```

When switching tabs, do not append/reinsert the document contents.

Loading a document must replace/synchronize the target buffer exactly once.

Returning to a tab must never duplicate its contents.

---

# 16. OPEN TABS

Tabs represent files/documents, not copies of their contents.

Example:

```text
Tab A → B:\Project\main.py
Tab B → B:\Project\index.html
Tab C → B:\Project\README.md
```

Each tab has a stable document identity/path.

Switching:

```text
A → B → C → A
```

must not reload and append the document repeatedly.

---

# 17. FILE IDENTITY

Do not use filename alone as identity.

These are different:

```text
src/main.py
tests/main.py
```

Use a canonical/normalized filesystem path or another stable filesystem-backed document identity.

Handle Windows path normalization correctly.

---

# 18. LSP INTEGRATION

The new filesystem/workspace architecture must work naturally with LSP.

For example:

```text
B:\Projects\PythonApp\
        ↓
workspace root
        ↓
Pyright
        ↓
actual files
```

Do not make LSP operate against SQLite copies.

The LSP must see the real workspace and real files.

---

# 19. EXPLORER PERFORMANCE

Do not rebuild the entire tree unnecessarily when:

- expanding a folder;
- collapsing a folder;
- creating a file;
- creating a folder;
- moving one file;
- renaming one file.

Update the smallest affected part of the tree.

Large projects must remain responsive.

---

# 20. ACCEPTANCE TEST

Create this real filesystem structure:

```text
MindForgeTest/
├── README.md
├── src/
│   ├── main.py
│   └── components/
│       └── App.tsx
├── web/
│   └── index.html
└── docs/
    └── Architecture.md
```

Then verify:

- [ ] open workspace;
- [ ] expand/collapse folders;
- [ ] create folder;
- [ ] create nested folder;
- [ ] create file;
- [ ] rename file;
- [ ] rename folder;
- [ ] move one file;
- [ ] move one folder;
- [ ] move multiple files;
- [ ] move multiple folders/files;
- [ ] prevent invalid nested moves;
- [ ] delete file;
- [ ] delete folder;
- [ ] switch tabs repeatedly;
- [ ] edit a file;
- [ ] switch tabs;
- [ ] return to the file;
- [ ] verify text appears exactly once;
- [ ] close MindForge;
- [ ] reopen workspace;
- [ ] verify files are exactly as they were;
- [ ] verify LSP still sees the real workspace.

---

# FINAL ARCHITECTURAL RULE

From this point forward:

> **The filesystem is the source of truth for user files.**

MindForge provides:

```text
Filesystem
    +
Neovim
    +
LSP
    +
Knowledge/Index layer
    +
Beautiful UI
```

Do not build another hidden document database.

Do not duplicate source files.

Do not make SQLite responsible for whether a file exists.

Make MindForge behave like a serious filesystem-first IDE while preserving everything unique about MindForge.
