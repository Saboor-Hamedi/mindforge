# MindForge — Production-Grade LSP, Completion & Language Intelligence System

## Mission

MindForge has now evolved beyond a Markdown-only editor.

The existing system already has:

- Markdown editing
- Wikilinks
- Backlinks
- Knowledge graph
- SQLite-backed data
- Tabs
- Hybrid editor
- Embedded Neovim
- Vim motions
- Neovim commands
- Syntax highlighting
- File extension detection
- Status bar language detection
- Code-file editing
- LSP integration
- Working LSP completion

The next objective is to turn the current LSP functionality into a **production-grade Language Intelligence System** while preserving everything that already works.

This must NOT become a VS Code clone.

MindForge remains a knowledge-centric environment where Markdown, notes, code, Wikilinks, graph relationships, and programming all coexist.

---

# 1. CURRENT STATE

LSP is already installed and functioning.

Do NOT treat LSP as a future-only feature anymore.

We now want to build a proper system around it.

For example, commands such as:

```text
:LspInstall html
:LspList
```

should eventually become first-class MindForge commands.

The existing LSP functionality must be inspected first.

Do not replace working code blindly.

---

# 2. MOST IMPORTANT UI PROBLEM

The current completion popup works functionally, but the UI is not polished enough.

The current behavior resembles:

```text
┌──────────────────────────────────────────┐
│ html5                                     │
├──────────────────────────────────────────┤
│ html       HTML5 document skeleton       │
│ html5      HTML5 document skeleton       │
└──────────────────────────────────────────┘
                       ┌────────────────────┐
                       │ documentation     │
                       │                    │
                       │ <!DOCTYPE html>   │
                       │                    │
                       └────────────────────┘
```

It feels like raw Neovim completion UI rather than a native MindForge component.

The completion system must be redesigned visually while preserving the underlying LSP behavior.

---

# 3. MIND FORGE MUST OWN THE COMPLETION UI

Do NOT simply expose Neovim's default completion popup.

Neovim/LSP should provide the data.

MindForge should render the completion UI.

Conceptually:

```text
LSP
 │
 │ completion items
 ▼
Neovim
 │
 │ popupmenu events
 ▼
MindForge Vim backend
 │
 ▼
MindForge Completion UI
```

The UI must be rendered using the existing MindForge/egui visual system.

Do not create a second unrelated UI framework.

---

# 4. COMPLETION POPUP DESIGN

The completion popup should be:

- compact
- fast
- elegant
- keyboard-friendly
- visually integrated
- context-aware
- non-intrusive

It should feel like part of the editor.

### Requirements

The popup should have:

- rounded corners consistent with MindForge
- subtle border
- subtle shadow
- existing editor surface color
- compact row height
- proper text alignment
- syntax/language icons where useful
- completion kind indicator
- selected-row indication
- documentation preview
- scrollbar only when necessary

Do NOT use a huge accent-colored background.

The MindForge accent should be used sparingly.

For example:

```text
┌─────────────────────────────────────────────┐
│  ▸ html      HTML5 document skeleton        │
│    html5     HTML5 document skeleton        │
│    head      HTML head element              │
│    header    HTML header element            │
└─────────────────────────────────────────────┘
```

The selected item should be visually clear without becoming a giant colored rectangle.

---

# 5. COMPLETION LAYOUT

Use a two-column completion design when documentation exists:

```text
┌────────────────────────────┬────────────────────────────┐
│ completion                 │ documentation              │
│                            │                            │
│ ▸ html       HTML element  │ HTML5 document skeleton    │
│   html5      HTML5         │                            │
│   head       HTML head     │ <!DOCTYPE html>            │
│   body       HTML body     │                            │
│                            │                            │
└────────────────────────────┴────────────────────────────┘
```

But:

**Do not always show the documentation pane.**

Only show it when:

- the selected completion item has documentation
- there is enough available space
- the user has not disabled it

If documentation is unavailable:

```text
completion popup only
```

Do not create a large empty panel.

---

# 6. POPUP POSITIONING

Completion must appear relative to the actual editor cursor.

It must never appear:

- outside the editor
- over the titlebar
- over tabs
- outside the application window
- detached from the cursor

Calculate:

```text
Neovim grid position
        ↓
MindForge editor coordinates
        ↓
egui screen coordinates
```

Respect:

```text
actual_editor_rect
```

