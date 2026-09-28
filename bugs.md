# Improvement Suggestions

1. Use one app-owned command bar for Vim `:`, `/`, and `?`, routing Mindforge actions internally and forwarding Neovim commands and searches to Neovim.
2. Show matching text highlights while a search query is being edited; move to the selected match on Enter and clear temporary highlights on Escape.
3. Make `showcmd` display the full pending or completed Vim sequence, including longer counts, operators, motions, and register prefixes; do not truncate commands to two characters.
4. Share gutter sizing, relative line numbers, wrapped-line markers, and text alignment between Hybrid and Vim.
5. Keep Vim note titles, colors, typography, caret, and status presentation consistent with the rest of the app.
6. Keep typing sounds consistent across Hybrid and Vim for text entry and editing keys, honoring the selected sound profile.
7. Position wikilink autocomplete beside the active insertion point in both editor modes, including scrolled and wrapped lines.
8. In Hybrid mode, make Backspace remove both characters of an auto-paired delimiter when the caret is between them.
9. Add a workspace abstraction so the editor can support ordinary folders and files without coupling UI code directly to filesystem operations.
10. Keep SQLite for settings, metadata, and rebuildable search indexes while supporting Markdown files as portable workspace content.
11. Scan workspaces and update search indexes incrementally in background tasks so large folders do not block typing or rendering.
12. Detect files changed outside Mindforge and offer clear reload, compare, or keep-my-edits choices.
13. Make file creation, rename, save, and delete recoverable with atomic saves, trash support, backups, and clear conflict handling.
14. Build a responsive, virtualized file explorer that remains usable with large folders and many open tabs.
15. Use one consistent, bundled icon set with sensible file-type mappings and a fallback icon for unknown formats.
16. Provide one fast picker for files, notes, headings, commands, recent items, and backlinks, with keyboard navigation and previews.
17. Improve Markdown authoring with wikilink, tag, heading, and snippet completion plus optional formatting and templates.
18. Make heading outline, backlinks, link navigation, and preview actions available through consistent keyboard shortcuts and commands.
19. Offer optional code-note tools such as language-aware highlighting, completion, diagnostics, and formatting without requiring users to configure Neovim plugins manually.
20. Add searchable settings, beginner-friendly editor profiles, keybinding discovery, and a health panel that explains missing tools and configuration problems.
