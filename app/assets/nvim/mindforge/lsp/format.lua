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
  sql = {
    { 'sql-formatter' },
    { 'sqlfluff', 'format', '-' },
    { 'pg_format' },
    { 'sleek' },
    { 'sqlfmt', '-' },
    { 'prettier', '--stdin-filepath', 'query.sql' },
  },
}

local function resolve_exe(bin_name)
  local exe = vim.fn.exepath(bin_name)
  if exe ~= '' then
    return exe
  end

  local is_windows = vim.fn.has('win32') == 1
  local ok, paths = pcall(require, 'mindforge.lsp.paths')
  if ok and paths and paths.npm_prefix then
    local managed = paths.npm_prefix() .. '/node_modules/.bin/' .. bin_name .. (is_windows and '.cmd' or '')
    if vim.uv.fs_stat(managed) then
      return managed
    end
  end

  local home = vim.uv.os_homedir() or vim.env.USERPROFILE or vim.env.HOME or ''
  local suffixes = is_windows and { '.exe', '.cmd', '.bat', '' } or { '' }
  for _, dir in ipairs({ '/.cargo/bin', '/go/bin', '/scoop/shims', '/.local/bin', '/AppData/Roaming/npm' }) do
    for _, suffix in ipairs(suffixes) do
      local candidate = home .. dir .. '/' .. bin_name .. suffix
      if vim.uv.fs_stat(candidate) then
        return candidate
      end
    end
  end

  return nil
end

local function run_filter(command, text)
  local exe = resolve_exe(command[1])
  if not exe then
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

