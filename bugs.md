# Improvement Suggestions

- Use the app command bar for Vim `:`, `/`, and `?` input, routing app commands internally and forwarding Neovim commands and searches to Neovim.
- Update search highlights as the query changes, move to a match on Enter, and clear preview highlights on Escape.
- Expand `showcmd` to display pending and completed multi-key Vim sequences such as `dd`, `dw`, and `gg`.
- Share gutter sizing, relative line numbers, wrapped-line markers, and text alignment between Hybrid and Vim.
- Keep Vim note titles, colors, typography, and status presentation consistent with the app UI.
- Keep typing sounds consistent across Hybrid and Vim for text entry and editing keys, while honoring the selected sound profile.
- Improve `showcmd` so it retains and displays the complete pending or completed Vim sequence instead of stopping at two characters, including longer counts, operators, motions, and register prefixes.
