-- Guards Neovim's LSP change tracker against a document-swap race.
--
-- MindForge reuses one Neovim buffer for every document. While a document is
-- swapped (detach -> rename -> set text -> re-enable), a buffer edit can reach
-- `vim.lsp._changetracking.send_changes` for a client whose per-buffer state
-- does not exist yet (its didOpen is still pending) or was just removed (it
-- was detached). Neovim then fails with
--   _changetracking.lua: attempt to index local 'buf_state' (a nil value)
-- Skipping that change is correct in both cases: a pending didOpen sends the
-- full text anyway, and a detached client must not receive changes.

local M = {}

local installed = false

local function upvalue(fn, wanted)
  for index = 1, 255 do
    local name, value = debug.getupvalue(fn, index)
    if name == nil then
      return nil
    end
    if name == wanted then
      return value
    end
  end
  return nil
end

--- Wraps `send_changes` so one group's missing state never aborts the others
--- or surfaces as a Lua callback error.
function M.install()
  if installed then
    return
  end
  local ok, tracking = pcall(require, 'vim.lsp._changetracking')
  if not ok or type(tracking) ~= 'table' or type(tracking.send_changes) ~= 'function' then
    return
  end
  installed = true

  local original = tracking.send_changes
  local send_for_group = upvalue(original, 'send_changes_for_group')
  local get_group = upvalue(original, 'get_group')
  local group_key = upvalue(original, 'group_key')

  if send_for_group and get_group and group_key then
    -- Same as Neovim's implementation, but each group is isolated.
    tracking.send_changes = function(bufnr, firstline, lastline, new_lastline)
      local groups = {}
      for _, client in pairs(vim.lsp.get_clients({ bufnr = bufnr })) do
        local group = get_group(client)
        groups[group_key(group)] = group
      end
      for _, group in pairs(groups) do
        pcall(send_for_group, bufnr, firstline, lastline, new_lastline, group)
      end
    end
  else
    -- Internals changed in this Neovim version: fall back to a plain guard.
    tracking.send_changes = function(...)
      pcall(original, ...)
    end
  end
end

return M
