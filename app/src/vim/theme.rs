//! Neovim Theme Synchronization.
//!
//! # Purpose
//! Translates MindForge's application theme palette into Neovim's internal
//! highlight definitions (`Normal`, `CursorLine`, `Visual`, `Comment`, `String`, etc.).
//!
//! # Architecture & Responsibilities
//! - Formats colors to hex and issues Lua `vim.api.nvim_set_hl` calls over RPC.
//! - Clears row layout caches so font changes or theme swaps redraw immediately.
//! - Invalidates grid row revisions to trigger full painter refresh on color switch.
//!
//! # Non-Goals & Invariants
//! - Must NOT manage egui widgets or painter state directly.
//! - Must NOT block on synchronous RPC (uses asynchronous notifications).

use super::backend::VimBackend;
use crate::ui::theme::Theme;
use rmpv::Value;

impl VimBackend {
    /// Synchronizes MindForge's theme colors and accent into Neovim's highlight definitions
    /// and resets cached text galleys for instant, zero-delay color updates.
    pub fn sync_theme(&mut self, theme: &Theme) {
        let accent_hex = crate::accent::hex_from_color(theme.accent);
        let text_hex = crate::accent::hex_from_color(theme.text);
        let bg_hex = crate::accent::hex_from_color(theme.bg);
        let muted_hex = crate::accent::hex_from_color(theme.muted);
        let surface_hex = crate::accent::hex_from_color(theme.surface());
        let hl_hex = crate::accent::hex_from_color(theme.highlight);
        let is_light = theme.is_light();

        let h1_fg = accent_hex.clone();
        let h2_fg = "#61afef";
        let h3_fg = "#98c379";
        let h4_fg = "#e06c75";
        let h5_fg = "#d19a66";
        let h6_fg = "#d19a66";

        let bold_fg = hl_hex.clone();
        let italic_fg = if is_light { "#0284c7" } else { "#61afef" };
        let code_fg = "#d19a66"; // amber, exactly matching Hybrid syntax
        let wikilink_fg = "#c678dd"; // purple, exactly matching Hybrid syntax
        let stmt_fg = if is_light { "#9333ea" } else { "#c678dd" };
        let ident_fg = if is_light { "#0284c7" } else { "#61afef" };
        let str_fg = if is_light { "#16a34a" } else { "#98c379" };
        let url_fg = if is_light { "#0284c7" } else { "#56b6c2" };
        let type_fg = if is_light { "#ea580c" } else { "#e5c07b" };

        let lua_code = format!(
            r##"
            vim.opt.termguicolors = true
            vim.opt.background = "{bg_mode}"
            local hls = {{
                Normal = {{ fg = "{text}", bg = "{bg}" }},
                NormalNC = {{ fg = "{text}", bg = "{bg}" }},
                CursorLine = {{ bg = "{surface}" }},
                Visual = {{ bg = "{hl}" }},
                Search = {{ fg = "{bg}", bg = "{accent}" }},
                CurSearch = {{ fg = "{bg}", bg = "{accent}", bold = true }},
                Title = {{ fg = "{h1_fg}", bold = true }},
                markdownH1 = {{ fg = "{h1_fg}", bold = true }},
                markdownH2 = {{ fg = "{h2_fg}", bold = true }},
                markdownH3 = {{ fg = "{h3_fg}", bold = true }},
                markdownH4 = {{ fg = "{h4_fg}", bold = true }},
                markdownH5 = {{ fg = "{h5_fg}", bold = true }},
                markdownH6 = {{ fg = "{h6_fg}", bold = true }},
                htmlH1 = {{ fg = "{h1_fg}", bold = true }},
                htmlH2 = {{ fg = "{h2_fg}", bold = true }},
                htmlH3 = {{ fg = "{h3_fg}", bold = true }},
                htmlH4 = {{ fg = "{h4_fg}", bold = true }},
                htmlH5 = {{ fg = "{h5_fg}", bold = true }},
                htmlH6 = {{ fg = "{h6_fg}", bold = true }},
                markdownHeadingDelimiter = {{ fg = "{accent}", bold = true }},
                markdownH1Delimiter = {{ fg = "{h1_fg}", bold = true }},
                markdownH2Delimiter = {{ fg = "{h2_fg}", bold = true }},
                markdownH3Delimiter = {{ fg = "{h3_fg}", bold = true }},
                markdownH4Delimiter = {{ fg = "{h4_fg}", bold = true }},
                markdownH5Delimiter = {{ fg = "{h5_fg}", bold = true }},
                markdownH6Delimiter = {{ fg = "{h6_fg}", bold = true }},
                markdownHeadingRule = {{ fg = "{muted}" }},
                markdownBold = {{ fg = "{bold_fg}", bold = true }},
                htmlBold = {{ fg = "{bold_fg}", bold = true }},
                markdownItalic = {{ fg = "{italic_fg}", italic = true }},
                htmlItalic = {{ fg = "{italic_fg}", italic = true }},
                markdownBoldItalic = {{ fg = "{bold_fg}", bold = true, italic = true }},
                htmlBoldItalic = {{ fg = "{bold_fg}", bold = true, italic = true }},
                markdownCode = {{ fg = "{code_fg}", bg = "{surface}" }},
                markdownCodeBlock = {{ fg = "{code_fg}", bg = "{surface}" }},
                markdownCodeDelimiter = {{ fg = "{muted}" }},
                markdownBlockquote = {{ fg = "{muted}", italic = true }},
                markdownListMarker = {{ fg = "{accent}", bold = true }},
                markdownOrderedListMarker = {{ fg = "{accent}" }},
                markdownRule = {{ fg = "{muted}" }},
                markdownUrl = {{ fg = "{url_fg}", underline = true }},
                markdownLinkText = {{ fg = "{h2_fg}", underline = true }},
                markdownLink = {{ fg = "{muted}" }},
                markdownWikiLink = {{ fg = "{wikilink_fg}" }},
                ["@markup.link.wikilink"] = {{ fg = "{wikilink_fg}" }},
                markdownId = {{ fg = "{accent}" }},
                markdownIdDeclaration = {{ fg = "{accent}" }},
                markdownAutomaticLink = {{ fg = "{url_fg}", underline = true }},
                ["@markup.heading"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.1"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.2"] = {{ fg = "{h2_fg}", bold = true }},
                ["@markup.heading.3"] = {{ fg = "{h3_fg}", bold = true }},
                ["@markup.heading.4"] = {{ fg = "{h4_fg}", bold = true }},
                ["@markup.heading.5"] = {{ fg = "{h5_fg}", bold = true }},
                ["@markup.heading.6"] = {{ fg = "{h6_fg}", bold = true }},
                ["@markup.heading.1.markdown"] = {{ fg = "{h1_fg}", bold = true }},
                ["@markup.heading.2.markdown"] = {{ fg = "{h2_fg}", bold = true }},
                ["@markup.heading.3.markdown"] = {{ fg = "{h3_fg}", bold = true }},
                ["@markup.heading.4.markdown"] = {{ fg = "{h4_fg}", bold = true }},
                ["@markup.heading.5.markdown"] = {{ fg = "{h5_fg}", bold = true }},
                ["@markup.heading.6.markdown"] = {{ fg = "{h6_fg}", bold = true }},
                ["@markup.strong"] = {{ fg = "{bold_fg}", bold = true }},
                ["@markup.italic"] = {{ fg = "{italic_fg}", italic = true }},
                ["@markup.raw"] = {{ fg = "{code_fg}", bg = "{surface}" }},
                ["@markup.raw.block"] = {{ fg = "{code_fg}", bg = "{surface}" }},
                ["@markup.quote"] = {{ fg = "{muted}", italic = true }},
                ["@markup.list"] = {{ fg = "{accent}", bold = true }},
                ["@markup.list.checked"] = {{ fg = "{muted}" }},
                ["@markup.list.unchecked"] = {{ fg = "{accent}", bold = true }},
                ["@markup.link.url"] = {{ fg = "{url_fg}", underline = true }},
                ["@markup.link.label"] = {{ fg = "{h2_fg}", underline = true }},
                Comment = {{ fg = "{muted}", italic = true }},
                Statement = {{ fg = "{stmt_fg}" }},
                Identifier = {{ fg = "{ident_fg}" }},
                Type = {{ fg = "{type_fg}" }},
                Special = {{ fg = "{accent}" }},
                String = {{ fg = "{str_fg}" }},
                SnippetTabstop = {{ underline = true, sp = "{accent}" }},
                SnippetActiveTabstop = {{ underline = true, bold = true, sp = "{accent}" }},
            }}
            for name, opts in pairs(hls) do
                pcall(vim.api.nvim_set_hl, 0, name, opts)
            end
            pcall(vim.cmd, "redraw!")
            "##,
            bg_mode = if is_light { "light" } else { "dark" },
            text = text_hex,
            bg = bg_hex,
            surface = surface_hex,
            hl = hl_hex,
            accent = accent_hex,
            muted = muted_hex,
            h1_fg = h1_fg,
            h2_fg = h2_fg,
            h3_fg = h3_fg,
            h4_fg = h4_fg,
            h5_fg = h5_fg,
            h6_fg = h6_fg,
            bold_fg = bold_fg,
            italic_fg = italic_fg,
            code_fg = code_fg,
            wikilink_fg = wikilink_fg,
            stmt_fg = stmt_fg,
            ident_fg = ident_fg,
            str_fg = str_fg,
            url_fg = url_fg,
            type_fg = type_fg,
        );
        let _ = self.client.notify(
            "nvim_exec_lua",
            vec![Value::from(lua_code), Value::Array(vec![])],
        );
        self.row_layouts.clear();
        for revision in &mut self.grid.row_revision {
            *revision = revision.wrapping_add(1);
        }
    }
}
