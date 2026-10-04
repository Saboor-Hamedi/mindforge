-- VS Code style completion: as-you-type triggering, accept/navigate keys and
-- diagnostics. The popup itself is drawn by MindForge from ext_popupmenu.

local M = {}

local function has_completion_client(buf)
  return #vim.lsp.get_clients({ bufnr = buf, method = 'textDocument/completion' }) > 0
end

local function enable_buffer_completion(buf)
  if not vim.api.nvim_buf_is_valid(buf) then
    return false
  end
  local completion = vim.lsp.completion
  if not completion or type(completion.enable) ~= 'function' then
    return false
  end
  local found = false
  for _, client in ipairs(vim.lsp.get_clients({ bufnr = buf })) do
    local ok, supported = pcall(client.supports_method, client, 'textDocument/completion')
    if ok and supported then
      found = true
      local enabled, err = pcall(completion.enable, true, client.id, buf, { autotrigger = true })
      if not enabled then
        vim.notify('MindForge could not enable completion: ' .. tostring(err), vim.log.levels.WARN)
      end
    end
  end
  return found
end

local function request_completion()
  local buf = vim.api.nvim_get_current_buf()
  enable_buffer_completion(buf)
  if not has_completion_client(buf) then
    return false
  end
  local completion = vim.lsp.completion
  if not completion or type(completion.get) ~= 'function' then
    return false
  end
  local ok, err = pcall(completion.get)
  if not ok then
    vim.notify('MindForge completion request failed: ' .. tostring(err), vim.log.levels.WARN)
  end
  return ok
end

local function feed(keys)
  vim.api.nvim_feedkeys(vim.api.nvim_replace_termcodes(keys, true, false, true), 'n', false)
end

local function pum_selected()
  return vim.fn.complete_info({ 'selected' }).selected
end

local function map(modes, lhs, rhs, opts)
  vim.keymap.set(modes, lhs, rhs, vim.tbl_extend('force', { silent = true }, opts or {}))
end

local function setup_options()
  -- Keep matches unselected while typing. The custom popup accepts the exact
  -- item on click, so users do not need a keyboard selection just to accept it.
  vim.opt.completeopt = { 'menu', 'menuone', 'noinsert', 'noselect', 'fuzzy' }
  vim.opt.pumheight = 12
  vim.opt.shortmess:append('c')
  vim.diagnostic.config({
    -- Diagnostic messages are metadata; rendering them as virtual text can
    -- extend/wrap Neovim grid rows. MindForge renders extmark underlines as an
    -- overlay and keeps the complete message available through diagnostic float.
    virtual_text = false,
    signs = false,
    underline = true,
    update_in_insert = false,
    severity_sort = true,
  })
end

local function setup_triggers(group)
  vim.api.nvim_create_autocmd('LspAttach', {
    group = group,
    callback = function(event)
      enable_buffer_completion(event.buf)
    end,
  })
  vim.api.nvim_create_autocmd({ 'BufEnter', 'FileType' }, {
    group = group,
    callback = function(event)
      vim.schedule(function()
        local buf = event.buf
        if buf == vim.api.nvim_get_current_buf() then
          enable_buffer_completion(buf)
        end
      end)
    end,
  })

  -- Servers only auto-trigger on punctuation; also offer completion while
  -- typing identifiers. The debounce keeps fast typing free of extra requests.
  local timer = vim.uv.new_timer()
  vim.api.nvim_create_autocmd('InsertCharPre', {
    group = group,
    callback = function()
      if not vim.v.char:match('[%w_!]') then
        return
      end
      local buf = vim.api.nvim_get_current_buf()
      if not has_completion_client(buf) then
        return
      end
      timer:stop()
      timer:start(60, 0, vim.schedule_wrap(function()
        if vim.api.nvim_get_mode().mode:sub(1, 1) == 'i' and vim.api.nvim_get_current_buf() == buf then
          vim.cmd('redraw') -- show the typed text and caret before any completion work
          pcall(vim.lsp.completion.get)
        end
      end))
    end,
  })
end

local function setup_keys()
  map('i', '<Tab>', function()
    if vim.fn.pumvisible() == 1 then
      feed(pum_selected() >= 0 and '<C-y>' or '<C-n><C-y>')
    elseif vim.snippet.active({ direction = 1 }) then
      vim.snippet.jump(1)
    elseif request_completion() then
      -- With a completion-capable client, Tab explicitly requests matches.
    else
      feed('<Tab>')
    end
  end)
  map({ 'i', 's' }, '<S-Tab>', function()
    if vim.fn.pumvisible() == 1 then
      feed('<C-p>')
    elseif vim.snippet.active({ direction = -1 }) then
      vim.snippet.jump(-1)
    else
      feed('<S-Tab>')
    end
  end)
  map('s', '<Tab>', function()
    if vim.snippet.active({ direction = 1 }) then
      vim.snippet.jump(1)
    else
      feed('<Tab>')
    end
  end)
  map('i', '<CR>', function()
    if vim.fn.pumvisible() == 1 then
      if pum_selected() >= 0 then
        return '<C-y>'
      end
      -- Nothing highlighted: accept an Emmet-style abbreviation when the menu
      -- has a result. LSP completion `word` can be the expansion, not the typed
      -- abbreviation, so matching it against the line incorrectly misses valid items.
      local first = vim.fn.complete_info({ 'items' }).items[1]
      local before = vim.api.nvim_get_current_line():sub(1, vim.fn.col('.') - 1)
      if first and before:match('[%w_]+[>+*#%.].*$') then
        return '<C-n><C-y>'
      end
      return '<C-e><CR>'
    end
    local col = vim.fn.col('.')
    local line = vim.fn.getline('.')
    if line:sub(col - 1, col - 1) == '{' and line:sub(col, col) == '}' then
      return '<CR><Esc>O'
    end
    return '<CR>'
  end, { expr = true })
  map('i', '<Esc>', function()
    return vim.fn.pumvisible() == 1 and '<C-e>' or '<Esc>'
  end, { expr = true })
  map('i', '<C-Space>', request_completion)
  map('n', 'gd', vim.lsp.buf.definition)
end

--- `group` is the shared augroup; `on_filetype(buf)` runs for every FileType.
function M.setup(group, on_filetype)
  setup_options()
  -- Load the lazy completion modules now so the first keystroke does not pay for it.
  vim.schedule(function()
    pcall(require, 'vim.lsp.completion')
    pcall(function() return vim.snippet end)
  end)
  setup_triggers(group)
  setup_keys()
  vim.api.nvim_create_autocmd('FileType', {
    group = group,
    callback = function(event)
      on_filetype(event.buf)
    end,
  })
end

return M