and all editor padding/gutter geometry.

If there is not enough room below the cursor, intelligently place the popup above it.

If there is not enough horizontal room, shift it left.

The popup must remain completely visible.

---

# 7. COMPLETION KEYBOARD BEHAVIOR

Completion must support standard behavior.

When completion is visible:

```text
↑ / k       previous item
↓ / j       next item
Enter       accept
Tab         next/accept where appropriate
Shift+Tab   previous item
Esc         dismiss
Ctrl+Space  trigger completion
```

Do not interfere with normal Vim behavior when completion is not active.

Neovim remains responsible for the actual completion semantics.

MindForge is responsible for presentation and routing.

---

# 8. COMPLETION TEXT

Support:

- label
- detail
- kind
- documentation
- insert text
- filter text
- additional text edits where provided

Use LSP completion item information correctly.

Do not simply display the raw completion label.

For example:

```text
html       HTML element
```

rather than:

```text
html
```

when useful metadata exists.

---

# 9. COMPLETION KIND ICONS

Provide subtle visual indicators for:

- Text
- Method
- Function
- Constructor
- Field
- Variable
- Class
- Interface
- Module
- Property
- Keyword
- Snippet
- Constant
- Enum
- Struct
- Event
- Operator
- Reference
- File

Do not use visually noisy icons.

Use the existing MindForge icon language if available.

---

# 10. DOCUMENTATION PREVIEW

When an LSP completion item contains documentation, display it in a compact documentation panel.

Support:

- plain text
- Markdown documentation
- code blocks
- basic formatting

Do not allow documentation to make the popup enormous.

Use:

```text
maximum width
maximum height
scrolling
```

and truncate intelligently.

---

# 11. COMPLETION PERFORMANCE

This is critical.

Completion must never freeze the application.

Never block the egui/UI thread waiting for:

- LSP
- Neovim
- language server
- documentation
- completion requests

The architecture must remain asynchronous.

```text
Keyboard
   ↓
Neovim
   ↓
LSP
   ↓
async response
   ↓
MindForge state
   ↓
completion popup
```

Never:

```text
UI thread
   ↓
WAIT FOR LSP
   ↓
freeze
```

---

# 12. LANGUAGE SERVER MANAGER

Create a centralized language-server management layer.

Conceptually:

```text
src/
├── language/
│   ├── language.rs
│   ├── registry.rs
│   ├── server.rs
│   ├── manager.rs
│   ├── installer.rs
│   ├── configuration.rs
│   └── capabilities.rs
│
├── lsp/
│   ├── client.rs
│   ├── protocol.rs
│   ├── completion.rs
│   ├── diagnostics.rs
│   ├── hover.rs
│   ├── definition.rs
│   ├── references.rs
│   ├── rename.rs
│   ├── formatting.rs
│   └── code_actions.rs
│
├── vim/
│   └── existing Neovim integration
│
└── hybrid/
    └── existing Hybrid editor
```

Adapt this to the existing MindForge structure.

Do not create duplicate systems.

---

# 13. LANGUAGE SERVER REGISTRY

Create a centralized registry.

Conceptually:

```rust
LanguageServerDefinition {
    id,
    display_name,
    languages,
    executable,
    arguments,
    installation_method,
    configuration,
}
```

Examples:

```text
HTML
CSS
JavaScript
TypeScript
Python
Rust
JSON
YAML
PHP
```

The registry should define:

```text
language
      ↓
server
      ↓
installation
      ↓
configuration
```

---

# 14. `:LspInstall`

Implement a MindForge command:

```text
:LspInstall <language>
```

Examples:

```text
:LspInstall html
:LspInstall css
:LspInstall typescript
:LspInstall python
:LspInstall rust
```

The command should:

1. Resolve the language.
2. Determine the configured language server.
3. Check whether it is already installed.
4. If installed, report that.
5. If not installed, install it using the configured installation mechanism.
6. Show progress.
7. Capture errors.
8. Verify installation.
9. Update the language-server registry/state.

Do NOT hard-code a single installation method for every language.

Different servers may use:

- npm
- pip
- cargo
- system package
- standalone binary
- other supported mechanism

The installation layer should abstract this.

---

# 15. `:LspList`

Implement:

```text
:LspList
```

It should show something like:

```text
Language Servers

✓ HTML          vscode-html-language-server
✓ CSS           vscode-css-language-server
✓ TypeScript    typescript-language-server
✗ Python        not installed
✓ Rust          rust-analyzer
```

