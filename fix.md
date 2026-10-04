# MindForge — Production-Grade Autocomplete & Completion Dropdown

## SCOPE — READ THIS FIRST

The current LSP/autocomplete functionality is **already working**.

Do NOT modify or redesign:

- Neovim architecture
- Hybrid editor
- Markdown editor
- Markdown highlighting
- LSP installation
- LSP server management
- Language detection
- File naming
- Status bar
- SQLite
- Tabs
- Knowledge graph
- Wikilinks
- Backlinks
- command system

**This task is ONLY about the autocomplete/completion experience.**

The goal is to transform the current working completion popup into a **beautiful, robust, fast, native MindForge completion UI**.

Do not change the underlying LSP functionality unless a tiny change is absolutely necessary to obtain the data required by the UI.

---

# 1. CURRENT PROBLEM

Autocomplete currently works, but the UI needs substantial refinement.

The current completion experience has:

- a completion list beside the cursor
- selected completion item
- completion documentation panel

However, visually it currently feels too much like a raw editor/LSP popup.

The goal is:

> **Keep the existing completion intelligence, completely improve its presentation, geometry, interaction, and robustness.**

It should feel like it was designed specifically for MindForge.

---

# 2. DESIGN PRINCIPLE

The completion popup should feel like:

```text
              MindForge Editor
                     │
                     ▼
              ┌───────────────┐
              │ completion    │
              │               │
              │ ▸ h2          │
              │   h1          │
              │   header      │
              └───────────────┘
```

Not:

```text
Neovim popup
      +
another random documentation window
```

MindForge owns the presentation.

The LSP provides completion data.

---

# 3. COMPLETION POPUP STRUCTURE

Use a compact two-panel design only when documentation is useful.

Conceptually:

```text
┌──────────────────────────────┬──────────────────────────┐
│ completion                   │ Documentation             │
│                              │                          │
│ ▸ h2        <h2></h2>       │ <h2>                     │
│   h1        <h1></h1>       │                          │
│   header    <header>        │ </h2>                    │
│                              │                          │
└──────────────────────────────┴──────────────────────────┘
```

But documentation must be **optional**.

If the selected item has no useful documentation:

```text
┌──────────────────────────────┐
│ ▸ h2        <h2></h2>       │
│   h1        <h1></h1>       │
│   header    <header>        │
└──────────────────────────────┘
```

Do NOT display a large empty documentation panel.

---

# 4. POPUP SIZE MUST BE CONTENT-DRIVEN

Do NOT use enormous fixed dimensions.

The popup should calculate its size based on:

- number of visible items
- row height
- longest relevant label
- detail text
- documentation availability
- available editor/window space

Use sensible maximum dimensions.

For example conceptually:

```text
minimum width
maximum width
minimum height
maximum height
```

The popup should never dominate the editor.

---

# 5. COMPACT ROW DESIGN

Each completion item should be compact.

A row should contain approximately:

```text
[icon]  label                    detail
```

For example:

```text
▣  h2                         <h2></h2>
▣  h1                         <h1></h1>
▣  header                     <header></header>
```

Do not make rows unnecessarily tall.

The selected item should be clearly visible, but subtle.

---

# 6. SELECTED ITEM

This is particularly important.

The selected completion item currently has too much visual weight.

Do NOT use a huge bright accent background.

Instead use a subtle MindForge selection treatment.

For example:

```text
┌─────────────────────────────────────┐
│   h1                 <h1></h1>      │
│ ▸ h2                 <h2></h2>      │  ← selected
│   h3                 <h3></h3>      │
└─────────────────────────────────────┘
```

Possible visual treatment:

- subtle surface change
- thin accent indicator
- accent-colored icon
- accent-colored label
- very subtle background difference

The selection must be obvious without overpowering the editor.

---

# 7. ACCENT COLOR

Use the existing MindForge accent color.

However:

**Do NOT flood the popup with the accent color.**

Accent should be used for:

- selected indicator
- selected icon
- selected label where appropriate
- subtle focus details

Avoid:

```text
████████████████████
████ selected ██████
████████████████████
```

Prefer subtle visual hierarchy.

---

# 8. BORDER / SURFACE / SHADOW

The popup should visually belong to MindForge.

Use the existing MindForge design language.

It should have:

- appropriate surface/background
- subtle border
- subtle shadow
- consistent corner radius
- consistent spacing

Do not introduce arbitrary new colors.

