I’m experiencing two issues with the Neovim editor in MindForge.

**1. Mousepad scrolling behavior**

When I place my fingers on the laptop touchpad and scroll up or down, the caret eventually reaches the top or bottom boundary of the editor and seems to get stuck there instead of behaving naturally within the viewport.

There is another related issue in Neovim Visual mode. When I press `V` to enter Visual Line mode and then scroll up or down using the touchpad, the editor starts selecting text as I scroll. I want to understand why this happens and whether mouse-wheel events, Neovim input handling, or egui's scrolling behavior are interacting incorrectly.

**2. Editor initialization and layout instability**

When I open a note, the editor does not appear fully stable immediately.

- The document's text takes a noticeable amount of time to appear.
- The line-number gutter initially becomes narrower than its normal width.
- After approximately one or two seconds, the gutter returns to its normal width.

The editor eventually looks correct, but the initial transition feels unstable and unpolished.

Since MindForge uses Neovim as its editor engine, I want to understand what happens between opening a note, loading its contents into Neovim, receiving redraw events, and laying out the editor in egui.

These are observations, not confirmed diagnoses. Please help me understand the underlying causes before proposing changes. I want to investigate the actual event flow, rendering lifecycle, and layout calculations rather than applying superficial fixes.
