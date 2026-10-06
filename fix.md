# MindForge — Final Deep Cleanup Audit

You have already removed Hybrid mode and the old SQLite architecture.

Do NOT repeat the migration blindly.

Now perform a FINAL repository-wide cleanup and audit.

The goal is to find anything that was missed, became dead, duplicated, misleading, or unnecessary after those removals.

## 1. Search Everything

Search the ENTIRE repository, not only `app/src`.

Look for:

- db
- database
- sqlite
- rusqlite
- db_worker
- Hybrid
- hybrid
- editor_mode
- mode switching
- legacy editor
- old editor abstractions
- migration code
- compatibility code
- dead modules
- unused files
- TODO/FIXME leftovers
- commented-out implementations

Do not assume something is obsolete just because its name looks old. Inspect each result and determine whether it is still needed.

## 2. Database Cleanup

We no longer use SQLite.

Find every remaining database-related file, module, name, function, comment, dependency, and abstraction.

If `db_worker` is now only handling `.mindforge/settings.json` or other JSON persistence, it should NOT continue to be called a database worker.

Refactor/rename it to accurately represent its real responsibility.

Do not delete valid persistence functionality.

The final architecture must clearly distinguish:

Filesystem → user files/source of truth

`.mindforge` → application metadata/settings/session state

No database/document-storage layer.

## 3. Hybrid Cleanup

Hybrid has been removed.

Verify that there are no remaining:

- Hybrid modules
- Hybrid state
- editor-mode enums
- mode switching
- Hybrid settings
- Hybrid commands
- Hybrid UI
- Hybrid tests
- Hybrid compatibility code
- unused abstractions created for supporting two editors

Neovim is now the ONLY editor engine.

## 4. Dead Code

Find and remove:

- unused functions
- unused structs
- unused enums
- unused fields
- unused modules
- unused imports
- unreachable code
- obsolete helpers
- obsolete comments
- stale documentation
- compatibility wrappers
- migration leftovers

Actually remove them. Do not merely report them.

## 5. Duplicate Code

Look for duplicate implementations of:

- filesystem operations
- workspace operations
- persistence
- path handling
- tab management
- Neovim communication
- editor state
- Explorer operations
- menu handling
- configuration
- error handling

If two pieces of code perform essentially the same responsibility, determine the correct implementation and consolidate them.

Do not create unnecessary abstractions just to make code look DRY.

## 6. Misleading Names

Look for names that still describe the old architecture.

Examples:

- `db_worker`
- `database`
- `hybrid`
- `editor_mode`
- `legacy`
- `migration`
- `manager` where a more precise name is appropriate

Rename code when the current name no longer reflects its responsibility.

## 7. God Files

Identify genuinely oversized modules with multiple unrelated responsibilities.

Only split them when there is a clear architectural benefit.

Do not turn one large file into dozens of tiny files.

Each module should have a clear responsibility.

## 8. Dependencies

Inspect `Cargo.toml` and workspace dependencies.

Remove dependencies that are no longer required after:

- SQLite removal
- Hybrid removal
- old inline editor removal

Then verify the dependency tree is clean.

## 9. Final Repository Search

After cleanup, run repository-wide searches again for:

`db`
`database`
`sqlite`
`rusqlite`
`Hybrid`
`hybrid`
`editor_mode`
`legacy`
`migration`

Review every remaining occurrence.

Some legitimate occurrences may remain, but every one must have a valid reason.

## 10. Verification

Run:

- cargo fmt
- cargo check --workspace
- cargo test --workspace

Then launch the actual application.

Verify:

- workspace restoration
- Explorer
- file opening
- Neovim editing
- undo/redo
- LSP
- Markdown
- tabs
- menus
- persistence
- application shutdown/startup

Do not stop at compilation.

## Important

Do not perform another massive rewrite.

Do not add features.

Do not change working behavior unnecessarily.

This is a forensic cleanup pass:

FIND → INSPECT → REMOVE/REFACTOR → VERIFY.

The final codebase should look like these old systems never existed.
