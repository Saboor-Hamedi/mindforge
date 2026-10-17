-- Single-line status messages (the editor shows one line under the buffer).

local M = {}

function M.say(msg)
  pcall(vim.rpcnotify, 0, 'mindforge_status', 'MindForge LSP: ' .. msg)
  pcall(vim.notify, 'MindForge LSP: ' .. msg, vim.log.levels.INFO)
end

return M