Do not create a completely separate visual theme.

Reuse existing theme tokens wherever possible.

---

# 9. DOCUMENTATION PANEL

The documentation panel should be intelligent.

When a completion item has documentation:

```text
┌──────────────────────┬─────────────────────────┐
│ ▸ h2                 │ HTML heading level 2    │
│   h1                 │                         │
│   h3                 │ <h2>Heading</h2>        │
└──────────────────────┴─────────────────────────┘
```

Requirements:

- readable typography
- comfortable padding
- automatic wrapping
- scrolling when necessary
- reasonable maximum width
- reasonable maximum height

Do not let documentation become a giant panel.

---

# 10. DOCUMENTATION SHOULD FOLLOW THE SELECTED ITEM

When the user moves:

```text
↓
↓
↓
```

the documentation should update to the selected completion item.

It must not require the user to confirm the item first.

But avoid expensive redraw/re-render work if the selection hasn't actually changed.

---

# 11. DOCUMENTATION FORMATTING

Support LSP documentation that arrives as:

- plain text
- Markdown
- structured documentation

Render it cleanly.

For code examples, use a compact code style.

For example:

```text
HTML heading element

Example:

<h2>Section title</h2>
```

Do not display raw protocol structures.

Do not expose internal LSP JSON.

---

# 12. CURSOR-RELATIVE POSITIONING

The popup must appear naturally beside the actual editor cursor.

The coordinate path should be:

```text
LSP / Neovim cursor
        ↓
Neovim grid coordinates
        ↓
MindForge editor coordinates
        ↓
egui screen coordinates
        ↓
completion popup
```

Do NOT position the popup using global window coordinates without accounting for:

- editor origin
- gutter
- editor padding
- tabs
- titlebar
- sidebar
- scroll offset

---

# 13. BOUNDARY CLAMPING

The popup must never go outside the visible application/editor area.

If there isn't enough room below:

```text
cursor
  ↓
[not enough room]
```

place the popup above the cursor.

If there isn't enough room on the right:

```text
cursor → [window edge]
```

place the documentation panel/list on the left or reduce its width.

The algorithm should adapt automatically.

Conceptually:

```text
Preferred:
       cursor
          ↓
       popup

If insufficient bottom space:
       popup
          ↑
       cursor

If insufficient right space:
       popup ← cursor
```

---

# 14. EDITOR CLIPPING

The completion popup must respect the editor/window boundaries.

It must never appear:

- in the titlebar
- outside the application
- over unrelated application chrome
- detached from the editor
- behind important UI

Use the existing editor rectangle and egui clipping/area system correctly.

---

# 15. KEYBOARD NAVIGATION

Autocomplete must be completely usable without a mouse.

Support:

```text
↑ / k       previous item
↓ / j       next item
Enter       accept
Tab         accept / next according to existing Neovim behavior
Shift+Tab   previous
Esc         dismiss
Ctrl+Space  trigger completion
```

Do not break Vim behavior when completion is not visible.

Do not hard-code Vim semantics that Neovim already owns.

---

# 16. COMPLETION ACCEPTANCE

When the user accepts an item:

```text
Enter
```

the selected LSP completion should be inserted exactly as provided by the existing completion system.

Do not manually reconstruct completion text unless the current architecture requires it.

Respect:

- insertText
- textEdit
- insertTextFormat
- additionalTextEdits

where supported by the existing implementation.

---

# 17. FILTERING

As the user types:

```text
h
```

then:

```text
ht
```

then:

```text
html
```

the completion list should update smoothly.

Do not rebuild the entire UI unnecessarily.

The current selected item should remain stable where possible.

Do not cause visible flicker.

---

# 18. NO FLICKER

Autocomplete should feel stable.

Avoid:

```text
popup appears
popup disappears
popup appears
popup moves
documentation flashes
```

during normal typing.

State updates should be controlled.

If a new completion response arrives:

- reconcile it
- preserve selection when possible
- update only what changed

---

# 19. LOADING STATE

If completion takes noticeable time, do not freeze the editor.

The editor remains completely usable.

If necessary, show a very subtle loading indicator.

For example:

```text
┌───────────────────────────┐
│ Searching…                │
└───────────────────────────┘
```

But do NOT show a spinner for tiny requests that complete immediately.

Avoid visual noise.

---

# 20. EMPTY RESULT

If no completion exists:

Do not leave an empty giant popup.

Prefer:

```text
No suggestions
```

