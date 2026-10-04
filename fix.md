# MindForge — Code Editor Foundation, Language Detection & LSP-Ready Architecture

## Mission

Extend the existing MindForge editor so it can become a **beautiful, robust multi-language code editor**, while preserving the existing Markdown experience exactly as it works today.

This is **Phase 1**.

The goal is NOT to build the entire IDE yet.

The goal is to establish an excellent foundation for:

- Markdown documents
- Code files
- Automatic language detection
- Manual language selection
- High-quality syntax highlighting
- Hybrid editing
- Embedded Neovim editing
- Future LSP integration

### CRITICAL:

**Prepare the architecture for LSP, but DO NOT install, download, launch, connect to, or implement LSP functionality in this phase.**

LSP comes later.

---

# 1. NON-NEGOTIABLE: PROTECT EXISTING FUNCTIONALITY

MindForge already has a working Markdown editor.

Do not break it.

Do not redesign it.

Do not replace it with a generic code editor.

Do not remove or weaken:

- Markdown editing
- Markdown syntax highlighting
- Wikilinks
- Backlinks
- Graph
- Notes
- Tabs
- Explorer
- Search
- SQLite/database functionality
- Existing Hybrid editor
- Embedded Neovim
- Existing command system
- Existing application UI

The current Markdown experience is a core MindForge feature.

Treat it as the baseline that the new code functionality must integrate around.

---

# 2. THE NEW MINDSET

MindForge should become a **knowledge-centric editor that can also work with code**.

It is NOT a VS Code clone.

The architecture should conceptually become:

```text
                         MINDFORGE
                            │
             ┌──────────────┴──────────────┐
             │                             │
        KNOWLEDGE                        CODE
             │                             │
        Markdown                    File Detection
        Wikilinks                   Syntax Highlighting
        Backlinks                   Hybrid Editor
        Graph                       Neovim
        Notes                       Future LSP
             │                             │
             └──────────────┬──────────────┘
                            │
                     Shared MindForge
                         UI / Workspace
```

Markdown remains first-class.

Code becomes another first-class document type.

---

# 3. FILE TYPE / LANGUAGE DETECTION

When a file is opened, MindForge should determine its language from the file extension.

Examples:

```text
.md       → Markdown
.js       → JavaScript
.jsx      → JavaScript React
.ts       → TypeScript
.tsx      → TypeScript React
.py       → Python
.rs       → Rust
.php      → PHP
.html     → HTML
.htm      → HTML
.css      → CSS
.json     → JSON
.yaml     → YAML
.yml      → YAML
.sql      → SQL
```

The implementation must have **one centralized source of truth**.

Do not scatter extension checks throughout the codebase.

Conceptually:

```rust
FileLanguage::from_extension(path)
```

or whatever equivalent fits the existing architecture.

Do not blindly create a new abstraction if MindForge already has an appropriate file/language model.

---

# 4. UNKNOWN FILES

Unknown extensions must remain usable.

If MindForge doesn't recognize a file:

```text
Unknown extension
       ↓
Plain Text
```

Do not crash.

Do not refuse to open the file.

Do not apply an incorrect language.

The editor should gracefully fall back to plain text.

---

# 5. SYNTAX HIGHLIGHTING

This is the primary feature of Phase 1.

Code files should have **beautiful, robust, performant syntax highlighting**.

Initial language support should include at least:

- JavaScript
- JSX
- TypeScript
- TSX
- Python
- Rust
- PHP
- HTML
- CSS
- JSON
- YAML
- SQL

Use the existing MindForge highlighting/editor infrastructure wherever possible.

Do not create an entirely separate editor renderer for every language.

Conceptually:

```text
                    Editor
                      │
                  Language
                      │
          ┌───────────┼───────────┐
          │           │           │
      Markdown       Code      Plain Text
          │           │
      Existing       ├── JS
      system         ├── TS
                     ├── Python
                     ├── Rust
                     ├── PHP
                     └── ...
```

---

# 6. MARKDOWN HIGHLIGHTING MUST REMAIN INTACT

The existing Markdown highlighting is already good.

Do not replace it unnecessarily.

Do not force Markdown through a new generic syntax system if that risks regressions.

Opening:

```text
my_book.md
```

must continue to provide the existing MindForge Markdown experience.

---

# 7. CODE EDITING MUST BE REAL EDITING

This is not merely a syntax-coloring feature.

Code files must be genuinely editable.

The user should be able to:

- type
- delete
- select
- copy
- paste
- undo
- redo
- use arrows
- Home
- End
- Page Up
- Page Down
- search
- scroll
- switch tabs
- switch between Hybrid and Neovim

Do not implement advanced IDE functionality yet.

