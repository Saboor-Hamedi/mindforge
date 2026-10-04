-- Publishes the language-server state of the current buffer so MindForge can
-- draw the status bar indicator: connected / starting / unavailable / none.

local registry = require('mindforge.lsp.registry')
local installer = require('mindforge.lsp.install')

local M = {}

local BUILTIN = 'mindforge-snippets'
local last

local function compute(buf)
  local ft = vim.bo[buf].filetype
  local names = registry.for_filetype(ft)
  if #names == 0 then
    return 'none', ''
  end
  local ready, starting = {}, {}
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
    if client.name ~= BUILTIN then
      table.insert(client.initialized and ready or starting, client.name)
    end
  end
  if #ready > 0 then
    return 'connected', table.concat(ready, ', ')
  end
  if #starting > 0 then
    return 'starting', table.concat(starting, ', ')
  end
  for _, name in ipairs(names) do
    if installer.is_installing(name) then
      return 'starting', name
    end
  end
  for _, name in ipairs(names) do
    if installer.is_installed(name) then
      return 'starting', name
    end
  end
  return 'unavailable', table.concat(names, ', ')
end

--- Sends the state when it changed (or `force`).
function M.publish(force)
  local state, detail = compute(vim.api.nvim_get_current_buf())
  local key = state .. '|' .. detail
  if force or key ~= last then
    last = key
    pcall(vim.rpcnotify, 0, 'mindforge_lsp_status', state, detail)
  end
end

function M.setup(group)
  local timer = vim.uv.new_timer()
  local function soon()
    timer:stop()
    timer:start(80, 0, vim.schedule_wrap(function()
      M.publish(false)
    end))
  end
  vim.api.nvim_create_autocmd({ 'LspAttach', 'LspDetach', 'BufEnter', 'FileType' }, { group = group, callback = soon })
  -- Servers become "connected" asynchronously after attaching.
  vim.api.nvim_create_autocmd('LspProgress', { group = group, callback = soon })
  local poll = vim.uv.new_timer()
  poll:start(1500, 1500, vim.schedule_wrap(function()
    M.publish(false)
  end))
end

return M