or simply dismiss the popup depending on existing Neovim behavior.

Do not display a large empty rectangle.

---

# 21. LARGE RESULT SETS

Some languages can return hundreds of completion items.

Do not render all of them.

Use a virtualized/limited visible list where appropriate.

For example:

```text
visible items
    ↓
10–20 rows
    ↓
scroll
```

The popup should remain fast even with a large completion result.

---

# 22. SCROLLING

If there are many completion items:

- mouse wheel should scroll
- keyboard navigation should scroll automatically
- selected item must always remain visible
- scrollbar should be subtle

Do not create a giant scrollbar.

---

# 23. MOUSE SUPPORT

Mouse interaction should work naturally:

- hover item
- highlight item
- click item
- scroll list
- click documentation area if appropriate

But keyboard interaction remains the primary workflow.

---

# 24. COMPLETION ITEM DETAILS

Display useful LSP information without clutter.

For example:

```text
h2                         <h2></h2>
```

or:

```text
map                        Array.map
```

or:

```text
useState                   React Hook
```

Use typography hierarchy:

```text
label      primary
detail     secondary
kind       subtle
```

Do not make everything the same visual weight.

---

# 25. ICONS

Use completion-kind icons where the LSP provides the information.

Examples:

```text
function
method
variable
class
interface
property
keyword
module
snippet
```

Keep them small.

Do not make icons colorful or oversized.

If MindForge already has an icon system, reuse it.

---

# 26. POPUP ANIMATION

Do NOT add a large animation.

The popup should feel instantaneous.

If an animation is used at all:

- extremely short
- subtle
- no movement of the editor
- no scaling from a tiny point
- no distracting effects

Performance and responsiveness are more important.

---

# 27. HIGH-DPI / SCALING

The popup must behave correctly at different display scaling factors.

Test:

```text
100%
125%
150%
200%
```

Ensure:

- text remains crisp
- dimensions scale correctly
- popup doesn't become enormous
- cursor alignment remains correct

---

# 28. THE POPUP MUST NOT MOVE THE DOCUMENT

Opening completion must never change:

- editor scroll
- cursor position
- text layout
- gutter
- line height

The popup is an overlay.

It must not participate in document layout.

---

# 29. COMPLETION + VIM

This is critical.

Neovim remains the source of truth for Vim behavior.

MindForge is rendering the completion UI.

Therefore:

```text
Keyboard
   ↓
Neovim
   ↓
completion state
   ↓
MindForge completion renderer
```

Do not create a second completion engine.

Do not duplicate Neovim's completion semantics.

Do not implement custom text insertion logic that conflicts with Neovim.

---

# 30. COMPLETION + HYBRID

If Hybrid also uses the same LSP completion infrastructure, the visual component should be reusable.

Conceptually:

```text
                Completion Model
                       │
              ┌────────┴────────┐
              │                 │
           Hybrid             Neovim
              │                 │
              └────────┬────────┘
                       │
                MindForge Popup
```

Do not duplicate the UI implementation.

If the current architecture only supports completion through Neovim, do not force Hybrid to support it in this task. Just keep the completion component reusable.

---

# 31. STATE MODEL

Keep completion state isolated and explicit.

Conceptually:

```rust
CompletionState {
    visible: bool,
    items: Vec<CompletionItem>,
    selected: usize,
    documentation: Option<Documentation>,
    anchor: CursorAnchor,
    loading: bool,
}
```

Adapt this to the existing architecture.

Avoid scattering popup state across unrelated editor state.

---

# 32. SELECTION STABILITY

When new completion results arrive:

If the currently selected item still exists:

```text
preserve selection
```

Otherwise:

```text
select first sensible item
```

Do not constantly jump the selection back to the first item while the user navigates.

---

# 33. DOCUMENTATION SIZE

Documentation should have sensible constraints.

For example conceptually:

```text
max documentation width
max documentation height
```

If documentation exceeds the maximum:

```text
scroll
```

Do not expand the popup indefinitely.

---

# 34. VISUAL HIERARCHY

Use a clear hierarchy:

```text
PRIMARY
completion label

SECONDARY
detail

TERTIARY
kind / metadata
```

Documentation:

```text
heading / signature
    ↓
description
    ↓
example
```

Do not make everything bright.

---

# 35. MIND FORGE DESIGN LANGUAGE

The completion popup must reuse existing MindForge:

