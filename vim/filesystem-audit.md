# SQLite responsibility audit

| Responsibility | Classification | Current source of truth |
| --- | --- | --- |
| Workspace file bytes and existence | FILE CONTENT / FILE PATH | Real filesystem paths; SQLite never stores workspace document bytes |
| Folder hierarchy and operations | FOLDER | Real directories enumerated and changed on disk |
| Open tabs and workspace root | METADATA | SQLite settings keep paths and UI state; startup verifies/reopens from disk |
| Knowledge note bodies | DOCUMENT STORAGE | Migrated at startup to `Documents/MindForge/Notes`; SQLite rows retain an empty body plus the backing path |
| Knowledge note topics, struggle tags, timestamps | METADATA | SQLite |
| Search and backlinks | SEARCH / GRAPH RELATIONSHIP | Derived from in-memory notes and metadata; no source workspace content cache |
| Daily activity | HISTORY / METADATA | SQLite counters |
| Security scan history | HISTORY / METADATA | SQLite result records |
| Spaced repetition cards and reviews | KNOWLEDGE RELATIONSHIP / HISTORY | SQLite; these are application learning data, not user files |
| Decisions and focus | KNOWLEDGE RELATIONSHIP / METADATA | SQLite |
| Scratch documents | FILE CONTENT | Written as real Markdown files; only paths/tab state are stored in SQLite |

At startup, legacy note bodies are written to individual files before their SQLite body column is cleared. Writes use exclusive file creation and are synced before SQLite is updated. Existing database backups made before migration may still contain the old note bodies. A migrated note whose file is missing reads as empty; note saves refuse to recreate missing files. Explorer rename, move, and delete operations update or remove the associated note metadata paths.

SQLite remains for settings, knowledge metadata, learning data, activity, and scan history. It is not deleted because those are independent app responsibilities. Open workspace files continue to work from the filesystem if metadata is lost, though saved tab/workspace preferences would need to be selected again.

## Filesystem and editor behavior

Workspace file tabs use normalized full paths as identity. Reopening an already-open path selects its existing tab. File saves and dirty-tab closes compare fingerprints and block when the file was changed or removed externally. The Explorer polls the real tree, and save conflicts protect unsaved buffers from blind reloads. Neovim receives real file paths, uses the workspace as its cwd, and replaces buffer lines on tab changes.

Explorer create, rename, delete, and move actions operate on disk. Folder deletion requires confirmation. Bulk moves preflight conflicts and descendant cycles, and roll back completed moves if a later move fails. Workspace imports now open the dropped folder or chosen file directly; they no longer copy source content into the note database.

## Known boundary

The tree poll detects external creation, deletion, and renames in the Explorer. It remaps open paths when a removed and added path have a unique matching file kind, size, and modification time. If those attributes are ambiguous, the tab remains at its previous path and saves stay blocked when that old path is gone. LSP receives the real workspace root and file paths; language-server rename notifications have not been separately verified here.
