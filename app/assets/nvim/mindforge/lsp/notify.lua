-- Single-line status messages (the editor shows one line under the buffer).

local M = {}

function M.say(msg)
  vim.api.nvim_echo({ { 'MindForge LSP: ' .. msg } }, false, {})
end

return M