Include:

- language
- server
- installed/not installed
- version when available
- executable status

Make the output readable in the existing MindForge command UI.

---

# 16. FUTURE COMMANDS

Prepare the command architecture for:

```text
:LspInstall
:LspList
:LspRemove
:LspUpdate
:LspInfo
:LspRestart
:LspLog
```

Implement only the commands that are currently required.

Do not build unnecessary features merely for completeness.

---

# 17. LANGUAGE STATUS BAR

The existing status bar language selector should now become part of the language intelligence system.

For example:

```text
Ln 24   Col 8   UTF-8   Spaces: 4   Python   ● LSP
```

The language remains clickable.

The LSP indicator should communicate:

```text
● connected
○ starting
× unavailable
```

Use subtle styling.

Do not use giant colored badges.

---

# 18. LSP STATUS

The user should be able to immediately understand whether language intelligence is available.

Examples:

```text
Python     ●
```

means:

```text
Python LSP connected
```

while:

```text
Python     ○
```

means:

```text
LSP starting
```

and:

```text
Python     ×
```

means:

```text
LSP unavailable/not installed
```

Clicking the status indicator may eventually open LSP information.

---

# 19. LSP CAPABILITIES

The architecture should support the standard language-intelligence features.

Implement incrementally.

### Phase A — Current

- server lifecycle
- completion
- syntax integration

### Phase B

- diagnostics
- hover
- signature help

### Phase C

- go-to-definition
- references
- document symbols
- workspace symbols

### Phase D

- rename
- formatting
- code actions

Do not implement everything in one enormous change if the existing architecture isn't ready.

The architecture must support the progression.

---

# 20. DIAGNOSTICS

Prepare a beautiful diagnostic system.

Future UI:

```text
┌────────────────────────────────────┐
│ 10  const x =                      │
│              ^                     │
│              Expected expression   │
└────────────────────────────────────┘
```

Support:

- errors
- warnings
- information
- hints

Use the gutter for subtle indicators.

Do not cover the editor with large red/green UI.

---

# 21. HOVER

Prepare an integrated hover popup.

Conceptually:

```text
             ┌──────────────────────────┐
             │ function foo(x: number)  │
             │                          │
             │ Returns the value...     │
             └──────────────────────────┘
                      │
                      ▼
                  identifier
```

It should use MindForge's existing popup styling.

---

# 22. GO TO DEFINITION

Future:

```text
gd
```

should eventually use LSP go-to-definition.

Do not hard-code language-specific parsing.

LSP provides the location.

MindForge navigates to the corresponding document.

This should integrate naturally with:

- tabs
- notes
- workspace
- file explorer

---

# 23. LSP + MINDFORGE KNOWLEDGE GRAPH

This is where MindForge can become something different from VS Code.

Do not merely reproduce IDE functionality.

Eventually, code entities can participate in the MindForge knowledge system.

For example:

```text
Python function
       │
       ├── definition
       ├── references
       └── related Markdown note
```

But do NOT implement this entire knowledge/LSP graph integration in the current completion task.

Prepare the interfaces.

Do not overreach.

---

# 24. MARKDOWN REMAINS SPECIAL

Markdown must continue to support:

- Wikilinks
- backlinks
- graph
- preview
- knowledge features

Do not force Markdown into the code-LSP system unnecessarily.

Markdown may eventually use Markdown LSP features, but the existing MindForge Markdown intelligence remains authoritative for MindForge-specific behavior.

---

# 25. NEOVIM REMAINS THE VIM ENGINE

Do not create:

```text
MindForge Vim engine
+
Neovim Vim engine
```

There is only:

```text
Neovim
```

for Vim semantics.

MindForge provides:

```text
UI
language management
LSP integration
workspace
knowledge system
file system
tabs
status bar
completion presentation
```

---

# 26. HYBRID EDITOR

Hybrid must also benefit from language detection and syntax highlighting.

Where possible:

```text
Language
   ↓
Syntax/highlighting layer
   ↓
Hybrid
```

Neovim:

```text
Language
   ↓
Neovim
   ↓
LSP
```

Both should produce a visually consistent MindForge experience.

---

# 27. SHARED EDITOR VISUAL SYSTEM

Do not allow:

```text
Hybrid = one style
Neovim = completely different style
LSP popup = third style
```

