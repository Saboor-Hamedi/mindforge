-- Filesystem layout. Everything lives under <home>/nvim so the folder is
-- self-contained and safe to delete or back up:
--
--   <home>/nvim/init.lua       user's Neovim config (optional)
--   <home>/nvim/lsp.lua        MindForge LSP overrides (created on first run)
--   <home>/nvim/lsp/*.lua      native Neovim server definitions (optional)
--   <home>/nvim/servers/npm    servers installed by :LspInstall

local M = {}

--- MindForge data directory; falls back to Neovim's own data dir when the
--- host did not provide one.
function M.home()
  local home = vim.g.mindforge_home
  if type(home) ~= 'string' or home == '' then
    home = vim.fn.stdpath('data') .. '/mindforge'
  end
  return (home:gsub('\\', '/'))
end

function M.nvim_dir()
  return M.home() .. '/nvim'
end

function M.user_config()
  return M.nvim_dir() .. '/lsp.lua'
end

function M.native_dir()
  return M.nvim_dir() .. '/lsp'
end

function M.npm_prefix()
  return M.nvim_dir() .. '/servers/npm'
end

--- Creates `dir` (and parents). Returns false when it cannot be created.
function M.ensure(dir)
  if vim.uv.fs_stat(dir) then
    return true
  end
  return pcall(vim.fn.mkdir, dir, 'p') and vim.uv.fs_stat(dir) ~= nil
end

return M
