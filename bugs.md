# MindForge — Production-Grade UI Refinement and Editor Consistency

## Objective

Refine the existing MindForge UI to deliver a polished, responsive, production-grade desktop editor experience.

MindForge is built with Rust and egui. It already has an established visual identity, a file explorer, a code and Markdown editor, embedded Neovim, Hybrid editing mode, LSP integration, an AI Agent panel, Backlinks, Outline, Preview, and extensive customization settings.

The existing UI is already good. **This is a refinement project, not a redesign or a rewrite.**

Your responsibility is to identify and resolve the remaining UI inconsistencies, improve responsive layouts, and make the editing experience feel cohesive across the entire application.

Inspect the existing implementation before making changes. Reuse the current architecture wherever possible.

---

## 1. Preserve MindForge's Visual Identity

Maintain the existing visual language shown in the supplied screenshots.

Preserve:

- The dark navy and charcoal color palette.
- The existing typography and monospaced editor font.
- Syntax highlighting and code readability.
- The minimalist title bar and application chrome.
- The left File Explorer and central editor layout.
- The bottom status bar and Vim mode indicator.
- The existing settings modal and customization options.
- The restrained borders, spacing, and panel separation.

Do not replace the existing design with a generic interface.

Do not turn MindForge into a VS Code clone. Familiar editor conventions are welcome, but MindForge must retain its own identity.

Avoid unnecessary gradients, decorative effects, excessive borders, and visual noise.

## 2. Responsive Editor Layout and Panel Sizing

The editor must remain comfortable to use across different window sizes and panel configurations.

Currently, opening the AI Agent, Outline, Preview, or other right-side panels can leave the central editor too narrow. Long Markdown links and code blocks become awkwardly wrapped, reducing readability.

Improve the layout so that:

- The central editor receives an appropriate share of the available space.
- The File Explorer and right sidebar can be resized smoothly.
- Each panel has a sensible minimum width.
- Opening or closing a panel does not cause unexpected jumps.
- Resizing the application recalculates panel dimensions correctly.
- Maximizing, restoring, and resizing the window do not leave panels temporarily misplaced.
- Panel widths and visibility remain consistent with the existing layout behavior.
- The application handles narrow windows gracefully without overlapping controls or clipping essential content.

Where necessary, inspect the layout calculation and resize handling instead of applying superficial CSS-like or fixed-pixel adjustments.

Avoid arbitrary hard-coded widths that only work for one screenshot.

## 3. Make the Editor Experience Consistent Across Hybrid and Neovim Modes

This is a critical requirement.

MindForge supports both Hybrid editing and embedded Neovim. These are different editing engines, but they should feel like parts of the same application.

Maintain consistency in:

- Editor background and text colors.
- Font family, font size, and line height.
- Gutter width and line-number alignment.
- Current-line highlighting.
- Cursor positioning and visibility.
- Scrolling and viewport behavior.
- Text alignment and horizontal positioning.
- Editor padding and available text area.
- Status bar and mode indicators.
- Layout transitions when switching editing modes.

Do not force both engines to use identical internal rendering implementations. Instead, establish consistent visual and interaction contracts while respecting each engine's architecture.

Neovim must remain the source of truth for Vim editing behavior. Do not recreate Neovim motions or silently substitute hard-coded Vim behavior inside the Rust UI.

Preserve the working Neovim RPC integration, Lua configuration, asynchronous redraw handling, and LSP lifecycle.

## 4. Gutter, Line Numbers, and Cursor Behavior

Inspect the gutter implementation carefully.

Ensure that:

- Line numbers remain correctly aligned with their corresponding text lines.
- The current line number is displayed correctly.
- Relative and absolute numbering behave according to the active editor configuration.
- The gutter does not overlap the text area.
- The cursor does not appear displaced from the actual editing position.
- Scrolling does not separate the gutter from the corresponding text.
- Changing font size or window dimensions does not break alignment.
- Switching between Hybrid and Neovim does not produce visible layout jumps.

For embedded Neovim, verify that the Rust UI correctly interprets and renders the redraw information it receives. Do not assume that a Lua configuration problem is responsible for every visual inconsistency.

Keep existing keyboard behavior, cursor customization, and Vim modes intact.

## 5. Refine the Settings Modal

The current settings modal already has a good foundation. Improve its visual consistency without changing its overall design.

Focus on:

- Consistent spacing between settings sections.
- Consistent alignment of labels, sliders, toggles, and selectable options.
- Clear selected and unselected states.
- Readable text and adequate contrast.
- Consistent hover, focus, and active states.
- Keyboard navigation and accessible interaction.
- Sensible behavior when the modal is displayed in a smaller window.