The immediate objective is:

> **A robust, responsive code editor with excellent syntax highlighting.**

---

# 8. HYBRID EDITOR

The existing Hybrid editor should open code files.

For example:

```text
main.py
```

should open as Python.

```text
App.tsx
```

should open as TypeScript React.

Do not create an unnecessary second editor architecture.

Prefer:

```text
MindForge Editor
      │
      ├── Markdown document
      │
      └── Code document
```

rather than:

```text
Markdown Editor
      +
Completely separate Code Editor
```

Reuse the existing editor infrastructure wherever technically appropriate.

---

# 9. NEOVIM

The existing embedded Neovim integration is working.

**Do not break it.**

When the user opens a code file and switches using the existing:

```text
Ctrl+E
```

Neovim should continue editing that same document.

Neovim remains responsible for Vim semantics:

- Normal mode
- Insert mode
- Visual mode
- Motions
- Operators
- Registers
- Macros
- Undo/redo
- Search
- Vim commands
- Mappings

Do NOT create another homemade Vim implementation.

Do NOT move Vim motions back into Rust.

---

# 10. HYBRID + NEOVIM MUST FEEL LIKE ONE EDITOR

Hybrid and Neovim are two editing engines inside the same MindForge editor.

Maintain the existing visual contract between them.

They should share:

- font
- font size
- line height
- gutter
- line numbers
- padding
- editor origin
- colors
- selection
- cursor geometry
- scrolling
- scrollbar
- viewport

Switching:

```text
Hybrid
   ↓
Ctrl+E
   ↓
Neovim
```

must not feel like switching to another application.

Do not regress the existing work on editor consistency.

---

# 11. STATUS BAR LANGUAGE SELECTOR

This is an important part of the new code-editor experience.

MindForge already has a status bar.

The status bar should display the current document language.

For example:

```text
┌─────────────────────────────────────────────────────────────┐
│ Ln 24   Col 8   Spaces: 4   UTF-8          Python ▼        │
└─────────────────────────────────────────────────────────────┘
```

The language indicator should be interactive.

Clicking it opens a polished language-selection dropdown.

---

# 12. LANGUAGE DROPDOWN

The dropdown should feel like a native part of MindForge.

It should NOT look like a copy of VS Code.

Example:

```text
┌──────────────────────────────┐
│ Search language...           │
├──────────────────────────────┤
│ ✓ Auto Detect                │
│                              │
│ Markdown                     │
│ JavaScript                   │
│ TypeScript                   │
│ JSX                          │
│ TSX                          │
│ Python                       │
│ Rust                         │
│ PHP                          │
│ HTML                         │
│ CSS                          │
│ JSON                         │
│ YAML                         │
│ SQL                          │
│ Plain Text                   │
└──────────────────────────────┘
```

The dropdown should support:

- mouse selection
- keyboard navigation
- search/filtering
- Enter to select
- Escape to close
- clear selected state
- sensible scrolling for a long language list

Make the interaction smooth and responsive.

---

# 13. ACCENT COLOR RULE

This is a specific visual requirement.

The selected language should use the MindForge **accent color for text/icons/indicators only**.

Do NOT use a large accent-colored background.

Do NOT create a large accent rectangle behind the selected item.

Do NOT turn the language dropdown into a visually noisy component.

Conceptually:

```text
Normal item:
Python

Selected item:
Python   ← accent-colored text/icon
```

NOT:

```text
████████████████
█ Python       █   ← NO large accent background
████████████████
```

The dropdown should use the existing MindForge surface/background styling.

The accent color should be subtle and intentional.

---

# 14. AUTO DETECTION

By default:

```text
main.py
   ↓
Auto Detect
   ↓
Python
```

The status bar should show:

```text
Python
```

The language detection should be automatic whenever the file extension is recognized.

---

# 15. MANUAL LANGUAGE OVERRIDE

The user must also be able to manually choose a language.

Example:

```text
main.py
```

normally detects:

```text
Python
```

But the user can open:

```text
Language dropdown
      ↓
JavaScript
```

and MindForge should immediately use JavaScript syntax highlighting for the current editor.

This is useful for:

- unusual files
- extensionless files
- generated files
- embedded languages
- files where automatic detection is insufficient

---

# 16. AUTO DETECT MUST BE RESTORABLE

The dropdown must contain:

```text
Auto Detect
```

If the user manually overrides a file:

```text
Python → JavaScript
```

they can return to:

```text
Auto Detect
```

and MindForge returns to the language determined by the file extension.

---

# 17. MANUAL OVERRIDE STATE

Keep the implementation clean.

Conceptually:

```text
Document
   │
   ├── path
   ├── detected_language
   └── language_override
```

