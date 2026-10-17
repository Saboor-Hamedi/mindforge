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
//! - Initializes auto-closing pairs for parentheses, quotes, and brackets.
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
        "mindforge.lsp.guard",
        include_str!("../../assets/nvim/mindforge/lsp/guard.lua"),
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
    (
        "mindforge.lsp.format",
        include_str!("../../assets/nvim/mindforge/lsp/format.lua"),
    ),
    (
        "mindforge.autopairs",
        include_str!("../../assets/nvim/mindforge/autopairs.lua"),
    ),
];

/// Default user `init.lua` created in `<APPDATA>/mindforge/mindforge/nvim/init.lua`
/// when none exists yet. Never overwrites an existing file.
pub const DEFAULT_USER_INIT_LUA: &str = r#"-- MindForge Neovim Configuration (init.lua)
-- This file is loaded automatically on startup and live-reloaded whenever saved.
-- Every option, keymap, autocmd, and formatter configured here takes full precedence.

-- Gutter line numbers (works identically to terminal Neovim)
vim.opt.number = true
vim.opt.relativenumber = true

-- Indentation & editing feel
vim.opt.expandtab = true
vim.opt.tabstop = 4
vim.opt.softtabstop = 4
vim.opt.shiftwidth = 4
vim.opt.smartindent = true

-- Example custom keymaps:
-- vim.keymap.set("i", "jk", "<Esc>", { noremap = true, silent = true, desc = "Exit insert mode" })
"#;

