//! Neovim Embedded Bootstrapping & Lua Asset Management.
//!
//! # Purpose
//! Preloads MindForge's built-in LSP integration, key bindings, and editor
//! defaults directly into the embedded Neovim engine instance upon startup.
//!
//! # Architecture & Responsibilities
//! - Bundles Lua runtime scripts for LSP handlers, diagnostics, snippets, and Emmet.
//! - Configures initial Neovim options (relative line numbers, true colors, syntax on, cursorline).
//! - Normalizes indentation and visual line navigation shortcuts (`gj`/`gk`, `<C-]>`, `<A-Up/Down>`).
//! - Initializes auto-closing pairs for parentheses, quotes, and brackets to match Hybrid mode.
//!
//! # Non-Goals & Invariants
//! - Must NOT perform blocking network or filesystem requests.
//! - Must NOT manage egui rendering or UI state.

use rmpv::Value;

/// Embedded Lua scripts for MindForge's language server integration.
pub const LSP_MODULES: &[(&str, &str)] = &[
    (
        "mindforge.lsp",
        include_str!("../../assets/nvim/mindforge/lsp/init.lua"),
    ),
    (
        "mindforge.lsp.paths",
        include_str!("../../assets/nvim/mindforge/lsp/paths.lua"),
    ),
    (
        "mindforge.lsp.notify",
        include_str!("../../assets/nvim/mindforge/lsp/notify.lua"),
    ),
    (
        "mindforge.lsp.progress",
        include_str!("../../assets/nvim/mindforge/lsp/progress.lua"),
    ),
    (
        "mindforge.lsp.config",
        include_str!("../../assets/nvim/mindforge/lsp/config.lua"),
    ),
    (
        "mindforge.lsp.registry",
        include_str!("../../assets/nvim/mindforge/lsp/registry.lua"),
    ),
    (
        "mindforge.lsp.install",
        include_str!("../../assets/nvim/mindforge/lsp/install.lua"),
    ),
    (
        "mindforge.lsp.emmet",
        include_str!("../../assets/nvim/mindforge/lsp/emmet.lua"),
    ),
    (
        "mindforge.lsp.snippets",
        include_str!("../../assets/nvim/mindforge/lsp/snippets.lua"),
    ),
    (
        "mindforge.lsp.completion",
        include_str!("../../assets/nvim/mindforge/lsp/completion.lua"),
    ),
    (
        "mindforge.lsp.status",
        include_str!("../../assets/nvim/mindforge/lsp/status.lua"),
    ),
    (
        "mindforge.lsp.commands",
        include_str!("../../assets/nvim/mindforge/lsp/commands.lua"),
    ),
];

