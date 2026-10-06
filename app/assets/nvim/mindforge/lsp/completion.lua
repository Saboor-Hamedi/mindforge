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
  pcall(vim.api.nvim_set_hl, 0, 'SnippetTabstop', { underline = true, default = false })
  pcall(vim.api.nvim_set_hl, 0, 'SnippetActiveTabstop', { underline = true, bold = true, default = false })
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

  -- Cancel active snippet session whenever leaving insert/select mode,
  -- matching VS Code's behavior so placeholder highlights never linger.
  vim.api.nvim_create_autocmd('ModeChanged', {
    group = group,
    pattern = { '[is]:n', '[is]:v', '[is]:V', '[is]:\x16' },
    callback = function()
      if vim.snippet and vim.snippet.active and vim.snippet.active() then
        pcall(vim.snippet.stop)
      end
    end,
  })
  vim.api.nvim_create_autocmd('BufLeave', {
    group = group,
    callback = function()
      if vim.snippet and vim.snippet.active and vim.snippet.active() then
        pcall(vim.snippet.stop)
      end
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
    elseif vim.snippet.active() then
      pcall(vim.snippet.stop)
      feed('<Tab>')
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
    elseif vim.snippet.active() then
      pcall(vim.snippet.stop)
      feed('<Tab>')
    else
      feed('<Tab>')
    end
  end)
  local function close_floating_windows()
    local closed = false
    for _, win in ipairs(vim.api.nvim_tabpage_list_wins(0)) do
      local config = vim.api.nvim_win_get_config(win)
      if config.relative and config.relative ~= '' then
        pcall(vim.api.nvim_win_close, win, true)
        closed = true
      end
    end
    return closed
  end

  local function safe_rename()
    local clients = vim.lsp.get_clients({ bufnr = 0 })
    if #clients == 0 then
      vim.notify('LSP server is not running for this buffer', vim.log.levels.WARN)
      return
    end
    local supported = false
    for _, client in ipairs(clients) do
      if client.supports_method('textDocument/rename') then
        supported = true
        break
      end
    end
    if not supported then
      vim.notify('Rename is not supported by the active language server for this filetype', vim.log.levels.INFO)
      return
    end
    vim.lsp.buf.rename()
  end

  local function safe_definition()
    local clients = vim.lsp.get_clients({ bufnr = 0 })
    if #clients == 0 then
      vim.notify('LSP server is not running for this buffer', vim.log.levels.WARN)
      return
    end
    vim.lsp.buf.definition()
  end

  local function safe_references()
    local clients = vim.lsp.get_clients({ bufnr = 0 })
    if #clients == 0 then
      vim.notify('LSP server is not running for this buffer', vim.log.levels.WARN)
      return
    end
    vim.lsp.buf.references()
  end

  map('i', '<CR>', function()
    if vim.fn.pumvisible() == 1 then
      if pum_selected() >= 0 then
        return '<C-y>'
      end
      return '<C-n><C-y>'
    end
    if vim.snippet and vim.snippet.active and vim.snippet.active() and not vim.snippet.active({ direction = 1 }) then
      pcall(vim.snippet.stop)
    end
    return '<CR>'
  end, { expr = true })
  map('i', '<Esc>', function()
    if vim.snippet and vim.snippet.active and vim.snippet.active() then
      pcall(vim.snippet.stop)
    end
    return vim.fn.pumvisible() == 1 and '<C-e><Esc>' or '<Esc>'
  end, { expr = true })
  map('s', '<Esc>', function()
    if vim.snippet and vim.snippet.active and vim.snippet.active() then
      pcall(vim.snippet.stop)
    end
    return '<Esc>'
  end, { expr = true })
  map('n', '<Esc>', function()
    vim.cmd('nohlsearch')
    if vim.w._mindforge_live_search_id then
      pcall(vim.fn.matchdelete, vim.w._mindforge_live_search_id)
      vim.w._mindforge_live_search_id = nil
    end
    if vim.snippet and vim.snippet.stop then
      pcall(vim.snippet.stop)
    end
    close_floating_windows()
  end)
  map('i', '<C-Space>', request_completion)
  map('n', 'gd', safe_definition)
  map('n', 'K', vim.lsp.buf.hover)
  map('n', 'gr', safe_references)
  map('n', '<F2>', safe_rename)
  map('n', '<leader>rn', safe_rename)
  map({ 'n', 'v' }, '<M-CR>', vim.lsp.buf.code_action)
  map({ 'n', 'v' }, '<leader>ca', vim.lsp.buf.code_action)
  map({ 'n', 'v' }, '<leader>f', function() vim.lsp.buf.format({ async = true }) end)
  map({ 'n', 'v' }, '<A-F>', function() vim.lsp.buf.format({ async = true }) end)
  map('n', '<leader>d', vim.diagnostic.open_float)
  map('n', '[d', vim.diagnostic.goto_prev)
  map('n', ']d', vim.diagnostic.goto_next)
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
  vim.api.nvim_create_autocmd('BufEnter', {
    group = group,
    callback = function(event)
      local win = vim.api.nvim_get_current_win()
      local config = vim.api.nvim_win_get_config(win)
      if config.relative and config.relative ~= '' then
        vim.keymap.set('n', '<Esc>', '<cmd>close<CR>', { buffer = event.buf, silent = true, nowait = true })
        vim.keymap.set('n', 'q', '<cmd>close<CR>', { buffer = event.buf, silent = true, nowait = true })
      end
    end,
  })
end

return M