/// Initial configuration script executed inside Neovim on backend startup.
pub const INIT_LUA: &str = r#"
    -- Disable built-in SQL omni mappings so ftplugin/sql.vim never hijacks <C-c> or typing keys
    vim.g.omni_sql_no_default_maps = 1
    vim.g.ftplugin_sql_omni_key = ''

    vim.cmd([=[
        set mouse=a number relativenumber cursorline numberwidth=4 signcolumn=no foldcolumn=0 laststatus=0 noruler noshowmode showcmd virtualedit=onemore guicursor=a:ver1-Cursor/lCursor statusline=\  fillchars+=eob:\ ,stl:\ ,stlnc:\  shortmess+=ltToOCFsSWAI
        set expandtab tabstop=4 softtabstop=4 shiftwidth=4 smartindent
        syntax on
        syntax enable
        filetype plugin indent on
        set termguicolors
        hi Cursor NONE
        hi TermCursor NONE
        hi StatusLine NONE
        hi StatusLineNC NONE
    ]=])
    local nvim_dir = (vim.g.mindforge_home or (vim.fn.stdpath('data') .. '/mindforge')) .. '/nvim'
    pcall(function() vim.opt.runtimepath:prepend(nvim_dir) end)
    vim.g.mapleader = ' '
    local opts = { noremap = true, silent = true, expr = true }
    vim.keymap.set("n", "j", "v:count == 0 ? 'gj' : 'j'", opts)
    vim.keymap.set("n", "k", "v:count == 0 ? 'gk' : 'k'", opts)
    vim.keymap.set("n", "<Down>", "v:count == 0 ? 'gj' : '<Down>'", opts)
    vim.keymap.set("n", "<Up>", "v:count == 0 ? 'gk' : '<Up>'", opts)

    -- Publish current window's line-number gutter state to MindForge host
    _G._mindforge_sync_options = function()
        local show_numbers = (vim.wo.number or vim.wo.relativenumber) and true or false
        pcall(vim.rpcnotify, 0, 'mindforge_options', show_numbers)
    end

    -- Load or hot-reload the user's `<home>/nvim/init.lua` so all user options
    -- and keymaps behave identically to terminal Neovim.
    _G._mindforge_load_user_init = function(path, is_reload)
        if is_reload then
            vim.opt.expandtab = true
            vim.opt.tabstop = 4
            vim.opt.softtabstop = 4
            vim.opt.shiftwidth = 4
            vim.opt.smartindent = true
            pcall(function()
                require('mindforge.lsp.completion').remap()
                require('mindforge.autopairs').setup()
            end)
        end
        local init_path = (type(path) == 'string' and path ~= '') and path or (nvim_dir .. '/init.lua')
        if vim.uv.fs_stat(init_path) then
            -- Start from terminal-Neovim defaults before running init.lua so
            -- `number` and `relativenumber` reflect what init.lua sets.
            vim.opt.number = false
            vim.opt.relativenumber = false
            local ok, err = pcall(dofile, init_path)
            if not ok then
                local clean_err = tostring(err):gsub('^.*[/\\]', ''):gsub('^_?options%.lua:%d+:%s*', '')
                vim.notify('Error in init.lua: ' .. clean_err, vim.log.levels.ERROR)
            end
        else
            vim.opt.number = true
            vim.opt.relativenumber = true
        end
        -- Enforce fixed grid geometry invariants without touching number / relativenumber
        vim.opt.numberwidth = 4
        vim.opt.signcolumn = 'no'
        vim.opt.foldcolumn = '0'
        vim.opt.laststatus = 0
        vim.opt.statusline = ' '
        vim.opt.ruler = false
        vim.opt.showmode = false
        vim.opt.showcmd = true
        _G._mindforge_sync_options()
        pcall(vim.cmd, 'redraw!')
    end

    vim.api.nvim_create_autocmd('OptionSet', {
        pattern = { 'number', 'relativenumber' },
        callback = function()
            if _G._mindforge_sync_options then
                _G._mindforge_sync_options()
            end
        end,
    })

    -- Auto-reload init.lua whenever it is written from inside Neovim
    vim.api.nvim_create_autocmd('BufWritePost', {
        pattern = { '*/nvim/init.lua', '*\\nvim\\init.lua' },
        callback = function(ev)
            if _G._mindforge_load_user_init then
                _G._mindforge_load_user_init(ev.file, true)
            end
        end,
    })

    -- K = hover. Use the attached language server's hover when one is active;
    -- otherwise do nothing. The default `:Man` fallback prints the noisy
    -- "no identifier under cursor" error for code files, which then lingers in
    -- the message area.
    vim.keymap.set('n', 'K', function()
        for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
            if client.supports_method('textDocument/hover') then
                vim.lsp.buf.hover()
                return
            end
        end
    end, { silent = true })

    -- Clear search highlight, close quickfix/references, and clear active snippet on Escape in normal mode
    vim.keymap.set('n', '<Esc>', '<cmd>nohlsearch<CR><cmd>lua if vim.w._mindforge_live_search_id then pcall(vim.fn.matchdelete, vim.w._mindforge_live_search_id); vim.w._mindforge_live_search_id = nil end; if vim.snippet and vim.snippet.stop then pcall(vim.snippet.stop) end; pcall(vim.cmd, "cclose"); pcall(vim.cmd, "lclose")<CR>', { silent = true })

    vim.api.nvim_create_autocmd('FileType', {
        pattern = 'qf',
        callback = function(ev)
            vim.keymap.set('n', '<Esc>', '<cmd>cclose<CR>', { buffer = ev.buf, silent = true })
            vim.keymap.set('n', 'q', '<cmd>cclose<CR>', { buffer = ev.buf, silent = true })
        end,
    })

    -- Enforce identical gutter layout and hidden statusline regardless of filetype or window events
    vim.api.nvim_create_autocmd({ 'FileType', 'BufEnter', 'BufWinEnter', 'WinEnter', 'WinNew' }, {
        callback = function(ev)
            vim.opt_local.numberwidth = 4
            vim.opt_local.signcolumn = 'no'
            vim.opt_local.foldcolumn = '0'
            vim.opt.laststatus = 0
            vim.opt.statusline = ' '
            vim.opt.ruler = false
            vim.opt.showmode = false
            local ft = vim.bo[ev.buf or 0].filetype
            if ft == 'lua' or ft == 'vim' or ft == 'query' then
                pcall(vim.treesitter.start, ev.buf or 0)
            end
            if _G._mindforge_sync_options then
                _G._mindforge_sync_options()
            end
        end,
    })

    -- Clean up SQL omni maps if a built-in ftplugin still sets any
    vim.api.nvim_create_autocmd('FileType', {
        pattern = { 'sql', 'mysql', 'mariadb', 'sqlite', 'pgsql', 'plsql' },
        callback = function(ev)
            vim.bo[ev.buf].omnifunc = ''
        end,
    })

    -- Markdown wikilink syntax highlighting & remove intraword underscore error highlight
    vim.api.nvim_create_autocmd({ 'FileType', 'Syntax' }, {
        pattern = { 'markdown', 'md' },
        callback = function()
            pcall(vim.cmd, [=[
                silent! syntax clear markdownError
                hi! link markdownError Normal
                syntax match markdownWikiLink /\[\[[^\]]\+\]\]/
                hi def link markdownWikiLink markdownWikiLink
            ]=])
        end,
    })

    -- Note: Ctrl+] / Ctrl+[ indent/dedent is driven from Rust (`VimBackend::indent`)
    -- because `<C-[>` shares byte 0x1B with `<Esc>` in Neovim. Mapping `<C-[>` in
    -- Visual mode would hijack `<Esc>` and prevent exiting Visual selection!
    vim.keymap.set({ 'v', 'x', 's' }, '<Esc>', '<Esc>', { noremap = true, silent = true })

    -- Move lines up / down (Alt+Up / Alt+Down)
    vim.keymap.set('n', '<A-Up>', '<cmd>m .-2<CR>==', { noremap = true, silent = true })
    vim.keymap.set('n', '<A-Down>', '<cmd>m .+1<CR>==', { noremap = true, silent = true })
    vim.keymap.set('v', '<A-Up>', ":m '<-2<CR>gv=gv", { noremap = true, silent = true })
    vim.keymap.set('v', '<A-Down>', ":m '>+1<CR>gv=gv", { noremap = true, silent = true })
    vim.keymap.set('i', '<A-Up>', '<Esc><cmd>m .-2<CR>==gi', { noremap = true, silent = true })
    vim.keymap.set('i', '<A-Down>', '<Esc><cmd>m .+1<CR>==gi', { noremap = true, silent = true })

    -- Auto-pairs are installed by the `mindforge.autopairs` module (see
    -- LSP_BOOT), before the user's init.lua is sourced.
"#;

/// Lua bootstrap code executed to register package preloads and initialize MindForge's LSP module.
pub const LSP_BOOT: &str = r#"
    local mods, default_init = ...
    _G._mindforge_default_init_lua = default_init
    for name, src in pairs(mods) do
        package.preload[name] = assert(load(src, '=' .. name))
    end
    local ok, err = pcall(function() require('mindforge.lsp').setup() end)
    if not ok then vim.g.mindforge_lsp_error = tostring(err) end
    pcall(function() require('mindforge.autopairs').setup() end)
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
