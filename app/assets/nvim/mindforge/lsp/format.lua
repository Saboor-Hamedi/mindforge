-- External formatter fallback for `:Format` and <leader>f.
--
-- `vim.lsp.buf.format` only works when an attached server advertises
-- `textDocument/formatting`. Servers such as pyright do not implement it, so
-- when no server can format we fall back to a well-known command-line
-- formatter for the buffer's filetype. Everything is piped through
-- stdin -> stdout so nothing is written to disk.

local M = {}

-- filetype -> ordered list of candidate commands. Each command receives the
-- buffer on stdin and must write the formatted result to stdout.
local FORMATTERS = {
  python = { { 'ruff', 'format', '-' }, { 'black', '--quiet', '-' }, { 'autopep8', '-' } },
  rust = { { 'rustfmt', '--emit', 'stdout' } },
  go = { { 'gofmt' } },
  lua = { { 'stylua', '-' } },
  javascript = { { 'prettier', '--stdin-filepath', 'file.js' } },
  javascriptreact = { { 'prettier', '--stdin-filepath', 'file.jsx' } },
  typescript = { { 'prettier', '--stdin-filepath', 'file.ts' } },
  typescriptreact = { { 'prettier', '--stdin-filepath', 'file.tsx' } },
  json = { { 'prettier', '--stdin-filepath', 'file.json' } },
  jsonc = { { 'prettier', '--stdin-filepath', 'file.json' } },
  css = { { 'prettier', '--stdin-filepath', 'file.css' } },
  scss = { { 'prettier', '--stdin-filepath', 'file.scss' } },
  html = { { 'prettier', '--stdin-filepath', 'file.html' } },
  yaml = { { 'prettier', '--stdin-filepath', 'file.yaml' } },
  markdown = { { 'prettier', '--stdin-filepath', 'file.md' } },
  c = { { 'clang-format' } },
  cpp = { { 'clang-format' } },
}

local function run_filter(command, text)
  local exe = vim.fn.exepath(command[1])
  if exe == '' then
    return nil
  end
  local cmd = vim.deepcopy(command)
  cmd[1] = exe
  local ok, result = pcall(function()
    return vim.system(cmd, { text = true, stdin = text }, nil):wait()
  end)
  if not ok or not result or result.code ~= 0 then
    return nil
  end
  return result.stdout
end

--- Formats the current buffer.
--- Returns the name of the formatter used, or nil when nothing could format it.
function M.format()
  local buf = vim.api.nvim_get_current_buf()

  -- 1. Prefer a language server that actually implements formatting.
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
    if client.supports_method('textDocument/formatting') then
      vim.lsp.buf.format({ bufnr = buf, async = true })
      return 'lsp'
    end
  end

  -- 2. Fall back to a command-line formatter for this filetype.
  local ft = vim.bo[buf].filetype
  local text = table.concat(vim.api.nvim_buf_get_lines(buf, 0, -1, false), '\n') .. '\n'
  for _, command in ipairs(FORMATTERS[ft] or {}) do
    local output = run_filter(command, text)
    if output and output ~= '' then
      local cursor = vim.api.nvim_win_get_cursor(0)
      local lines = vim.split(output:gsub('\n$', ''), '\n', { plain = true })
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
      local row = math.min(cursor[1], math.max(1, #lines))
      pcall(vim.api.nvim_win_set_cursor, 0, { row, cursor[2] })
      return command[1]
    end
  end

  return nil
end

return M
