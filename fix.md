# MindForge — Reusable Explorer Context Menu System

I want to replace the current small Explorer right-click menus with a proper, reusable MindForge menu system.

## 1. Create a reusable `menu` module

Create a shared menu module/folder:

```text
ui/
└── menu/
    ├── menu_container.rs
    ├── menu_item.rs
    ├── menu_separator.rs
    ├── menu_model.rs
    └── mod.rs
```

The **folder/module is called `menu`**.

The main reusable menu container is called:

```text
menu_container
```

This must be the parent/container primitive for MindForge menus so the same menu system can be reused anywhere in the application.

Conceptually:

```text
menu/
├── menu_container
├── menu_item
├── menu_separator
├── menu_model
└── mod
```

The goal is:

```text
Explorer file menu
Explorer folder menu
Tab menu
Editor menu
Sidebar menu
future menus
        ↓
   menu_container
```

Do NOT create separate visual implementations for every menu.

The `menu_container` should own the common menu behavior and appearance.

---

# 2. Make it feel like a professional IDE

Use VS Code as a **behavioral/quality reference**, not as a visual clone.

The current right-click menu is far too small.

Make it:

- comfortably wide;
- comfortably tall;
- easy to read;
- generous enough for mouse interaction;
- properly aligned;
- visually clean;
- fast to open.

Do NOT make it tiny just because the menu has only a few items.

---

# 3. NO ROUND BORDER

This is important.

I do **not** want a rounded floating-card appearance.

Avoid:

```text
╭──────────────╮
│              │
╰──────────────╯
```

Use a more professional IDE-style menu:

```text
┌──────────────────────────────┐
│ Open                         │
│ Open With                    │
│                              │
│ Rename                       │
│ Delete                       │
│                              │
│ Copy Path                    │
└──────────────────────────────┘
```

Use a subtle border/shadow only if appropriate.

**No excessive rounded corners.**

Do not turn the menu into a modern rounded card.

---

# 4. Menu geometry

Establish consistent menu tokens:

```text
Menu width
Menu item height
Horizontal padding
Icon column width
Shortcut column width
Separator spacing
Section spacing
```

All menus should use these shared values.

Do not hard-code different dimensions in individual menus.

The `menu_container` should guarantee consistent geometry.

---

# 5. File context menu

When right-clicking a FILE, show file-specific actions.

For example:

```text
Open
Open With

────────────

Rename
Delete

────────────

Copy Path
Reveal in File Explorer
```

Only include actions that actually exist in MindForge.

Do not add fake menu entries.

---

# 6. Folder context menu

When right-clicking a FOLDER, show folder-specific actions.

For example:

```text
Open
New File
New Folder

────────────

Rename
Delete

────────────

Copy Path
Reveal in File Explorer
```

Folder actions must operate on the actual filesystem.

---

# 7. File and folder menus must NOT be identical

Establish one menu system but allow different menu models.

Conceptually:

```rust
MenuTarget::File(...)
MenuTarget::Folder(...)
MenuTarget::Tab(...)
MenuTarget::Editor(...)
```

Then:

```text
MenuTarget
      ↓
Menu model
      ↓
menu_container
      ↓
MenuItems
```

The **container is shared**.

The **content/actions are contextual**.

---

# 8. Right-click positioning

The menu should open at the pointer position.

However, it must remain completely inside the application/window.

If the user right-clicks near:

- bottom edge;
- right edge;

automatically reposition the menu so it remains visible.

Do not allow it to render partially off-screen.

---

# 9. Menu interaction

Support:

- mouse hover;
- click;
- keyboard navigation where practical;
- Escape to close;
- click outside to close;
- submenu support if needed later.

Hovering an item should produce a subtle selection state.

Do not make hover colors excessively bright.

---

# 10. Icons

Use MindForge's existing icon system.

For example:

```text
Open              [icon]
Rename            [icon]
Delete            [icon]
Copy Path         [icon]
Reveal            [icon]
```

