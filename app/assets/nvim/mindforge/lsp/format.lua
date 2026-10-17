-- External formatter fallback for `:Format` and <leader>f.
--
-- `vim.lsp.buf.format` only works when an attached server advertises
-- `textDocument/formatting`. Servers such as pyright do not implement it, so
-- when no server can format we fall back to a well-known command-line
-- formatter for the buffer's filetype. Everything is piped through
-- stdin -> stdout so nothing is written to disk.

local M = {}

local SQL_FORMATTER_CFG = '{"keywordCase":"upper","tabWidth":2,"linesBetweenQueries":2}'

local SQL_COMMANDS = {
  { 'sql-formatter', '--config', SQL_FORMATTER_CFG },
  { 'sqlfluff', 'format', '-' },
  { 'pg_format' },
  { 'sleek' },
  { 'sqlfmt', '-' },
}

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
  toml = { { 'taplo', 'fmt', '-' } },
  c = { { 'clang-format' } },
  cpp = { { 'clang-format' } },
  sql = SQL_COMMANDS,
  mysql = SQL_COMMANDS,
  mariadb = SQL_COMMANDS,
  sqlite = SQL_COMMANDS,
  pgsql = SQL_COMMANDS,
  plsql = SQL_COMMANDS,
}

local SQL_FILETYPES = {
  sql = true,
  mysql = true,
  mariadb = true,
  sqlite = true,
  pgsql = true,
  plsql = true,
}

local function resolve_exe(bin_name)
  local is_windows = vim.fn.has('win32') == 1
  local ok, paths = pcall(require, 'mindforge.lsp.paths')
  if ok and paths and paths.npm_prefix then
    local managed = paths.npm_prefix() .. '/node_modules/.bin/' .. bin_name .. (is_windows and '.cmd' or '')
    if vim.uv.fs_stat(managed) then
      return managed
    end
  end

  local exe = vim.fn.exepath(bin_name)
  if exe ~= '' then
    if is_windows then
      local lower = exe:lower()
      local has_ext = lower:match('%.exe$') or lower:match('%.cmd$') or lower:match('%.bat$') or lower:match('%.com$')
      if not has_ext then
        for _, ext in ipairs({ '.exe', '.cmd', '.bat', '.com' }) do
          if vim.uv.fs_stat(exe .. ext) then
            return exe .. ext
          end
        end
      end
    end
    return exe
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

--- Builds a spawnable command array for `vim.system`. On Windows, `.cmd` and
--- `.bat` shims cannot be spawned directly by libuv; npm `.cmd` shims are
--- resolved to `node <script.js>` (avoiding cmd.exe quoting issues) and other
--- batch files fall back to `cmd.exe /c`.
local function build_spawn_cmd(exe, extra_args)
  local is_windows = vim.fn.has('win32') == 1
  local lower = exe:lower()
  if is_windows and lower:match('%.cmd$') then
    local ok_read, lines = pcall(vim.fn.readfile, exe)
    if ok_read and type(lines) == 'table' then
      local script
      for _, line in ipairs(lines) do
        script = line:match('"%%dp0%%[\\/]+([^"]+%.[mc]?js)"') or script
      end
      local node = vim.fn.exepath('node')
      if script and node ~= '' then
        local dir = vim.fs.dirname(exe)
        local cmd = { node, dir .. '/' .. script:gsub('\\', '/') }
        return vim.list_extend(cmd, extra_args)
      end
    end
    local cmd = { 'cmd.exe', '/c', exe }
    return vim.list_extend(cmd, extra_args)
  elseif is_windows and lower:match('%.bat$') then
    local cmd = { 'cmd.exe', '/c', exe }
    return vim.list_extend(cmd, extra_args)
  end
  local cmd = { exe }
  return vim.list_extend(cmd, extra_args)
end

