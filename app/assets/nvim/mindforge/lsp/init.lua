-- MindForge LSP integration (Neovim-native, runs inside the embedded Neovim).
--
--   init.lua        orchestration: server configs, enabling, hints
--   registry.lua    built-in server catalogue
--   install.lua     :LspInstall back end (npm / rustup)
--   config.lua      user overrides in <home>/nvim/lsp.lua
--   paths.lua       filesystem layout
--   completion.lua  completion triggers, keys, diagnostics
--   commands.lua    :Lsp* user commands
--   snippets.lua    built-in snippet server (HTML skeleton, React, ...)
--   notify.lua      status messages
--   progress.lua    busy indicator (installs, LSP indexing)

local registry = require('mindforge.lsp.registry')
local installer = require('mindforge.lsp.install')
local snippets = require('mindforge.lsp.snippets')
local paths = require('mindforge.lsp.paths')
local config = require('mindforge.lsp.config')
local completion = require('mindforge.lsp.completion')
local commands = require('mindforge.lsp.commands')
local say = require('mindforge.lsp.notify').say
local progress = require('mindforge.lsp.progress')
local status = require('mindforge.lsp.status')

local M = {}

local state = { user = {}, hinted = {}, ready = false }

local function disabled(name)
  return vim.tbl_contains(state.user.disable or {}, name)
end

--- Builds the vim.lsp config for `name`, or nil when it cannot run yet.
local function build_config(name)
  local spec = registry.servers[name]
  local override = (state.user.servers or {})[name]
  if not spec and not override then
    return nil
  end
  local server = vim.deepcopy(spec and spec.config or {})
  local args = server.cmd_args or {}
  server.cmd_args = nil
  local command = spec and installer.command(spec)
  if command then
    server.cmd = vim.list_extend(command, args)
  end
  if spec and spec.tsserver then
    local lib = installer.typescript_lib(spec)
    if not lib then
      return nil
    end
    server.init_options = vim.tbl_deep_extend('force', server.init_options or {}, { tsserver = { path = lib } })
  end
  if not server.root_dir then
    server.root_markers = server.root_markers or { '.git' }
  end
  if override then
    server = vim.tbl_deep_extend('force', server, override)
  end
  return server.cmd and server or nil
end

local function enable(name)
  if disabled(name) then
    return
  end
  local server = build_config(name)
  if server then
    vim.lsp.config(name, server)
    vim.lsp.enable(name)
  end
end

local function enable_native_files()
  local dir = paths.native_dir()
  if not vim.uv.fs_stat(dir) then
    return
  end
  for _, file in ipairs(vim.fn.glob(dir .. '/*.lua', false, true)) do
    local name = vim.fn.fnamemodify(file, ':t:r')
    if not disabled(name) then
      vim.lsp.enable(name)
    end
  end
end

--- (Re)registers and enables every server that is runnable.
function M.refresh()
  for _, name in ipairs(registry.names()) do
    enable(name)
  end
  for name in pairs(state.user.servers or {}) do
    if not registry.servers[name] then
      enable(name)
    end
  end
  enable_native_files()
end

local function hint_missing(buf)
  if state.user.hints == false then
    return
  end
  local ft = vim.bo[buf].filetype
  if ft == '' or state.hinted[ft] then
    return
  end
  local missing = {}
  for _, name in ipairs(registry.for_filetype(ft)) do
    if not disabled(name) and not installer.is_installed(name) and not installer.is_installing(name) then
      table.insert(missing, name)
    end
  end
  if #missing > 0 then
    state.hinted[ft] = true
    say(ft .. ' language server missing. Run :LspInstall ' .. table.concat(missing, ' '))
  end
end

--- Called before MindForge swaps the document in the shared Neovim buffer, so
--- servers of the previous language never see the next document.
function M.reset_buffer()
  local buf = vim.api.nvim_get_current_buf()
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
    if not vim.startswith(client.name, 'mindforge-') then
      -- Detach the client from the buffer instead of stopping the process.
      -- This keeps the language server alive in the background with its
      -- in-memory index intact so tab switches do not force full re-indexing.
      pcall(function()
        if vim.lsp.buf_detach_client then
          vim.lsp.buf_detach_client(buf, client.id)
        elseif client.detach_from_buf then
          client:detach_from_buf(buf)
        end
      end)
    end
  end
end

function M.setup()
  if state.ready then
    return
  end
  state.ready = true
  paths.ensure(paths.nvim_dir())
  vim.opt.runtimepath:prepend(paths.nvim_dir())
  state.user = config.load()

  local group = vim.api.nvim_create_augroup('MindForgeLsp', { clear = true })
  completion.setup(group, function(buf)
    pcall(snippets.attach, buf)
    hint_missing(buf)
  end)
  progress.watch_lsp(group)
  commands.setup(M.refresh)
  status.setup(group)
  M.refresh()
end

return M