Then:

```text
effective_language =
    language_override
    ?? detected_language
```

Adapt this to the existing document model rather than blindly implementing this exact structure.

The important principle is that **detected language and manually selected language are separate concepts**.

---

# 18. PERSISTENCE

If technically appropriate within the existing architecture, a manual language override should persist for the document/session rather than disappearing immediately because the editor re-renders.

However:

Do not introduce a large persistence system just for this feature.

Use the existing document/tab/session state architecture.

Do not add unnecessary database complexity.

---

# 19. MARKDOWN NOTE NAMING

For newly created Markdown notes, maintain clean naming.

Preferred:

```text
my_book.md
python_basics.md
database_design.md
api_reference.md
my_project_notes.md
```

Avoid unnecessarily messy note names such as:

```text
my book.md
My Book!!.md
hello@@world.md
my---book.md
```

Use the existing filesystem/Wikilink constraints.

Do not silently rename files.

If the user enters an invalid Markdown note name, provide clear validation or a clean suggestion.

---

# 20. DO NOT RESTRICT CODE FILENAMES

Do NOT apply the Markdown naming rules to code.

These are valid and common:

```text
package.json
vite.config.ts
index.test.ts
main.spec.ts
.env
.gitignore
```

Code files should follow normal ecosystem conventions.

---

# 21. LSP — PREPARE NOW, IMPLEMENT LATER

This is one of the most important requirements.

The architecture must be **LSP-ready**.

But LSP is NOT part of this implementation phase.

Think:

```text
                 MindForge
                     │
               File Language
                     │
          ┌──────────┴──────────┐
          │                     │
    Syntax Highlighting     Future LSP
          │                     │
        NOW                  LATER
```

Create clean extension points that will allow future integration of:

- completion
- hover
- diagnostics
- go-to-definition
- references
- rename
- code actions
- formatting

But do not implement those features now.

---

# 22. ABSOLUTELY NO LSP INSTALLATION

During this task:

**DO NOT:**

- install language servers
- download language servers
- launch language servers
- spawn language-server processes
- connect to LSP
- automatically install Node/Python/Rust/PHP tooling
- modify system package managers
- add LSP installation workflows
- add autocomplete
- add diagnostics
- add hover
- add go-to-definition
- add references
- add LSP formatting
- add LSP code actions

The final state must be:

```text
LSP architecture:
        ✓ Prepared

LSP server:
        ✗ Not installed

LSP process:
        ✗ Not running

LSP communication:
        ✗ Not active

LSP features:
        ✗ Not implemented
```

The agent must understand that **“prepare for LSP” does not mean “start implementing LSP.”**

---

# 23. FUTURE LSP BOUNDARY

Design the architecture so that a future Phase 2 can cleanly become:

```text
                  MindForge
                      │
               Language Manager
                      │
          ┌───────────┼───────────┐
          │           │           │
       Python     TypeScript     Rust
          │           │           │
         LSP         LSP         LSP
          │           │           │
       Server      Server      Server
```

But stop before implementing this.

---

# 24. PERFORMANCE

MindForge is already responsive and handles substantial SQLite data.

Do not introduce performance regressions.

Avoid:

- reparsing the entire document on every keystroke
- rebuilding the entire editor every frame
- cloning entire buffers unnecessarily
- blocking the UI thread
- unnecessary allocations
- unnecessary Neovim synchronization
- unnecessary database queries
- recreating the editor when changing language
- expensive syntax processing unrelated to the changed text

Syntax highlighting should be incremental or efficiently cached where the existing technology supports it.

---

# 25. INSPECT BEFORE MODIFYING

Before writing code, inspect the repository.

Identify:

- existing editor abstraction
- Hybrid editor
- Neovim integration
- Markdown highlighting
- syntax-highlighting implementation
- document model
- tab model
- file loading
- file-extension handling
- status bar
- theme system
- accent-color system
- editor visual configuration
- existing state management
- existing persistence mechanisms

Do not create duplicate abstractions if the project already has suitable ones.

Reuse existing architecture wherever possible.

---

# 26. IMPLEMENTATION ORDER

Follow this order.

### Step 1 — Repository inspection

Understand the current architecture.

### Step 2 — Language model

Establish or extend the centralized file-language mapping.

### Step 3 — File detection

Automatically identify the language from the extension.

### Step 4 — Syntax highlighting

Add robust code syntax highlighting while preserving Markdown.

### Step 5 — Hybrid editor

Verify code files can be edited normally.

### Step 6 — Neovim

Verify code files continue working through the existing Neovim integration.

### Step 7 — Status bar

Add the language indicator.

### Step 8 — Language dropdown