local function run_filter(command, text)
  local exe = resolve_exe(command[1])
  if not exe then
    return nil
  end
  local extra_args = {}
  for idx = 2, #command do
    extra_args[#extra_args + 1] = command[idx]
  end
  local cmd = build_spawn_cmd(exe, extra_args)
  local ok, result = pcall(function()
    return vim.system(cmd, { text = true, stdin = text }, nil):wait()
  end)
  if not ok or not result or result.code ~= 0 or not result.stdout or result.stdout == '' then
    return nil
  end
  return result.stdout
end

local user_formatters = {}

--- Registers an in-memory formatter for a filetype.
--- `target` can be:
--- - a command table: { 'sql-formatter', '--config', '{"keywordCase": "upper"}' }
--- - a list of candidate command tables: { { 'sql-formatter' }, { 'sqlfluff', 'format', '-' } }
--- - a Lua function: function(text, bufnr) return formatted_text end
--- - a command string: 'sql-formatter --language postgresql'
function M.register(filetype, target)
  user_formatters[filetype] = target
end

M.formatters = FORMATTERS

--- Discovers user-defined formatters for `ft`.
--- Checks in order:
--- 1. in-memory M.register()
--- 2. <home>/nvim/formatters/<filetype>.lua (e.g. nvim/formatters/sql.lua)
--- 3. <home>/nvim/formatters.lua (table returning { sql = ... })
--- 4. vim.g.mindforge_formatters
--- 5. <home>/nvim/lsp.lua (config.load().formatters)
local function get_user_formatter(ft)
  if user_formatters[ft] then
    return user_formatters[ft]
  end

  local ok, paths = pcall(require, 'mindforge.lsp.paths')
  if ok and paths and paths.nvim_dir then
    local ft_file = paths.nvim_dir() .. '/formatters/' .. ft .. '.lua'
    if vim.uv.fs_stat(ft_file) then
      local ok_load, res = pcall(dofile, ft_file)
      if ok_load and res ~= nil then
        return res
      end
    end

    local main_file = paths.formatters_config and paths.formatters_config() or (paths.nvim_dir() .. '/formatters.lua')
    if vim.uv.fs_stat(main_file) then
      local ok_load, res = pcall(dofile, main_file)
      if ok_load and type(res) == 'table' and res[ft] then
        return res[ft]
      end
    end
  end

  if type(vim.g.mindforge_formatters) == 'table' and vim.g.mindforge_formatters[ft] then
    return vim.g.mindforge_formatters[ft]
  end

  local ok_cfg, config = pcall(require, 'mindforge.lsp.config')
  if ok_cfg and config and config.load then
    local cfg = config.load()
    local custom = cfg.formatters or cfg.format
    if type(custom) == 'table' and custom[ft] then
      return custom[ft]
    end
  end

  return nil
end

--- Dialect-aware, multi-line SQL formatter supporting Standard SQL, SQLite,
--- MySQL, MariaDB, and PostgreSQL. Formats SELECT, INSERT, UPDATE, DELETE,
--- CREATE/ALTER/DROP TABLE, CTEs, subqueries, JOINs, and CASE expressions.
local function format_sql_builtin(text)
  local MULTI_KEYWORDS = {
    'CREATE OR REPLACE VIEW',
    'CREATE TEMPORARY TABLE',
    'CREATE TEMP TABLE',
    'CREATE UNLOGGED TABLE',
    'CREATE UNIQUE INDEX',
    'INSERT OR IGNORE INTO',
    'INSERT OR REPLACE INTO',
    'ON DUPLICATE KEY UPDATE',
    'LEFT OUTER JOIN',
    'RIGHT OUTER JOIN',
    'FULL OUTER JOIN',
    'NATURAL LEFT JOIN',
    'NATURAL INNER JOIN',
    'SELECT DISTINCT',
    'UNION DISTINCT',
    'WITH RECURSIVE',
    'DELETE FROM',
    'INSERT INTO',
    'REPLACE INTO',
    'CREATE TABLE',
    'ALTER TABLE',
    'DROP TABLE',
    'TRUNCATE TABLE',
    'CREATE INDEX',
    'DROP INDEX',
    'CREATE VIEW',
    'DROP VIEW',
    'CREATE DATABASE',
    'DROP DATABASE',
    'CREATE TRIGGER',
    'DROP TRIGGER',
    'INNER JOIN',
    'LEFT JOIN',
    'RIGHT JOIN',
    'FULL JOIN',
    'CROSS JOIN',
    'NATURAL JOIN',
    'STRAIGHT_JOIN',
    'GROUP BY',
    'ORDER BY',
    'PARTITION BY',
    'UNION ALL',
    'ON CONFLICT',
    'DO NOTHING',
    'DO UPDATE SET',
    'DO UPDATE',
    'IF NOT EXISTS',
    'IF EXISTS',
    'PRIMARY KEY',
    'FOREIGN KEY',
    'NOT NULL',
    'AUTO_INCREMENT',
    'CURRENT_TIMESTAMP',
    'CURRENT_DATE',
    'CURRENT_TIME',
    'ON DELETE CASCADE',
    'ON DELETE SET NULL',
    'ON DELETE RESTRICT',
    'ON DELETE NO ACTION',
    'ON UPDATE CASCADE',
    'ON UPDATE SET NULL',
    'ON UPDATE RESTRICT',
    'ON UPDATE NO ACTION',
    'WITHOUT ROWID',
    'DEFAULT CHARSET',
    'CHARACTER SET',
    'ADD COLUMN',
    'ADD CONSTRAINT',
    'ADD PRIMARY KEY',
    'ADD FOREIGN KEY',
    'ADD UNIQUE',
    'ADD INDEX',
    'ADD KEY',
    'MODIFY COLUMN',
    'CHANGE COLUMN',
    'ALTER COLUMN',
    'DROP COLUMN',
    'RENAME COLUMN',
    'RENAME TO',
    'FETCH FIRST',
    'FETCH NEXT',
    'FOR UPDATE',
    'FOR SHARE',
    'IS NOT NULL',
    'IS NULL',
    'NOT IN',
    'NOT LIKE',
    'NOT BETWEEN',
    'NOT EXISTS',
    'START TRANSACTION',
  }

  local SINGLE_KEYWORDS = {
    'SELECT', 'FROM', 'WHERE', 'AND', 'OR', 'NOT', 'IN', 'IS', 'NULL',
    'LIKE', 'ILIKE', 'BETWEEN', 'EXISTS', 'JOIN', 'ON', 'USING', 'AS',
    'HAVING', 'LIMIT', 'OFFSET', 'UNION', 'INTERSECT', 'EXCEPT', 'VALUES',
    'UPDATE', 'SET', 'DELETE', 'INSERT', 'INTO', 'CREATE', 'ALTER', 'DROP',
    'TRUNCATE', 'TABLE', 'TEMP', 'TEMPORARY', 'VIEW', 'INDEX', 'TRIGGER',
    'PROCEDURE', 'FUNCTION', 'DATABASE', 'SCHEMA', 'USE', 'PRAGMA',
    'BEGIN', 'COMMIT', 'ROLLBACK', 'SAVEPOINT', 'RELEASE', 'EXPLAIN',
    'ANALYZE', 'VACUUM', 'ATTACH', 'DETACH', 'REINDEX', 'WITH', 'RECURSIVE',
    'CASE', 'WHEN', 'THEN', 'ELSE', 'END', 'DISTINCT', 'ALL', 'ANY', 'SOME',
    'ASC', 'DESC', 'NULLS', 'FIRST', 'LAST', 'OVER', 'WINDOW', 'RETURNING',
    'REFERENCES', 'DEFAULT', 'UNIQUE', 'CHECK', 'CONSTRAINT', 'COLLATE',
    'AUTOINCREMENT', 'UNSIGNED', 'ZEROFILL', 'ENGINE', 'CHARSET', 'ROWID',
    'CASCADE', 'RESTRICT', 'CONFLICT', 'REPLACE', 'IGNORE', 'KEY',
    'INTEGER', 'INT', 'BIGINT', 'SMALLINT', 'TINYINT', 'MEDIUMINT',
    'SERIAL', 'BIGSERIAL', 'DECIMAL', 'NUMERIC', 'FLOAT', 'DOUBLE', 'REAL',
    'VARCHAR', 'CHAR', 'TEXT', 'TINYTEXT', 'MEDIUMTEXT', 'LONGTEXT',
    'BLOB', 'BYTEA', 'BINARY', 'VARBINARY', 'BOOLEAN', 'BOOL', 'BIT',
    'DATE', 'TIME', 'DATETIME', 'TIMESTAMP', 'TIMESTAMPTZ', 'YEAR',
    'JSON', 'JSONB', 'UUID', 'ENUM', 'TRUE', 'FALSE',
    'COUNT', 'SUM', 'AVG', 'MIN', 'MAX', 'COALESCE', 'NULLIF', 'IFNULL',
    'NVL', 'CAST', 'CONVERT', 'CONCAT', 'SUBSTRING', 'SUBSTR', 'TRIM',
    'UPPER', 'LOWER', 'LENGTH', 'ROUND', 'FLOOR', 'CEIL', 'ABS', 'NOW',
    'DATE_FORMAT', 'STRFTIME', 'ROW_NUMBER', 'RANK', 'DENSE_RANK', 'LAG', 'LEAD',
  }

  local SINGLE_KW_SET = {}
  for _, kw in ipairs(SINGLE_KEYWORDS) do
    SINGLE_KW_SET[kw] = true
  end

  table.sort(MULTI_KEYWORDS, function(a, b)
    return #a > #b
  end)

  -- Clauses that start at the current block's base indentation
  local TOP_CLAUSES = {
    ['WITH'] = true,
    ['WITH RECURSIVE'] = true,
    ['SELECT'] = true,
    ['SELECT DISTINCT'] = true,
    ['FROM'] = true,
    ['INNER JOIN'] = true,
    ['LEFT JOIN'] = true,
    ['RIGHT JOIN'] = true,
    ['FULL JOIN'] = true,
    ['CROSS JOIN'] = true,
    ['LEFT OUTER JOIN'] = true,
    ['RIGHT OUTER JOIN'] = true,
    ['FULL OUTER JOIN'] = true,
    ['NATURAL JOIN'] = true,
    ['NATURAL LEFT JOIN'] = true,
    ['NATURAL INNER JOIN'] = true,
    ['STRAIGHT_JOIN'] = true,
    ['JOIN'] = true,
    ['WHERE'] = true,
    ['GROUP BY'] = true,
    ['HAVING'] = true,
    ['WINDOW'] = true,
    ['ORDER BY'] = true,
    ['LIMIT'] = true,
    ['OFFSET'] = true,
    ['FETCH FIRST'] = true,
    ['FETCH NEXT'] = true,
    ['FOR UPDATE'] = true,
    ['FOR SHARE'] = true,
    ['UNION'] = true,
    ['UNION ALL'] = true,
    ['UNION DISTINCT'] = true,
    ['INTERSECT'] = true,
    ['EXCEPT'] = true,
    ['INSERT INTO'] = true,
    ['INSERT OR IGNORE INTO'] = true,
    ['INSERT OR REPLACE INTO'] = true,
    ['REPLACE INTO'] = true,
    ['VALUES'] = true,
    ['ON DUPLICATE KEY UPDATE'] = true,
    ['ON CONFLICT'] = true,
    ['DO UPDATE SET'] = true,
    ['DO UPDATE'] = true,
    ['DO NOTHING'] = true,
    ['UPDATE'] = true,
    ['SET'] = true,
    ['DELETE FROM'] = true,
    ['RETURNING'] = true,
    ['CREATE TABLE'] = true,
    ['CREATE TEMPORARY TABLE'] = true,
    ['CREATE TEMP TABLE'] = true,
    ['CREATE UNLOGGED TABLE'] = true,
    ['ALTER TABLE'] = true,
    ['DROP TABLE'] = true,
    ['TRUNCATE TABLE'] = true,
    ['CREATE INDEX'] = true,
    ['CREATE UNIQUE INDEX'] = true,
    ['DROP INDEX'] = true,
    ['CREATE VIEW'] = true,
    ['CREATE OR REPLACE VIEW'] = true,
    ['DROP VIEW'] = true,
    ['CREATE DATABASE'] = true,
    ['DROP DATABASE'] = true,
    ['CREATE TRIGGER'] = true,
    ['DROP TRIGGER'] = true,
    ['USE'] = true,
    ['PRAGMA'] = true,
    ['BEGIN'] = true,
    ['START TRANSACTION'] = true,
    ['COMMIT'] = true,
    ['ROLLBACK'] = true,
  }

  local ALTER_ACTIONS = {
    ['ADD COLUMN'] = true,
    ['ADD CONSTRAINT'] = true,
    ['ADD PRIMARY KEY'] = true,
    ['ADD FOREIGN KEY'] = true,
    ['ADD UNIQUE'] = true,
    ['ADD INDEX'] = true,
    ['ADD KEY'] = true,
    ['MODIFY COLUMN'] = true,
    ['CHANGE COLUMN'] = true,
    ['ALTER COLUMN'] = true,
    ['DROP COLUMN'] = true,
    ['RENAME COLUMN'] = true,
    ['RENAME TO'] = true,
  }

  local SELECT_CLAUSES = {
    ['SELECT'] = true,
    ['SELECT DISTINCT'] = true,
  }

  local SET_CLAUSES = {
    ['SET'] = true,
    ['ON DUPLICATE KEY UPDATE'] = true,
    ['DO UPDATE SET'] = true,
  }

  local JOIN_CLAUSES = {
    ['JOIN'] = true,
    ['INNER JOIN'] = true,
    ['LEFT JOIN'] = true,
    ['RIGHT JOIN'] = true,
    ['FULL JOIN'] = true,
    ['CROSS JOIN'] = true,
    ['LEFT OUTER JOIN'] = true,
    ['RIGHT OUTER JOIN'] = true,
    ['FULL OUTER JOIN'] = true,
    ['NATURAL JOIN'] = true,
    ['NATURAL LEFT JOIN'] = true,
    ['NATURAL INNER JOIN'] = true,
    ['STRAIGHT_JOIN'] = true,
  }

  local CREATE_TABLE_CLAUSES = {
    ['CREATE TABLE'] = true,
    ['CREATE TEMPORARY TABLE'] = true,
    ['CREATE TEMP TABLE'] = true,
    ['CREATE UNLOGGED TABLE'] = true,
  }

  -- 1. Lexical tokenization
  local raw_tokens = {}
  local len = #text
  local i = 1
  while i <= len do
    local c = text:sub(i, i)
    local c2 = text:sub(i, i + 1)
    local c3 = text:sub(i, i + 2)

    if c2 == '--' or c == '#' then
      local j = text:find('\n', i, true) or (len + 1)
      table.insert(raw_tokens, { type = 'comment', val = vim.trim(text:sub(i, j - 1)) })
      i = j
    elseif c2 == '/*' then
      local j = text:find('*/', i + 2, true)
      if j then
        table.insert(raw_tokens, { type = 'comment', val = text:sub(i, j + 1) })
        i = j + 2
      else
        table.insert(raw_tokens, { type = 'comment', val = text:sub(i) })
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
          if text:sub(j + 1, j + 1) == quote then
            j = j + 2
          else
            j = j + 1
            break
          end
        else
          j = j + 1
        end
      end
      table.insert(raw_tokens, { type = 'literal', val = text:sub(i, j - 1) })
      i = j
    elseif c == '[' then
      local j = text:find(']', i + 1, true)
      if j then
        table.insert(raw_tokens, { type = 'literal', val = text:sub(i, j) })
        i = j + 1
      else
        table.insert(raw_tokens, { type = 'punct', val = c })
        i = i + 1
      end
    elseif c:match('%s') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('%s') do
        j = j + 1
      end
      table.insert(raw_tokens, { type = 'space', val = ' ' })
      i = j
    elseif c:match('%d') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('[%d%.eE]') do
        j = j + 1
      end
      table.insert(raw_tokens, { type = 'number', val = text:sub(i, j - 1) })
      i = j
    elseif (c == '@' or c == ':' or c == '$') and text:sub(i + 1, i + 1):match('[%w_@]') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('[%w_]') do
        j = j + 1
      end
      table.insert(raw_tokens, { type = 'param', val = text:sub(i, j - 1) })
      i = j
    elseif c:match('[%a_]') then
      local j = i + 1
      while j <= len and text:sub(j, j):match('[%w_]') do
        j = j + 1
      end
      table.insert(raw_tokens, { type = 'word', val = text:sub(i, j - 1) })
      i = j
    elseif c3 == '<=>' or c3 == '->>' then
      table.insert(raw_tokens, { type = 'op', val = c3 })
      i = i + 3
    elseif c2 == '>=' or c2 == '<=' or c2 == '!=' or c2 == '<>' or c2 == '||' or c2 == '&&' or c2 == '->' or c2 == ':=' then
      table.insert(raw_tokens, { type = 'op', val = c2 })
      i = i + 2
    elseif c2 == '::' then
      table.insert(raw_tokens, { type = 'cast', val = '::' })
      i = i + 2
    else
      table.insert(raw_tokens, { type = 'punct', val = c })
      i = i + 1
    end
  end

  -- Strip whitespace tokens into a dense stream (we control all spacing on output)
  local dense = {}
  for _, tok in ipairs(raw_tokens) do
    if tok.type ~= 'space' then
      table.insert(dense, tok)
    end
  end

  -- 2. Normalize multi-word and single-word SQL keywords, while protecting
  --    table.column / schema.table identifiers adjacent to `.`.
  local tokens = {}
  local idx = 1
  while idx <= #dense do
    local tok = dense[idx]
    local prev = dense[idx - 1]
    local next_tok = dense[idx + 1]
    local is_dot_qualified = (prev and prev.type == 'punct' and prev.val == '.')
      or (next_tok and next_tok.type == 'punct' and next_tok.val == '.')

    if tok.type == 'word' and not is_dot_qualified then
      local matched_kw = nil
      for _, kw in ipairs(MULTI_KEYWORDS) do
        local parts = vim.split(kw, ' ', { plain = true })
        local ok_match = true
        for p_idx, part in ipairs(parts) do
          local target = dense[idx + p_idx - 1]
          local target_prev = dense[idx + p_idx - 2]
          local target_next = dense[idx + p_idx]
          if not target or target.type ~= 'word' or target.val:upper() ~= part then
            ok_match = false
            break
          end
          if p_idx > 1 and target_prev and target_prev.type == 'punct' and target_prev.val == '.' then
            ok_match = false
            break
          end
          if p_idx == #parts and target_next and target_next.type == 'punct' and target_next.val == '.' then
            ok_match = false
            break
          end
        end
        if ok_match then
          matched_kw = kw
          idx = idx + #parts
          break
        end
      end

      if matched_kw then
        table.insert(tokens, { type = 'keyword', val = matched_kw })
      else
        local up = tok.val:upper()
        if SINGLE_KW_SET[up] then
          -- If a keyword like KEY, ORDER, GROUP, STATUS, USER, DATE, etc. is used
          -- as a column name in CREATE TABLE (e.g. right after `,` or `(` and followed by a type),
          -- keep its case unless it is a constraint starter.
          table.insert(tokens, { type = 'keyword', val = up, raw = tok.val })
        else
          table.insert(tokens, tok)
        end
        idx = idx + 1
      end
    else
      table.insert(tokens, tok)
      idx = idx + 1
    end
  end

  -- 3. Pre-match parentheses so we know whether each `(` is:
  --    - 'create_table' : column/constraint list of CREATE TABLE (...)
  --    - 'subquery'     : (SELECT ...) or (WITH ...)
  --    - 'inline'       : function args, VARCHAR(255), VALUES tuple, IN (...), expressions
  local paren_pair = {}
  local paren_stack = {}
  for t_idx, tok in ipairs(tokens) do
    if tok.type == 'punct' and tok.val == '(' then
      table.insert(paren_stack, t_idx)
    elseif tok.type == 'punct' and tok.val == ')' then
      local open_idx = table.remove(paren_stack)
      if open_idx then
        paren_pair[open_idx] = t_idx
        paren_pair[t_idx] = open_idx
      end
    end
  end

  local paren_kind = {}
  local awaiting_create_table_paren = false
  for t_idx, tok in ipairs(tokens) do
    if tok.type == 'keyword' and CREATE_TABLE_CLAUSES[tok.val] then
      awaiting_create_table_paren = true
    elseif tok.type == 'punct' and tok.val == ';' then
      awaiting_create_table_paren = false
    elseif tok.type == 'keyword' and tok.val == 'AS' and awaiting_create_table_paren then
      -- CREATE TABLE foo AS SELECT ...
      awaiting_create_table_paren = false
    elseif tok.type == 'punct' and tok.val == '(' then
      local first_inner = tokens[t_idx + 1]
      if awaiting_create_table_paren then
        paren_kind[t_idx] = 'create_table'
        awaiting_create_table_paren = false
      elseif first_inner and first_inner.type == 'keyword'
        and (SELECT_CLAUSES[first_inner.val] or first_inner.val == 'WITH' or first_inner.val == 'WITH RECURSIVE')
      then
        paren_kind[t_idx] = 'subquery'
      else
        paren_kind[t_idx] = 'inline'
      end
    end
  end

  -- Helper: check if a SELECT clause has multiple comma-separated items at its paren depth
  local function select_has_multiple_items(start_idx)
    local depth = 0
    for j = start_idx + 1, #tokens do
      local tk = tokens[j]
      if tk.type == 'punct' and tk.val == '(' then
        depth = depth + 1
      elseif tk.type == 'punct' and tk.val == ')' then
        if depth == 0 then
          return false
        end
        depth = depth - 1
      elseif depth == 0 then
        if tk.type == 'punct' and tk.val == ',' then
          return true
        elseif tk.type == 'punct' and tk.val == ';' then
          return false
        elseif tk.type == 'keyword' and TOP_CLAUSES[tk.val] then
          return false
        end
      end
    end
    return false
  end

  -- 4. Format token stream with nested context stack
  local lines = {}
  local current_line = ''
  local current_indent = 0

  local function indent_str(level)
    return string.rep('  ', math.max(0, level))
  end

  local function push_line()
    local trimmed = current_line:gsub('%s+$', '')
    if #trimmed > 0 then
      table.insert(lines, trimmed)
    end
    current_line = ''
  end

  local function start_line(level, text_part)
    push_line()
    current_indent = math.max(0, level)
    current_line = indent_str(current_indent) .. (text_part or '')
  end

  local function append_word(w)
    if #current_line == 0 then
      current_line = indent_str(current_indent) .. w
      return
    end
    local last_char = current_line:sub(-1)
    if last_char ~= ' ' and last_char ~= '(' and last_char ~= '.' then
      current_line = current_line .. ' '
    end
    current_line = current_line .. w
  end

  -- Context stack per block (top-level statement or subquery/create_table paren)
  local function new_ctx(base_indent, mode)
    return {
      base_indent = base_indent,
      mode = mode or 'statement', -- 'statement' | 'create_table' | 'subquery' | 'inline'
      clause = nil,
      in_between = false,
      case_depth = 0,
    }
  end

  local ctx_stack = { new_ctx(0, 'statement') }
  local function ctx()
    return ctx_stack[#ctx_stack]
  end

  local t_i = 1
  while t_i <= #tokens do
    local tok = tokens[t_i]
    local prev_tok = tokens[t_i - 1]
    local next_tok = tokens[t_i + 1]
    local c = ctx()
    local in_inline_paren = (c.mode == 'inline')

    if tok.type == 'comment' then
      push_line()
      table.insert(lines, indent_str(c.base_indent) .. tok.val)
    elseif tok.type == 'punct' and tok.val == '(' then
      local kind = paren_kind[t_i] or 'inline'
      if kind == 'create_table' then
        append_word('(')
        local child_indent = c.base_indent + 1
        table.insert(ctx_stack, new_ctx(child_indent, 'create_table'))
        start_line(child_indent, '')
      elseif kind == 'subquery' then
        append_word('(')
        local child_indent = c.base_indent + 1
        table.insert(ctx_stack, new_ctx(child_indent, 'subquery'))
        start_line(child_indent, '')
      else
        -- Inline parenthesis: function call `COUNT(`, type `VARCHAR(`, or grouped expr `(`
        -- Attach directly without space if preceded by a word/keyword that is a function/type name
        -- or right after another `(` or `.`.
        local no_space_before = false
        if prev_tok then
          if prev_tok.type == 'punct' and (prev_tok.val == '(' or prev_tok.val == '.') then
            no_space_before = true
          elseif prev_tok.type == 'word' then
            -- Function call or identifier before `(` unless after TABLE / INTO / JOIN / etc.
            local prev2 = tokens[t_i - 2]
            local is_table_col_list = prev2
              and prev2.type == 'keyword'
              and (prev2.val == 'INSERT INTO'
                or prev2.val == 'INSERT OR IGNORE INTO'
                or prev2.val == 'INSERT OR REPLACE INTO'
                or prev2.val == 'REPLACE INTO'
                or prev2.val == 'INTO')
            no_space_before = not is_table_col_list
          elseif prev_tok.type == 'keyword' then
            local kw = prev_tok.val
            if kw == 'COUNT' or kw == 'SUM' or kw == 'AVG' or kw == 'MIN' or kw == 'MAX'
              or kw == 'COALESCE' or kw == 'NULLIF' or kw == 'IFNULL' or kw == 'NVL'
              or kw == 'CAST' or kw == 'CONVERT' or kw == 'CONCAT' or kw == 'SUBSTRING'
              or kw == 'SUBSTR' or kw == 'TRIM' or kw == 'UPPER' or kw == 'LOWER'
              or kw == 'LENGTH' or kw == 'ROUND' or kw == 'FLOOR' or kw == 'CEIL'
              or kw == 'ABS' or kw == 'NOW' or kw == 'DATE_FORMAT' or kw == 'STRFTIME'
              or kw == 'ROW_NUMBER' or kw == 'RANK' or kw == 'DENSE_RANK' or kw == 'LAG'
              or kw == 'LEAD' or kw == 'VARCHAR' or kw == 'CHAR' or kw == 'DECIMAL'
              or kw == 'NUMERIC' or kw == 'FLOAT' or kw == 'DOUBLE' or kw == 'INT'
              or kw == 'INTEGER' or kw == 'BIGINT' or kw == 'SMALLINT' or kw == 'TINYINT'
              or kw == 'DATETIME' or kw == 'TIMESTAMP' or kw == 'TIME' or kw == 'BIT'
              or kw == 'BINARY' or kw == 'VARBINARY' or kw == 'ENUM'
            then
              no_space_before = true
            end
          end
        end

        if c.clause == 'VALUES' and prev_tok and prev_tok.type == 'keyword' and prev_tok.val == 'VALUES' then
          -- Put VALUES tuples on an indented line if multiple tuples follow
          local close_idx = paren_pair[t_i]
          local after_close = close_idx and tokens[close_idx + 1]
          if after_close and after_close.type == 'punct' and after_close.val == ',' then
            start_line(c.base_indent + 1, '(')
          else
            append_word('(')
          end
        elseif no_space_before and #current_line > 0 then
          current_line = current_line:gsub('%s+$', '') .. '('
        else
          append_word('(')
        end
        table.insert(ctx_stack, new_ctx(c.base_indent, 'inline'))
      end
    elseif tok.type == 'punct' and tok.val == ')' then
      local popped = (#ctx_stack > 1) and table.remove(ctx_stack) or c
      local parent = ctx()
      if popped.mode == 'create_table' or popped.mode == 'subquery' then
        start_line(parent.base_indent, ')')
      else
        current_line = current_line:gsub('%s+$', '') .. ')'
      end
    elseif tok.type == 'punct' and tok.val == ',' then
      current_line = current_line:gsub('%s+$', '') .. ','
      if c.mode == 'create_table' then
        start_line(c.base_indent, '')
      elseif not in_inline_paren and SELECT_CLAUSES[c.clause or ''] then
        start_line(c.base_indent + 1, '')
      elseif not in_inline_paren and SET_CLAUSES[c.clause or ''] then
        start_line(c.base_indent + 1, '')
      elseif not in_inline_paren and c.clause == 'VALUES' and next_tok and next_tok.type == 'punct' and next_tok.val == '(' then
        start_line(c.base_indent + 1, '')
      elseif not in_inline_paren and (c.clause == 'WITH' or c.clause == 'WITH RECURSIVE') then
        start_line(c.base_indent, '')
      else
        current_line = current_line .. ' '
      end
    elseif tok.type == 'punct' and tok.val == ';' then
      current_line = current_line:gsub('%s+$', '') .. ';'
      push_line()
      ctx_stack = { new_ctx(0, 'statement') }
      -- Add a blank line between separate statements
      if next_tok then
        table.insert(lines, '')
      end
    elseif tok.type == 'punct' and tok.val == '.' then
      current_line = current_line:gsub('%s+$', '') .. '.'
    elseif tok.type == 'cast' then
      current_line = current_line:gsub('%s+$', '') .. '::'
    elseif tok.type == 'op' or (tok.type == 'punct' and (tok.val == '=' or tok.val == '<' or tok.val == '>' or tok.val == '+' or tok.val == '-')) then
      append_word(tok.val)
    elseif tok.type == 'punct' and tok.val == '*' then
      if prev_tok and prev_tok.type == 'punct' and (prev_tok.val == '.' or prev_tok.val == '(') then
        current_line = current_line:gsub('%s+$', '') .. '*'
      else
        append_word('*')
      end
    elseif tok.type == 'keyword' then
      local kw = tok.val
      if not in_inline_paren and c.mode ~= 'create_table' and TOP_CLAUSES[kw] then
        c.clause = kw
        c.in_between = false
        start_line(c.base_indent, kw)
        if SELECT_CLAUSES[kw] and select_has_multiple_items(t_i) then
          start_line(c.base_indent + 1, '')
        elseif SET_CLAUSES[kw] then
          start_line(c.base_indent + 1, '')
        end
      elseif not in_inline_paren and c.mode ~= 'create_table' and ALTER_ACTIONS[kw] and c.clause == 'ALTER TABLE' then
        start_line(c.base_indent + 1, kw)
      elseif not in_inline_paren and c.mode ~= 'create_table' and kw == 'ON' and JOIN_CLAUSES[c.clause or ''] then
        c.in_between = false
        start_line(c.base_indent + 1, 'ON')
      elseif not in_inline_paren and c.mode ~= 'create_table' and (kw == 'AND' or kw == 'OR') then
        if kw == 'AND' and c.in_between then
          c.in_between = false
          append_word('AND')
        else
          start_line(c.base_indent + 1 + c.case_depth, kw)
        end
      elseif kw == 'BETWEEN' or kw == 'NOT BETWEEN' then
        c.in_between = true
        append_word(kw)
      elseif not in_inline_paren and c.mode ~= 'create_table' and kw == 'CASE' then
        append_word('CASE')
        c.case_depth = c.case_depth + 1
      elseif not in_inline_paren and c.mode ~= 'create_table' and c.case_depth > 0 and (kw == 'WHEN' or kw == 'ELSE') then
        start_line(c.base_indent + 1 + c.case_depth, kw)
      elseif not in_inline_paren and c.mode ~= 'create_table' and c.case_depth > 0 and kw == 'END' then
        c.case_depth = math.max(0, c.case_depth - 1)
        start_line(c.base_indent + 1 + c.case_depth, 'END')
      else
        append_word(kw)
      end
    else
      append_word(tok.val)
    end

    t_i = t_i + 1
  end

  push_line()
  -- Remove trailing blank lines
  while #lines > 0 and lines[#lines] == '' do
    table.remove(lines)
  end

  if #lines == 0 then
    return text
  end
  return table.concat(lines, '\n') .. '\n'
end

M.format_sql_builtin = format_sql_builtin

--- Formats the current buffer.
--- Returns the name of the formatter used, or nil when nothing could format it.
function M.format()
  local buf = vim.api.nvim_get_current_buf()
  local ft = vim.bo[buf].filetype
  if ft == '' then
    local name = vim.api.nvim_buf_get_name(buf)
    if name ~= '' then
      ft = vim.filetype.match({ filename = name }) or ''
    end
  end

  local raw_lines = vim.api.nvim_buf_get_lines(buf, 0, -1, false)
  local text = table.concat(raw_lines, '\n') .. '\n'

  local function apply_output(output)
    local normalized_out = output:gsub('\r\n', '\n'):gsub('\r', '\n')
    if normalized_out == text then
      return false
    end
    local cursor = vim.api.nvim_win_get_cursor(0)
    local lines = vim.split(normalized_out:gsub('\n$', ''), '\n', { plain = true })
    vim.api.nvim_buf_set_lines(buf, 0, -1, false, lines)
    local row = math.min(cursor[1], math.max(1, #lines))
    local max_col = math.max(0, #(lines[row] or ''))
    pcall(vim.api.nvim_win_set_cursor, 0, { row, math.min(cursor[2], max_col) })
    return true
  end

  -- 1. Check user-defined formatters first (<home>/nvim/init.lua, formatters.lua, lsp.lua).
  local user_fmt = get_user_formatter(ft)
  if user_fmt then
    if type(user_fmt) == 'function' then
      local ok, output = pcall(user_fmt, text, buf)
      if ok and type(output) == 'string' and output ~= '' then
        apply_output(output)
        return 'custom formatter'
      end
    elseif type(user_fmt) == 'table' then
      local candidates = type(user_fmt[1]) == 'table' and user_fmt or { user_fmt }
      for _, cmd in ipairs(candidates) do
        local output = run_filter(cmd, text)
        if output and output ~= '' then
          apply_output(output)
          return cmd[1]
        end
      end
    elseif type(user_fmt) == 'string' then
      local cmd = vim.split(user_fmt, '%s+')
      local output = run_filter(cmd, text)
      if output and output ~= '' then
        apply_output(output)
        return cmd[1]
      end
    end
  end

  -- 2. For non-SQL languages, prefer an attached language server that implements formatting.
  if not SQL_FILETYPES[ft] then
    for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
      if client.supports_method('textDocument/formatting') then
        vim.lsp.buf.format({ bufnr = buf, async = false, timeout_ms = 2500 })
        return client.name or 'lsp'
      end
    end
  end

  -- 3. Try candidate command-line formatters (e.g. sql-formatter, prettier, ruff, rustfmt).
  for _, command in ipairs(FORMATTERS[ft] or {}) do
    local output = run_filter(command, text)
    if output and output ~= '' then
      if apply_output(output) then
        return command[1]
      else
        return 'already formatted'
      end
    end
  end

  -- 4. Built-in multi-line SQL formatter for SQL / SQLite / MySQL / MariaDB / PostgreSQL.
  if SQL_FILETYPES[ft] then
    local output = format_sql_builtin(text)
    if output and output ~= '' then
      if apply_output(output) then
        return 'SQL formatter'
      else
        return 'already formatted'
      end
    end
  end

  return nil
end

return M