Instead:

```text
             MindForge Design System
                     │
       ┌─────────────┼─────────────┐
       │             │             │
    Hybrid        Neovim       LSP UI
       │             │             │
       └─────────────┴─────────────┘
                     │
              Same visual language
```

Use shared:

- colors
- typography
- spacing
- borders
- corner radius
- shadows
- accent treatment
- popup geometry
- editor metrics

---

# 28. COMPLETION VISUAL DESIGN RULES

The completion UI shown in the current implementation is too visually heavy.

Specifically improve:

### Current problems

- popup is too large
- selected row is too dominant
- documentation panel is too large
- excessive empty space
- completion popup feels detached
- typography doesn't feel fully integrated
- popup geometry does not feel native to MindForge

### Desired result

Compact:

```text
┌──────────────────────────────────┐
│ ▸ html      HTML element        │
│   html5     HTML5 document      │
│   head      HTML head element   │
│   body      HTML body element   │
└──────────────────────────────────┘
```

With optional documentation:

```text
┌──────────────────────┬──────────────────────┐
│ ▸ html               │ HTML element         │
│   html5              │                      │
│   head               │ <!DOCTYPE html>     │
│   body               │                      │
└──────────────────────┴──────────────────────┘
```

The popup should be approximately as small as the available content requires.

No arbitrary giant fixed dimensions.

---

# 29. ACCESSIBILITY

Completion must remain usable without a mouse.

Everything should be keyboard navigable.

Ensure:

- selected item is obvious
- focus state is obvious
- Escape works
- Enter works
- arrows work
- Tab behavior is predictable
- popup doesn't trap the user unexpectedly

---

# 30. PERFORMANCE

This entire system must be asynchronous.

Do not repeat the previous freeze problem.

Never:

```text
UI
 ↓
LSP request
 ↓
WAIT
 ↓
UI frozen
```

Instead:

```text
UI
 ↓
request
 ↓
continue rendering
 ↓
LSP response
 ↓
update state
 ↓
render
```

Completion should feel instantaneous when the server is responsive.

If the server is slow:

```text
editor remains fully usable
```

---

# 31. FILE AND WORKSPACE ROOT

LSP needs a proper workspace root.

Implement robust root detection using project markers where appropriate.

Examples:

```text
package.json
pyproject.toml
Cargo.toml
tsconfig.json
.git
composer.json
```

Do not assume the file's directory is always the workspace root.

The LSP manager should determine:

```text
current file
       ↓
workspace root
       ↓
language server
```

This is essential for real projects.

---

# 32. SERVER LIFECYCLE

Do not start a new language server for every file.

Prefer:

```text
Workspace
    ↓
Language Server Session
    ↓
Multiple files
```

Reuse sessions when appropriate.

Handle:

- startup
- initialization
- ready
- shutdown
- restart
- crash
- unavailable server
- reconnect

Gracefully.

---

# 33. MULTIPLE LANGUAGES

MindForge may have:

```text
project/
├── frontend/
│   ├── App.tsx
│   └── package.json
│
├── backend/
│   ├── main.py
│   └── pyproject.toml
│
└── rust/
    ├── Cargo.toml
    └── main.rs
```

The architecture must support multiple language servers/workspaces without global-state collisions.

---

# 34. ERROR HANDLING

LSP failures must never crash MindForge.

Examples:

```text
Server not installed
Server executable missing
Server failed to start
Server crashed
Invalid configuration
Timeout
Malformed response
Unsupported capability
```

Show useful information in the MindForge UI.

Do not dump raw protocol errors into the editor.

---

# 35. LOGGING / DEBUGGING

Provide a controlled LSP log.

Potential future command:

```text
:LspLog
```

The log should help diagnose:

- server startup
- initialization
- capabilities
- requests
- responses
- crashes

Do not spam production logs.

Allow appropriate debug-level logging.

---

# 36. DO NOT HARD-CODE LANGUAGE BEHAVIOR

Do not write:

```rust
if python { autocomplete_python() }
if javascript { autocomplete_javascript() }
```

LSP exists specifically to avoid that.

MindForge should communicate through the standard LSP protocol.

Language-specific knowledge belongs in the language server.

---

# 37. INSTALLER SECURITY

Do not blindly execute arbitrary installation commands from user input.

Language-server installation definitions must come from a trusted, centralized registry.

Validate:

- executable
- arguments
- package name
- installation method

Do not allow:

```text
:LspInstall <arbitrary shell command>
```

The user chooses a registered language, not a shell command.

---

# 38. TESTING

Test at least:

### HTML

```text
index.html
```

Test:

- completion
- tags
- attributes
- documentation
- syntax highlighting

### CSS

```text
styles.css
```

### JavaScript

```text
app.js
```

### TypeScript

```text
app.ts
```

### TSX

```text
App.tsx
```

### Python

```text
main.py
```

### Rust

```text
main.rs
```

Test completion and basic LSP behavior where the server is installed.

---

# 39. REGRESSION TEST

Verify all existing functionality:

```text
Markdown
Wikilinks
Backlinks
Graph
SQLite
Tabs
Explorer
Fuzzy finder
Terminal
Hybrid
Neovim
Vim motions
Command line
Status bar
File switching
```

Do not sacrifice existing MindForge functionality for LSP.

---

# 40. IMPLEMENTATION STRATEGY

Do not modify everything at once.

Work in controlled stages:

### Stage 1
Inspect current LSP implementation.

### Stage 2
Create/clean Language Server Manager.

### Stage 3
Create/clean server registry.

### Stage 4
Implement:

```text
:LspInstall
:LspList
```

### Stage 5
Clean completion data flow.

### Stage 6
Replace ugly/raw completion rendering with native MindForge completion UI.

### Stage 7
Implement documentation preview.

### Stage 8
Add LSP status to status bar.

### Stage 9
Add diagnostics architecture.

### Stage 10
Prepare hover/definition/references.

### Stage 11
Test multiple languages/workspaces.

Do not rush all features into one giant implementation.

---

# 41. MOST IMPORTANT ARCHITECTURAL RULE

Keep these responsibilities separate:

```text
                MIND FORGE
                    │
     ┌──────────────┼───────────────┐
     │              │               │
   Editor         Neovim           LSP
     │              │               │
 Rendering      Vim semantics    Language intelligence
     │              │               │
     └──────────────┼───────────────┘
                    │
              MindForge UI
```

Neovim does not become the UI.

LSP does not become the UI.

The language server does not become the editor.

MindForge remains the application.

---

# 42. FINAL PRODUCT VISION

The final experience should feel like:

```text
                    MINDFORGE
┌───────────────────────────────────────────────────────────┐
│ Tabs / Workspace / Knowledge                              │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  1 │ import React from "react"                            │
│  2 │                                                      │
│  3 │ function App() {                                     │
│  4 │   return <div>Hello</div>                            │
│  5 │ }                                                    │
│                                                           │
│                    ┌─────────────────────┐                │
│                    │ ▸ div   JSX element │                │
│                    │   div   HTML        │                │
│                    │   Dialog            │                │
│                    └─────────────────────┘                │
│                                                           │
├───────────────────────────────────────────────────────────┤
│ Ln 4  Col 15  UTF-8  Spaces: 2    TypeScript   ● LSP     │
└───────────────────────────────────────────────────────────┘
```

And for Markdown:

```text
┌───────────────────────────────────────────────────────────┐
│                                                           │
│ # My Book                                                 │
│                                                           │
│ This is my note about [[Programming]].                    │
│                                                           │
│ Wikilinks, backlinks, graph, preview...                   │
│                                                           │
├───────────────────────────────────────────────────────────┤
│ Ln 8  Col 12  Markdown                                    │
└───────────────────────────────────────────────────────────┘
```

The two worlds coexist naturally.

---

# FINAL DIRECTIVE

**Do not turn MindForge into VS Code.**

Build a better integration between:

```text
Knowledge
+
Markdown
+
Code
+
Neovim
+
LSP
+
MindForge UI
```

The current LSP already works.

Do not throw it away.

Improve the architecture around it.

Most importantly, fix the completion UI so that it looks like a **native MindForge component**, not a raw Neovim popup.

The completion system must be:

- beautiful
- compact
- fast
- keyboard-first
- responsive
- asynchronous
- context-aware
- visually consistent
- extensible

Then build the Language Server Manager around it so commands such as:

```text
:LspInstall html
:LspList
```

become reliable parts of MindForge.

**Inspect the current implementation first. Reuse working components. Make incremental changes. Do not rewrite working Neovim, Hybrid, Markdown, database, tab, or knowledge functionality.**