Implement:

- Auto Detect
- language list
- search
- keyboard navigation
- manual override
- Escape
- Enter

### Step 9 — Styling

Use MindForge's existing visual system.

Selected language:

**accent-colored text/icon only.**

No accent background block.

### Step 10 — LSP boundary

Ensure the architecture has a clean future extension point.

Do not activate LSP.

### Step 11 — Regression testing

Verify Markdown, Wikilinks, graph, tabs, Hybrid, Neovim, search, command line, and existing UI.

### Step 12 — STOP

Do not continue into LSP.

---

# 27. TEST MATRIX

Test these files:

```text
my_book.md
python_basics.py
app.js
main.ts
App.tsx
main.rs
index.php
index.html
styles.css
package.json
config.yaml
query.sql
unknown.xyz
```

Expected:

```text
my_book.md        → Markdown
python_basics.py → Python
app.js            → JavaScript
main.ts           → TypeScript
App.tsx           → TypeScript React
main.rs           → Rust
index.php         → PHP
index.html        → HTML
styles.css        → CSS
package.json      → JSON
config.yaml       → YAML
query.sql         → SQL
unknown.xyz       → Plain Text
```

Then test manual override.

Example:

```text
python_basics.py
       ↓
Auto Detect
       ↓
Python
       ↓
Language dropdown
       ↓
JavaScript
       ↓
JavaScript highlighting
```

Then:

```text
Language dropdown
       ↓
Auto Detect
       ↓
Python
```

---

# 28. FINAL ACCEPTANCE CRITERIA

The task is complete only when:

- [ ] Existing Markdown functionality remains intact.
- [ ] Existing Markdown syntax highlighting remains intact.
- [ ] Wikilinks remain intact.
- [ ] Backlinks remain intact.
- [ ] Graph remains intact.
- [ ] Existing tabs remain intact.
- [ ] Existing Neovim integration remains intact.
- [ ] Hybrid remains intact.
- [ ] Code files open normally.
- [ ] File extensions automatically determine language.
- [ ] Unknown extensions fall back to plain text.
- [ ] Code syntax highlighting is robust and responsive.
- [ ] JavaScript highlighting works.
- [ ] TypeScript highlighting works.
- [ ] TSX highlighting works.
- [ ] Python highlighting works.
- [ ] Rust highlighting works.
- [ ] PHP highlighting works.
- [ ] HTML highlighting works.
- [ ] CSS highlighting works.
- [ ] JSON highlighting works.
- [ ] YAML highlighting works.
- [ ] SQL highlighting works.
- [ ] Status bar displays the effective language.
- [ ] Status bar language indicator is clickable.
- [ ] Dropdown opens cleanly.
- [ ] Dropdown supports search/filtering.
- [ ] Dropdown supports keyboard navigation.
- [ ] Auto Detect is available.
- [ ] Manual language selection works.
- [ ] Manual override does not modify the filename.
- [ ] Auto Detect restores extension-based detection.
- [ ] Selected language uses accent-colored text/icon.
- [ ] Selected language does NOT receive a large accent background.
- [ ] Markdown naming remains clean.
- [ ] Code filenames remain unrestricted.
- [ ] Hybrid can edit code files.
- [ ] Neovim can edit code files.
- [ ] Ctrl+E continues working.
- [ ] Hybrid and Neovim maintain the same visual language.
- [ ] No old Vim implementation is reintroduced.
- [ ] No LSP server is installed.
- [ ] No LSP server is downloaded.
- [ ] No LSP server is launched.
- [ ] No LSP connection is established.
- [ ] No LSP features are implemented.
- [ ] Future LSP integration has a clean architectural boundary.
- [ ] No existing MindForge functionality regresses.

---

# FINAL DIRECTIVE

**Do not over-engineer this task.**

The desired result is:

```text
                  MINDFORGE
                      │
       ┌──────────────┴──────────────┐
       │                             │
    MARKDOWN                        CODE
       │                             │
   Existing system          File Extension Detection
   remains intact                    │
       │                      Syntax Highlighting
   Wikilinks                          │
   Backlinks                    Hybrid / Neovim
   Graph                              │
       │                       Status Bar Language
       │                              │
       └──────────────┬───────────────┘
                      │
              FUTURE LSP BOUNDARY
                      │
                 NOT ACTIVE
```

Build this foundation cleanly.

**Do not install LSP.**

**Do not implement LSP.**

**Prepare for LSP.**

Focus this phase on:

> **File extension → language detection → beautiful syntax highlighting → status-bar language selector → manual language override → clean Markdown naming.**

Everything that already works in MindForge must continue working exactly as before.

LSP is the next phase, not this phase.