Icons must be:

- consistent;
- small;
- aligned;
- visually secondary to the text.

Do not use random emoji.

---

# 11. Keyboard shortcuts

Where an action has a shortcut, provide a dedicated shortcut column.

Example:

```text
Open                         Enter
Rename                       F2
Delete                       Delete
```

The shortcut should be visually aligned to the right.

Do not manually pad strings with spaces.

Use proper layout columns.

---

# 12. Separators

Use separators to group actions.

Example:

```text
Open
Open With

──────────────

Rename
Delete

──────────────

Copy Path
Reveal in File Explorer
```

Do not overuse separators.

---

# 13. Explorer integration

Right-clicking an Explorer file:

```text
Explorer
   ↓
right click file
   ↓
menu_container
   ↓
file menu
```

Right-clicking a folder:

```text
Explorer
   ↓
right click folder
   ↓
menu_container
   ↓
folder menu
```

Both must use the same `menu_container`.

---

# 14. Do NOT interfere with normal clicking or dragging

This is especially important because the Explorer recently had click/drag problems.

Right-click context menus must NOT break:

- normal left-click;
- double-click;
- drag-and-drop;
- multi-selection;
- file opening;
- folder expansion.

The context-menu interaction must be isolated from the existing pointer/drag state machine.

---

# 15. Do NOT start dragging on right-click

Right-click must always belong to the context-menu interaction.

Left-click + movement beyond the drag threshold belongs to drag-and-drop.

Keep these states separate.

---

# 16. Menu architecture

Prefer something conceptually similar to:

```text
ui/
├── menu/
│   ├── menu_container.rs
│   ├── menu_item.rs
│   ├── menu_separator.rs
│   ├── menu_model.rs
│   └── mod.rs
│
└── explorer/
    └── context_menu.rs
```

Adapt this to the existing MindForge architecture rather than blindly creating this exact structure.

The important principle is:

> **The `menu` module is reusable infrastructure, and `menu_container` is its primary rendering/container component.**

---

# 17. Keep menu actions separate from rendering

Do not put filesystem operations directly inside `menu_container`.

Prefer:

```text
menu_container
      ↓
MenuAction
      ↓
Explorer / Workspace command
      ↓
Filesystem operation
```

For example:

```text
MenuAction::Rename
```

should trigger the existing rename workflow.

The menu should not implement its own filesystem logic.

---

# 18. Future reuse

After this implementation, it should be easy to create a `menu_container` for:

```text
Explorer file
Explorer folder
Tab
Editor
Sidebar
Terminal
Graph
```

without creating another menu renderer.

---

# 19. Visual acceptance

The final menu should feel:

- larger than the current menu;
- readable;
- professional;
- dense but comfortable;
- sharp/clean rather than rounded;
- consistent with MindForge;
- clearly associated with the clicked file/folder.

It should NOT feel like a tiny generic popup.

It should NOT feel like a rounded web-card component.

---

# 20. Final testing

Test:

### File

```text
right-click file
→ menu opens
→ correct file actions
→ menu positioned correctly
→ click action works
→ Escape closes
→ outside click closes
```

### Folder

```text
right-click folder
→ menu opens
→ correct folder actions
→ menu positioned correctly
→ actions work
```

### Edges

Right-click near:

- top;
- bottom;
- left;
- right;
- window corner.

Menu must remain visible.

### Interaction regression

Verify:

- left click still opens/selects;
- double click still opens;
- drag still works;
- multi-select still works;
- folder expansion still works;
- file movement still works.

---

## Definition of Done

MindForge has **one reusable `menu` module** with `menu_container` as its primary reusable container.

Files and folders use the same menu infrastructure but expose context-appropriate actions.

The menu is large enough to be comfortable, properly positioned, keyboard/mouse friendly, visually polished, and **not excessively rounded**.

Most importantly:

> **Do not patch the current menu until it looks acceptable. Build a reusable `menu` module and `menu_container` primitive that the rest of MindForge can use.**
