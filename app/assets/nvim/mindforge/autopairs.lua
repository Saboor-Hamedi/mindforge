-- MindForge auto-pairs: robust insert-mode bracket / quote pairing.
--
-- Why a dedicated module (instead of inline mappings):
--   * Works in every filetype, including SQL where `'` is a string quote.
--   * Context-aware: skips over an existing closer, never pairs inside words,
--     respects a preceding backslash, and can be disabled per-buffer.
--   * `<BS>` deletes both halves of an empty pair, `<CR>` expands an empty pair
--     into a block, and typing a closing bracket over an auto-inserted one just
--     moves the cursor instead of duplicating it.
--   * Re-runnable via `M.setup()` so MindForge can re-assert it after the user's
--     init.lua is (re)sourced, while still letting the user opt out.

local M = {}

local OPEN = { ['('] = ')', ['['] = ']', ['{'] = '}' }
local CLOSE = { [')'] = '(', [']'] = '[', ['}'] = '{' }
local QUOTE = { ['"'] = true, ["'"] = true, ['`'] = true }

local function is_enabled(buf)
  if vim.g.mindforge_autopair == false then
    return false
  end
  return vim.b[buf].mindforge_autopair ~= false
end

local function line_at()
  return vim.api.nvim_get_current_line()
end

-- Byte index just before the insert cursor, and the byte at it.
local function char_before()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  if col < 1 then
    return ''
  end
  return line_at():sub(col, col)
end

local function char_after()
  local col = vim.api.nvim_win_get_cursor(0)[2]
  local line = line_at()
  if col + 1 > #line then
    return ''
  end
  return line:sub(col + 1, col + 1)
end

local function is_word_char(ch)
  return ch ~= '' and ch:match('[%w_]') ~= nil
end

-- Insert-mode expr mapping. `replace_keycodes` turns the returned `<Left>` /
-- `<Right>` tokens into real keys.
local function imap(lhs, fn)
  vim.keymap.set('i', lhs, fn, {
    expr = true,
    noremap = true,
    silent = true,
    replace_keycodes = true,
    desc = 'MindForge auto-pair',
  })
end

local function insert_opener(open, close)
  imap(open, function()
    if not is_enabled(0) then
      return open
    end
    -- Already sitting on the closer (e.g. re-typing) -> just step over it.
    if char_after() == close then
      return '<Right>'
    end
    return open .. close .. '<Left>'
  end)
end

local function insert_closer(close)
  imap(close, function()
    if not is_enabled(0) then
      return close
    end
    if char_after() == close then
      return '<Right>'
    end
    return close
  end)
end

local function insert_quote(quote)
  imap(quote, function()
    if not is_enabled(0) then
      return quote
    end
    local prev, next = char_before(), char_after()
    -- Step over an existing quote.
    if next == quote then
      return '<Right>'
    end
    -- Escaped quote, or quote adjacent to a word (contraction / identifier).
    if prev == '\\' or is_word_char(prev) or is_word_char(next) then
      return quote
    end
    return quote .. quote .. '<Left>'
  end)
end

function M.setup()
  for open, close in pairs(OPEN) do
    insert_opener(open, close)
  end
  for close in pairs(CLOSE) do
    insert_closer(close)
  end
  for quote in pairs(QUOTE) do
    insert_quote(quote)
  end

  -- Backspace removes both halves of an empty pair.
  imap('<BS>', function()
    if not is_enabled(0) then
      return '<BS>'
    end
    local prev, next = char_before(), char_after()
    if OPEN[prev] == next then
      return '<Right><BS><BS>'
    end
    if QUOTE[prev] and next == prev then
      return '<Right><BS><BS>'
    end
    return '<BS>'
  end)

  -- Enter: handles completion popup, bracket expansion, and auto-bullet/numbered list continuation.
  -- Importantly, when the completion popup is open with NO item selected (`noselect`, selected == -1),
  -- pressing Enter closes the popup (`<C-e>`) and inserts a real newline instead of swallowing `<CR>`
  -- or accidentally accepting an unwanted completion.
  imap('<CR>', function()
    local pum_prefix = ''
    if vim.fn.pumvisible() == 1 then
      local info = vim.fn.complete_info({ 'selected' })
      if info and (info.selected or -1) >= 0 then
        return '<C-y>'
      end
      pum_prefix = '<C-e>'
    end

    if vim.snippet and vim.snippet.active and vim.snippet.active() and not vim.snippet.active({ direction = 1 }) then
      pcall(vim.snippet.stop)
    end

    if not is_enabled(0) then
      return pum_prefix .. '<CR>'
    end

    local prev, next = char_before(), char_after()
    if (prev == '{' and next == '}')
      or (prev == '(' and next == ')')
      or (prev == '[' and next == ']')
    then
      return pum_prefix .. '<CR><Esc>O'
    end

    local line = line_at()
    local col = vim.api.nvim_win_get_cursor(0)[2]

    -- Check if line is an empty bullet/numbered list item (user wants to exit the list)
    if line:match('^[ \t]*[-*+][ \t]+%[[ xX]%][ \t]*$')
      or line:match('^[ \t]*[-*+][ \t]*$')
      or line:match('^[ \t]*%d+%.[ \t]*$')
      or line:match('^[ \t]*>[ \t]*$')
    then
      return pum_prefix .. '<C-u>'
    end

    -- Check for markdown / note list continuation
    -- 1) Task checkbox: `- [ ] ` or `* [ ] `
    local t_indent, t_bullet = line:match('^([ \t]*)([-*+])[ \t]+%[[ xX]%][ \t]+')
    if t_indent and t_bullet then
      if col < #line then
        return pum_prefix .. '<CR>' .. t_bullet .. ' [ ] '
      else
        return pum_prefix .. '<CR><C-u>' .. t_indent .. t_bullet .. ' [ ] '
      end
    end

    -- 2) Numbered list: `1. `, `2. `, etc.
    local n_indent, n_str = line:match('^([ \t]*)(%d+)%.[ \t]+')
    if n_indent and n_str then
      local next_num = tostring(tonumber(n_str) + 1)
      if col < #line then
        return pum_prefix .. '<CR>' .. next_num .. '. '
      else
        return pum_prefix .. '<CR><C-u>' .. n_indent .. next_num .. '. '
      end
    end

    -- 3) Bullet list: `- `, `* `, `+ `
    local b_indent, b_char = line:match('^([ \t]*)([-*+])[ \t]+')
    if b_indent and b_char then
      if col < #line then
        return pum_prefix .. '<CR>' .. b_char .. ' '
      else
        return pum_prefix .. '<CR><C-u>' .. b_indent .. b_char .. ' '
      end
    end

    -- 4) Blockquote: `> `
    local q_indent, q_prefix = line:match('^([ \t]*)(>+[ \t]*)')
    if q_indent and q_prefix then
      if col < #line then
        return pum_prefix .. '<CR>' .. q_prefix
      else
        return pum_prefix .. '<CR><C-u>' .. q_indent .. q_prefix
      end
    end

    return pum_prefix .. '<CR>'
  end)
end

return M