local function format_sql_builtin(text)
  local TOP_CLAUSES = {
    ['SELECT'] = true, ['FROM'] = true, ['WHERE'] = true,
    ['GROUP BY'] = true, ['ORDER BY'] = true, ['HAVING'] = true,
    ['LIMIT'] = true, ['OFFSET'] = true, ['UNION'] = true,
    ['UNION ALL'] = true, ['INSERT INTO'] = true, ['VALUES'] = true,
    ['UPDATE'] = true, ['SET'] = true, ['DELETE FROM'] = true,
    ['CREATE TABLE'] = true, ['ALTER TABLE'] = true, ['DROP TABLE'] = true,
  }

  local SUB_CLAUSES = {
    ['JOIN'] = true, ['INNER JOIN'] = true, ['LEFT JOIN'] = true,
    ['RIGHT JOIN'] = true, ['FULL JOIN'] = true, ['CROSS JOIN'] = true,
    ['LEFT OUTER JOIN'] = true, ['RIGHT OUTER JOIN'] = true,
    ['ON'] = true, ['AND'] = true, ['OR'] = true,
  }

  local ALL_KEYWORDS = {
    'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'NOT', 'IN', 'IS', 'NULL',
    'LIKE', 'ILIKE', 'BETWEEN', 'EXISTS', 'JOIN', 'INNER JOIN', 'LEFT JOIN',
    'RIGHT JOIN', 'FULL JOIN', 'CROSS JOIN', 'LEFT OUTER JOIN', 'RIGHT OUTER JOIN',
    'ON', 'AS', 'GROUP BY', 'ORDER BY', 'HAVING', 'LIMIT', 'OFFSET', 'UNION',
    'UNION ALL', 'INSERT INTO', 'VALUES', 'UPDATE', 'SET', 'DELETE FROM',
    'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE', 'PRIMARY KEY', 'FOREIGN KEY',
    'REFERENCES', 'DEFAULT', 'NOT NULL', 'UNIQUE', 'CHECK', 'CONSTRAINT',
    'INDEX', 'CREATE INDEX', 'VIEW', 'CREATE VIEW', 'TRIGGER', 'PROCEDURE',
    'FUNCTION', 'BEGIN', 'COMMIT', 'ROLLBACK', 'CASE', 'WHEN', 'THEN', 'ELSE',
    'END', 'DISTINCT', 'ALL', 'ANY', 'SOME', 'ASC', 'DESC', 'COUNT', 'SUM',
    'AVG', 'MIN', 'MAX', 'COALESCE', 'CAST', 'OVER', 'PARTITION BY', 'WINDOW',
    'RETURNING', 'CONFLICT', 'DO NOTHING', 'DO UPDATE', 'TABLE', 'INTO'
  }

  table.sort(ALL_KEYWORDS, function(a, b) return #a > #b end)

  local tokens = {}
  local len = #text
  local i = 1
  while i <= len do
    local c = text:sub(i, i)
    local c2 = text:sub(i, i + 1)
    if c2 == '--' then
      local j = text:find('\n', i, true) or (len + 1)
      table.insert(tokens, { type = 'comment', val = text:sub(i, j - 1) })
      i = j
    elseif c2 == '/*' then
      local j = text:find('*/', i + 2, true)
      if j then
        table.insert(tokens, { type = 'comment', val = text:sub(i, j + 1) })
        i = j + 2
      else
        table.insert(tokens, { type = 'comment', val = text:sub(i) })
        break
      end
    elseif c == "'" or c == '"' or c == '`' then
      local quote = c
      local j = i + 1
      while j <= len do
        local cur = text:sub(j, j)
        if cur == '\\' then
          j = j + 2
        elseif cur == quote then
          j = j + 1
          break
        else
          j = j + 1
        end
      end
      table.insert(tokens, { type = 'literal', val = text:sub(i, j - 1) })
      i = j
    elseif c:match('%s') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('%s') do
        j = j + 1
      end
      table.insert(tokens, { type = 'space', val = ' ' })
      i = j
    elseif c:match('[%a_]') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('[%w_]') do
        j = j + 1
      end
      table.insert(tokens, { type = 'word', val = text:sub(i, j - 1) })
      i = j
    else
      table.insert(tokens, { type = 'punct', val = c })
      i = i + 1
    end
  end

  local i_tok = 1
  local normalized = {}
  while i_tok <= #tokens do
    local tok = tokens[i_tok]
    if tok.type == 'word' then
      local matched_kw = nil
      for _, kw in ipairs(ALL_KEYWORDS) do
        local parts = vim.split(kw, ' ')
        local matches = true
        local step = 0
        for p_idx, part in ipairs(parts) do
          local target_tok = tokens[i_tok + step]
          if not target_tok or target_tok.type ~= 'word' or target_tok.val:upper() ~= part then
            matches = false
            break
          end
          if p_idx < #parts then
            step = step + 1
            while tokens[i_tok + step] and tokens[i_tok + step].type == 'space' do
              step = step + 1
            end
          end
        end
        if matches then
          matched_kw = kw
          i_tok = i_tok + step + 1
          break
        end
      end
      if matched_kw then
        table.insert(normalized, { type = 'keyword', val = matched_kw })
      else
        table.insert(normalized, tok)
        i_tok = i_tok + 1
      end
    else
      table.insert(normalized, tok)
      i_tok = i_tok + 1
    end
  end

  local lines = {}
  local current_line = ''

  local function push_line()
    local trimmed = current_line:gsub('%s+$', '')
    if #trimmed > 0 then
      table.insert(lines, trimmed)
    end
    current_line = ''
  end

  for _, tok in ipairs(normalized) do
    if tok.type == 'keyword' then
      if TOP_CLAUSES[tok.val] then
        push_line()
        current_line = tok.val
      elseif SUB_CLAUSES[tok.val] then
        push_line()
        current_line = '  ' .. tok.val
      else
        if #current_line > 0 and not current_line:match('%s$') and not current_line:match('%($') then
          current_line = current_line .. ' '
        end
        current_line = current_line .. tok.val
      end
    elseif tok.type == 'punct' then
      if tok.val == ';' then
        current_line = current_line .. ';'
        push_line()
      elseif tok.val == ',' then
        current_line = current_line .. ','
        if current_line:match('^SELECT') or current_line:match('^%s%s') then
          push_line()
          current_line = '  '
        else
          current_line = current_line .. ' '
        end
      else
        if tok.val == '(' or tok.val == ')' then
          current_line = current_line .. tok.val
        else
          current_line = current_line .. ' ' .. tok.val .. ' '
        end
      end
    elseif tok.type == 'comment' then
      push_line()
      table.insert(lines, tok.val)
    elseif tok.type == 'space' then
      if #current_line > 0 and not current_line:match('%s$') and not current_line:match('%($') then
        current_line = current_line .. ' '
      end
    else
      current_line = current_line .. tok.val
    end
  end
  push_line()

  return table.concat(lines, '\n') .. '\n'
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

  -- 3. Built-in SQL formatting fallback if no external tool is installed.
  if ft == 'sql' then
    local output = format_sql_builtin(text)
    if output and output ~= '' and output ~= text then
      local cursor = vim.api.nvim_win_get_cursor(0)
      local lines = vim.split(output:gsub('\n$', ''), '\n', { plain = true })
      vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
      local row = math.min(cursor[1], math.max(1, #lines))
      pcall(vim.api.nvim_win_set_cursor, 0, { row, cursor[2] })
      return 'built-in SQL formatter'
    elseif output and output == text then
      return 'already formatted'
    end
  end

  return nil
end

return M