/// Initial configuration script executed inside Neovim on backend startup.
pub const INIT_LUA: &str = r#"
    vim.cmd([[
        set mouse=a number relativenumber cursorline numberwidth=4 signcolumn=no laststatus=0 noruler noshowmode virtualedit=onemore guicursor=a:ver1-Cursor/lCursor fillchars+=eob:\ 
        syntax on
        syntax enable
        filetype plugin indent on
        set termguicolors
        hi Cursor NONE
        hi TermCursor NONE
    ]])
    vim.g.mapleader = ' '
    local opts = { noremap = true, silent = true, expr = true }
    vim.keymap.set("n", "j", "v:count == 0 ? 'gj' : 'j'", opts)
    vim.keymap.set("n", "k", "v:count == 0 ? 'gk' : 'k'", opts)
    vim.keymap.set("n", "<Down>", "v:count == 0 ? 'gj' : '<Down>'", opts)
    vim.keymap.set("n", "<Up>", "v:count == 0 ? 'gk' : '<Up>'", opts)

    -- The user's embedded init maps `jk` to Escape in Insert mode.
    -- That prefix mapping delays every literal `j`; remove it only in
    -- this editor instance so ordinary typing is immediate.
    pcall(vim.keymap.del, 'i', 'jk')

    -- Clear search highlight and active snippet session on Escape in normal mode
    vim.keymap.set('n', '<Esc>', '<cmd>nohlsearch<CR><cmd>lua if vim.w._mindforge_live_search_id then pcall(vim.fn.matchdelete, vim.w._mindforge_live_search_id); vim.w._mindforge_live_search_id = nil end; if vim.snippet and vim.snippet.stop then pcall(vim.snippet.stop) end<CR>', { silent = true })

    -- Enforce identical gutter layout regardless of filetype or LSP attachment
    vim.api.nvim_create_autocmd({ 'FileType', 'BufEnter', 'BufWinEnter' }, {
        callback = function()
            vim.opt_local.numberwidth = 4
            vim.opt_local.signcolumn = 'no'
            vim.opt_local.foldcolumn = '0'
        end,
    })

    -- Markdown wikilink syntax highlighting
    vim.api.nvim_create_autocmd({ 'FileType' }, {
        pattern = { 'markdown', 'md' },
        callback = function()
            pcall(vim.cmd, [[
                syntax match markdownWikiLink /\[\[[^\]]\+\]\]/
                hi def link markdownWikiLink markdownWikiLink
            ]])
        end,
    })

    -- Indent / Dedent (Ctrl+] / Ctrl+[)
    vim.keymap.set('n', '<C-]>', '>>', { noremap = true, silent = true })
    vim.keymap.set('v', '<C-]>', '>gv', { noremap = true, silent = true })
    vim.keymap.set('i', '<C-]>', '<C-t>', { noremap = true, silent = true })
    vim.keymap.set('n', '<C-[>', '<<', { noremap = true, silent = true })
    vim.keymap.set('v', '<C-[>', '<gv', { noremap = true, silent = true })
    vim.keymap.set('i', '<C-[>', '<C-d>', { noremap = true, silent = true })

    -- Move lines up / down (Alt+Up / Alt+Down)
    vim.keymap.set('n', '<A-Up>', '<cmd>m .-2<CR>==', { noremap = true, silent = true })
    vim.keymap.set('n', '<A-Down>', '<cmd>m .+1<CR>==', { noremap = true, silent = true })
    vim.keymap.set('v', '<A-Up>', ":m '<-2<CR>gv=gv", { noremap = true, silent = true })
    vim.keymap.set('v', '<A-Down>', ":m '>+1<CR>gv=gv", { noremap = true, silent = true })
    vim.keymap.set('i', '<A-Up>', '<Esc><cmd>m .-2<CR>==gi', { noremap = true, silent = true })
    vim.keymap.set('i', '<A-Down>', '<Esc><cmd>m .+1<CR>==gi', { noremap = true, silent = true })

    -- Auto-closing pairs so the Neovim surface matches Hybrid typing.
    if vim.g.mindforge_autopair ~= false then
        local function next_char()
            local col = vim.fn.col('.')
            return vim.fn.getline('.'):sub(col, col)
        end
        local function prev_char()
            local col = vim.fn.col('.') - 1
            if col < 1 then return '' end
            return vim.fn.getline('.'):sub(col, col)
        end
        local imap = function(lhs, fn)
            vim.keymap.set('i', lhs, fn, { expr = true, noremap = true, silent = true })
        end
        for open, close in pairs({ ['('] = ')', ['['] = ']', ['{'] = '}' }) do
            imap(open, function() return open .. close .. '<Left>' end)
            imap(close, function()
                if next_char() == close then return '<Right>' end
                return close
            end)
        end
        for _, quote in ipairs({ '"', "'", '`' }) do
            imap(quote, function()
                if next_char() == quote then return '<Right>' end
                if prev_char():match('[%w_]') or next_char():match('[%w_]') then return quote end
                return quote .. quote .. '<Left>'
            end)
        end
        local pairs_map = { ['('] = ')', ['['] = ']', ['{'] = '}', ['"'] = '"', ["'"] = "'", ['`'] = '`' }
        imap('<BS>', function()
            local p, n = prev_char(), next_char()
            if p ~= '' and pairs_map[p] == n then return '<Right><BS><BS>' end
            return '<BS>'
        end)
    end
"#;

/// Lua bootstrap code executed to register package preloads and initialize MindForge's LSP module.
pub const LSP_BOOT: &str = r#"
    local mods = ...
    for name, src in pairs(mods) do
        package.preload[name] = assert(load(src, '=' .. name))
    end
    local ok, err = pcall(function() require('mindforge.lsp').setup() end)
    if not ok then vim.g.mindforge_lsp_error = tostring(err) end
"#;

/// Encodes the LSP module map into MessagePack for Neovim consumption.
pub fn build_lsp_modules_value() -> Value {
    Value::Map(
        LSP_MODULES
            .iter()
            .map(|(name, source)| (Value::from(*name), Value::from(*source)))
            .collect(),
    )
}
