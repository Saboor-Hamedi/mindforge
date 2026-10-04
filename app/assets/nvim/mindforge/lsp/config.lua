-- Loads the user's <home>/nvim/lsp.lua, creating a commented template when the
-- file (or the whole nvim folder) does not exist yet.

local paths = require('mindforge.lsp.paths')
local say = require('mindforge.lsp.notify').say

local M = {}

local TEMPLATE = [[
-- MindForge LSP configuration (this file is yours; MindForge never overwrites it).
--
-- Install servers from the editor:   :LspInstall html     :LspInstall ts_ls
-- See what is available:             :LspList
--
-- Override a built-in server, or add your own:
return {
  -- Servers you do not want to run:
  -- disable = { 'emmet_ls' },

  -- Print a hint when a server for the current file type is not installed:
  -- hints = false,

  servers = {
    -- pyright = { settings = { python = { analysis = { typeCheckingMode = 'basic' } } } },

    -- A completely custom server (any language):
    -- lua_ls = {
    --   cmd = { 'lua-language-server' },
    --   filetypes = { 'lua' },
    --   root_markers = { '.luarc.json', '.git' },
    -- },
  },
}
]]

--- Returns the user's config table (empty when missing or invalid).
function M.load()
  local path = paths.user_config()
  if not paths.ensure(paths.nvim_dir()) then
    return {}
  end
  if not vim.uv.fs_stat(path) then
    pcall(vim.fn.writefile, vim.split(TEMPLATE, '\n'), path)
    return {}
  end
  local ok, result = pcall(dofile, path)
  if not ok then
    say('error in lsp.lua: ' .. tostring(result))
    return {}
  end
  return type(result) == 'table' and result or {}
end

return M