- background/surface colors
- text colors
- muted text colors
- accent color
- border treatment
- corner radius
- shadow
- typography
- spacing

Do not invent an entirely new visual language.

The popup should look as though it has always been part of MindForge.

---

# 36. DO NOT COPY VS CODE

Use VS Code only as a reference for **interaction quality**, not visual duplication.

MindForge should have its own identity.

The result should feel:

```text
MindForge
```

not:

```text
VS Code with different colors
```

---

# 37. TEST CASES

Test autocomplete with:

### HTML

```html
<div>
  h2
</div>
```

Verify:

- `h2` completion
- tag documentation
- correct popup positioning

### JavaScript

```javascript
const myVariable =
```

### TypeScript

```typescript
const value: str
```

### Python

```python
import os
os.
```

### Rust

```rust
String::
```

Test:

- completion
- keyboard navigation
- documentation
- acceptance
- dismissal
- scrolling

---

# 38. EDGE CASES

Test completion:

- near top of editor
- near bottom
- near left edge
- near right edge
- long lines
- horizontally scrolled lines
- large documents
- many completion results
- no completion results
- missing documentation
- huge documentation
- rapid typing
- rapid Esc/Enter
- switching tabs while completion is open
- switching Hybrid/Neovim
- closing the document while popup is open

There must be no panic, freeze, stale popup, or orphaned documentation panel.

---

# 39. IMPORTANT PERFORMANCE RULE

The completion popup must never block the editor.

Never perform blocking operations on the UI thread.

Never wait synchronously for:

- LSP
- Neovim
- documentation
- completion results

The editor must remain responsive.

---

# 40. IMPLEMENTATION ORDER

Do this in order:

### Step 1
Inspect the existing completion implementation.

### Step 2
Identify the existing LSP completion data model.

### Step 3
Separate completion data/state from rendering if currently mixed together.

### Step 4
Implement/clean the MindForge completion popup.

### Step 5
Implement smart popup positioning.

### Step 6
Implement compact list styling.

### Step 7
Implement selected-item styling.

### Step 8
Implement optional documentation panel.

### Step 9
Implement keyboard/mouse interaction.

### Step 10
Implement boundary detection and flipping.

### Step 11
Test large result sets.

### Step 12
Test rapid typing and dismissal.

### Step 13
Test all supported languages.

### Step 14
Stop.

Do not expand the task into other LSP features.

---

# FINAL ACCEPTANCE CRITERIA

The task is complete when:

- [ ] Existing LSP completion still works.
- [ ] Completion appears at the correct cursor position.
- [ ] Popup stays inside the application/editor boundaries.
- [ ] Popup intelligently flips above/below the cursor.
- [ ] Popup intelligently handles left/right boundaries.
- [ ] Popup size adapts to content.
- [ ] Popup is compact.
- [ ] Selected item is clearly visible.
- [ ] Selected item is not visually overpowering.
- [ ] Accent color is used subtly.
- [ ] No giant accent background.
- [ ] Documentation appears only when useful.
- [ ] Documentation panel is appropriately sized.
- [ ] Documentation scrolls when necessary.
- [ ] Completion supports keyboard navigation.
- [ ] Enter accepts.
- [ ] Escape dismisses.
- [ ] Tab behavior remains compatible with existing Neovim behavior.
- [ ] Mouse selection works.
- [ ] Large result sets remain performant.
- [ ] Rapid typing does not cause flicker.
- [ ] No completion request blocks the UI.
- [ ] No stale completion popup remains after switching documents.
- [ ] No popup appears outside the editor/application.
- [ ] Completion does not move the document.
- [ ] Completion does not alter editor scroll.
- [ ] Completion does not alter gutter/layout.
- [ ] Completion works with the existing Neovim/LSP pipeline.
- [ ] Existing Vim behavior remains intact.
- [ ] Existing Hybrid behavior remains intact.
- [ ] Existing Markdown behavior remains intact.

---

# FINAL INSTRUCTION

**Do not rewrite the LSP.**

**Do not rewrite Neovim.**

**Do not redesign the editor.**

**Do not touch unrelated MindForge systems.**

The LSP already provides the intelligence.

Your job in this task is to make the **MindForge autocomplete experience exceptional**.

The final experience should feel like:

> The language server provides the intelligence, Neovim provides the editing semantics, and MindForge provides the beautiful completion experience.

Inspect the existing implementation first, then make focused changes only to autocomplete state, rendering, positioning, interaction, and visual polish.
