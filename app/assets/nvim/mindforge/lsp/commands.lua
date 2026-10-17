-- User commands: :LspInstall :LspUninstall :LspList :LspInfo :LspRestart :LspConfig
-- LSP commands: :Format :Rename :CodeAction :References :Hover :Definition

local registry = require('mindforge.lsp.registry')
local installer = require('mindforge.lsp.install')
local paths = require('mindforge.lsp.paths')
local say = require('mindforge.lsp.notify').say

local M = {}

local function server_names()
  return registry.names()
end

--- Sends every server with its status to MindForge's list panel.
local function push_list(open, filter)
  local rows = {}
  for _, name in ipairs(registry.names()) do
    local spec = registry.servers[name]
    local status = 'available'
    if installer.is_installing(name) then
      status = 'installing'
    elseif installer.is_installed(name) then
      status = #vim.lsp.get_clients({ name = name }) > 0 and 'running' or 'installed'
    end
    rows[#rows + 1] = { name, status, spec.desc or '', table.concat(spec.config.filetypes or {}, ' ') }
  end
  return pcall(vim.rpcnotify, 0, 'mindforge_lsp_list', rows, open and true or false, filter or '')
end

--- `refresh` re-registers servers after an install changes what can run.
function M.setup(refresh)
  local function cmd(name, fn, opts)
    vim.api.nvim_create_user_command(name, fn, opts)
    local lower = name:lower()
    if lower ~= name then
      pcall(vim.cmd, string.format('cabbrev %s %s', lower, name))
    end
  end

  cmd('LspInstall', function(opts)
    local names = registry.resolve_targets(opts.fargs, vim.bo.filetype)
    if vim.bo.filetype == 'lua' or vim.tbl_contains(names, 'lua_ls') then
      if vim.bo.filetype == '' or vim.bo.filetype == 'text' then
        local bufname = vim.api.nvim_buf_get_name(0):lower()
        if bufname:match('%.lua$') then
          vim.bo.filetype = 'lua'
          vim.bo.syntax = 'lua'
        end
      end
      pcall(vim.treesitter.start, 0)
    end
    if #names == 0 then
      local target = (#opts.fargs > 0) and table.concat(opts.fargs, ', ') or vim.bo.filetype
      return say('no known server for "' .. target .. '". Try :LspList')
    end
    for _, name in ipairs(names) do
      installer.install(name, function(ok)
        if ok then
          refresh()
        end
        push_list(false)
      end)
    end
    push_list(false)
  end, { nargs = '*', complete = server_names, desc = 'Install language servers' })

  cmd('LspUninstall', function(opts)
    local names = registry.resolve_targets(opts.fargs, vim.bo.filetype)
    for _, name in ipairs(names) do
      installer.uninstall(name, function()
        pcall(vim.lsp.enable, name, false)
        push_list(false)
      end)
    end
  end, { nargs = '+', complete = server_names, desc = 'Uninstall language servers' })

  cmd('LspList', function(opts)
    if not push_list(true, opts.args) then
      local installed = {}
      for _, name in ipairs(registry.names()) do
        if installer.is_installed(name) then
          table.insert(installed, name)
        end
      end
      say(#installed .. ' installed of ' .. #registry.names() .. ': ' .. table.concat(installed, ', '))
    end
  end, { nargs = '?', desc = 'Browse language servers' })

  cmd('LspInfo', function()
    local names = {}
    for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
      local root = client.config.root_dir
      table.insert(names, client.name .. (root and (' @ ' .. vim.fn.fnamemodify(root, ':~')) or ''))
    end
    say('attached: ' .. (#names > 0 and table.concat(names, '; ') or 'none') .. '  (filetype ' .. vim.bo.filetype .. ')')
  end, { desc = 'Show language servers attached to this buffer' })

  cmd('LspLog', function()
    local file = vim.lsp.log.get_filename()
    if vim.fn.filereadable(file) == 1 then
      vim.cmd('edit ' .. vim.fn.fnameescape(file))
    else
      say('no LSP log yet (' .. file .. ')')
    end
  end, { desc = 'Open the language server log' })

  cmd('LspRestart', function()
    local buf = vim.api.nvim_get_current_buf()
    local clients = vim.lsp.get_clients({ bufnr = buf })
    if #clients == 0 then
      say('no language servers attached to restart')
      return
    end
    local ft = vim.bo[buf].filetype
    local pending = 0
    for _, client in ipairs(clients) do
      if not vim.startswith(client.name, 'mindforge-') then
        pending = pending + 1
        local orig_on_exit = client.on_exit
        client.on_exit = function(...)
          if orig_on_exit then pcall(orig_on_exit, ...) end
          pending = pending - 1
          if pending <= 0 then
            vim.schedule(function()
              if vim.api.nvim_buf_is_valid(buf) then
                vim.api.nvim_exec_autocmds('FileType', { buffer = buf })
                say('LSP restarted for ' .. ft)
              end
            end)
          end
        end
        pcall(client.stop, client)
      end
    end
    if pending == 0 then
      vim.api.nvim_exec_autocmds('FileType', { buffer = buf })
      return
    end
    -- Safety fallback if on_exit does not fire
    vim.defer_fn(function()
      if pending > 0 and vim.api.nvim_buf_is_valid(buf) then
        vim.api.nvim_exec_autocmds('FileType', { buffer = buf })
      end
    end, 1500)
  end, { desc = 'Restart language servers for this buffer' })

  cmd('LspConfig', function()
    paths.ensure(paths.nvim_dir())
    vim.cmd('edit ' .. vim.fn.fnameescape(paths.user_config()))
  end, { desc = 'Open your MindForge LSP configuration' })

  local function open_init_config()
    local path = paths.init_config()
    paths.ensure(paths.nvim_dir())
    if not vim.uv.fs_stat(path) and _G._mindforge_default_init_lua then
      pcall(vim.fn.writefile, vim.split(_G._mindforge_default_init_lua, '\n'), path)
    end
    vim.cmd('edit ' .. vim.fn.fnameescape(path))
  end

  cmd('Init', open_init_config, { desc = 'Open your MindForge Neovim init.lua configuration' })
  cmd('InitConfig', open_init_config, { desc = 'Open your MindForge Neovim init.lua configuration' })
  cmd('Config', open_init_config, { desc = 'Open your MindForge Neovim init.lua configuration' })

  cmd('FormatConfig', function()
    local path = paths.formatters_config()
    paths.ensure(paths.nvim_dir())
    if not vim.uv.fs_stat(path) then
      local template = [[-- MindForge Custom Formatters (this file is yours; MindForge never overwrites it).
--
-- Configure custom formatters for any filetype or override built-in ones.
-- You can specify:
--   1. An executable and arguments:  { 'sql-formatter', '--config', '{"keywordCase": "upper"}' }
--   2. Multiple fallback commands:   { { 'sql-formatter' }, { 'sqlfluff', 'format', '-' } }
--   3. A Lua function:               function(text, bufnr) return formatted_text end
--
return {
  -- Custom SQL formatter:
  -- sql = { 'sql-formatter', '--config', '{"keywordCase": "upper", "tabWidth": 2}' },

  -- Custom Python formatter:
  -- python = { 'black', '--quiet', '-' },

  -- Custom Lua function:
  -- markdown = function(text)
  --   return text:gsub('\t', '  ')
  -- end,
}
]]
      pcall(vim.fn.writefile, vim.split(template, '\n'), path)
    end
    vim.cmd('edit ' .. vim.fn.fnameescape(path))
  end, { desc = 'Open your MindForge custom formatters configuration' })

  --- True when any attached client advertises `method`.
  local function supports(method)
    for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
      if client.supports_method(method) then
        return true
      end
    end
    return false
  end

  cmd('Format', function()
    local name = require('mindforge.lsp.format').format()
    if name then
      if name == 'lsp' then
        say('Formatted with LSP')
      elseif name == 'already formatted' then
        say('file is already formatted')
      else
        say('Formatted with ' .. name)
      end
    else
      local ft = vim.bo.filetype
      if ft == 'sql' then
        say('no SQL formatter available (run :LspInstall sql or install sql-formatter)')
      else
        say('no formatter available for this file (install one or :LspInstall)')
      end
    end
  end, { desc = 'Format current buffer (LSP or external formatter)' })

  cmd('Rename', function()
    local clients = vim.lsp.get_clients({ bufnr = 0 })
    if #clients == 0 then return say('no LSP server attached for rename') end
    local supported = false
    for _, client in ipairs(clients) do
      if client.supports_method('textDocument/rename') then
        supported = true
        break
      end
    end
    if not supported then return say('Rename is not supported by the attached LSP server for this filetype') end
    vim.lsp.buf.rename()
  end, { desc = 'Rename symbol under cursor' })

  cmd('CodeAction', function()
    if #vim.lsp.get_clients({ bufnr = 0 }) == 0 then
      return say('no LSP server attached for code action. Use :LspInstall or :LspList')
    end
    if not supports('textDocument/codeAction') then
      return say('code actions are not supported for this filetype')
    end
    vim.lsp.buf.code_action()
  end, { desc = 'Trigger LSP code action' })

  cmd('References', function()
    if #vim.lsp.get_clients({ bufnr = 0 }) == 0 then
      return say('no LSP server attached to find references. Use :LspInstall or :LspList')
    end
    if not supports('textDocument/references') then
      return say('find references is not supported for this filetype')
    end
    vim.lsp.buf.references()
  end, { desc = 'Find symbol references' })

  cmd('Hover', function()
    if #vim.lsp.get_clients({ bufnr = 0 }) == 0 then
      return say('no LSP server attached for hover documentation. Use :LspInstall or :LspList')
    end
    if not supports('textDocument/hover') then
      return say('hover documentation is not supported for this filetype')
    end
    vim.lsp.buf.hover()
  end, { desc = 'Show hover documentation' })

  cmd('Definition', function()
    if #vim.lsp.get_clients({ bufnr = 0 }) == 0 then
      return say('no LSP server attached to go to definition. Use :LspInstall or :LspList')
    end
    if not supports('textDocument/definition') then
      return say('go to definition is not supported for this filetype')
    end
    vim.lsp.buf.definition()
  end, { desc = 'Go to symbol definition' })
end

return M
