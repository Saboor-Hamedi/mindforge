-- VS Code style completion: as-you-type triggering, accept/navigate keys and
-- diagnostics. The popup itself is drawn by MindForge from ext_popupmenu.

local M = {}

local function has_completion_client(buf)
  return #vim.lsp.get_clients({ bufnr = buf, method = 'textDocument/completion' }) > 0
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
  vim.opt.completeopt = { 'menu', 'menuone', 'noinsert', 'noselect', 'fuzzy' }
  vim.opt.pumheight = 12
  vim.opt.shortmess:append('c')
  vim.diagnostic.config({
    virtual_text = { prefix = '●', spacing = 2 },
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
      local client = vim.lsp.get_client_by_id(event.data.client_id)
      if client and client:supports_method('textDocument/completion') then
        vim.lsp.completion.enable(true, client.id, event.buf, { autotrigger = true })
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
      return pum_selected() >= 0 and '<C-y>' or '<C-e><CR>'
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
  map('i', '<C-Space>', function()
    pcall(vim.lsp.completion.get)
  end)
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