Preserve the existing caret customization options, including block, beam, underline, candle, fire, water, snow, neon, and rainbow styles, along with width, blinking, and animation controls.

Do not remove existing customization capabilities or change their meaning.

## 6. Improve Right Sidebar Behavior

The Preview, AI Agent, Backlinks, and Outline panels should behave as a unified sidebar system.

Verify that:

- Switching tabs does not unnecessarily recreate the entire editor.
- Opening and closing the sidebar preserves the editor's state.
- Panel resizing remains smooth.
- The selected tab is visually clear.
- Content does not become clipped or unexpectedly compressed.
- The sidebar responds correctly to window resizing.
- Long outlines, backlinks, and AI conversations remain usable.
- The central editor does not jump unexpectedly when panels change.

Avoid expensive full-layout recalculations or unnecessary redraw work when only a small portion of the UI changes.

## 7. Maintain Performance and Responsiveness

MindForge should remain responsive during normal editing, file navigation, resizing, and Neovim activity.

Inspect the existing implementation for:

- Unnecessary per-frame allocations.
- Repeated layout calculations.
- Redundant state updates.
- Excessive cloning of editor or document data.
- Blocking operations on the UI thread.
- Unnecessary reinitialization of Neovim or LSP clients.
- Excessive redraw requests or event-processing delays.

Do not optimize blindly. Measure or reproduce an issue before changing performance-sensitive code.

Preserve the existing asynchronous architecture and ensure that Neovim redraw events, input events, and UI repaint scheduling continue to work correctly.

## 8. Architecture and Code Quality

Follow the project's existing Rust and egui architecture.

- Reuse existing components and styling utilities.
- Avoid duplicating layout logic across panels.
- Keep Hybrid and Neovim responsibilities clearly separated.
- Do not introduce a second source of truth for editor state.
- Avoid broad refactors unrelated to the identified issues.
- Avoid adding dependencies unless there is a demonstrated need.
- Keep changes focused, maintainable, and testable.

Before editing, identify the relevant files and understand how layout state, editor state, resizing, and rendering interact.

If the implementation is already correct in an area, leave it alone.

## 9. Regression Protection

The UI refinement must not break existing functionality.

Explicitly protect:

- Embedded Neovim startup and `init.lua` loading.
- Vim motions and insert mode.
- Neovim redraw and RPC processing.
- LSP attachment, diagnostics, and completion.
- Hybrid editing behavior.
- File Explorer navigation.
- Markdown editing and preview.
- AI Agent and right sidebar functionality.
- Caret customization.
- Keyboard shortcuts.
- Existing window management and persistence behavior.

Do not sacrifice functional correctness for visual similarity.

## 10. Validation Requirements

After implementing changes, run the relevant tests and build checks supported by the project.

Manually or programmatically validate:

1. The normal window layout.
2. A maximized window and a restored window.
3. Resizing the File Explorer.
4. Opening and closing the right sidebar.
5. Switching between Preview, AI Agent, Backlinks, and Outline.
6. Switching between Hybrid and Neovim.
7. Line-number and gutter alignment while scrolling.
8. Cursor positioning and text input.
9. Opening the settings modal and changing caret options.
10. LSP activity and completion while the editor is open.
11. Mermaid and Markdown content where relevant.
12. Existing performance-sensitive interactions.

Use screenshots or measurements where useful to verify visible improvements rather than relying exclusively on compilation success.

Do not claim that a behavior works unless it has actually been verified.

---

## Required Workflow

Follow this sequence:

**Phase 1 — Inspect**

Understand the existing UI architecture, relevant Rust modules, editor modes, panel layout, settings components, and Neovim integration.

**Phase 2 — Diagnose**

Identify concrete UI defects and their root causes. Separate confirmed problems from assumptions. Prioritize issues that affect usability or risk breaking editor behavior.

**Phase 3 — Implement**

Make focused improvements while preserving the existing visual identity and working functionality.

**Phase 4 — Validate**

Run appropriate tests, build checks, and targeted interaction tests. Inspect the resulting layout at different window sizes.

**Phase 5 — Report**

Summarize:

- What was wrong.
- What caused each confirmed issue.
- Which files were changed.
- What improvements were implemented.
- Which tests passed or failed.
- Which issues remain unresolved.
- Any behavior that could not be verified.

## Final Acceptance Criteria

The work is complete when MindForge retains its recognizable design while offering a more consistent and responsive editing experience, properly sized panels, reliable gutter and cursor alignment, and smooth transitions between Hybrid and embedded Neovim.

**The priority is not to make the UI look different. It is to make the existing UI work and feel as polished as it looks.**

Do not perform unrelated refactors, remove features, or claim success without verification.
