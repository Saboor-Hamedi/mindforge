-- User commands: :LspInstall :LspUninstall :LspList :LspInfo :LspRestart :LspConfig

local registry = require('mindforge.lsp.registry')
local installer = require('mindforge.lsp.install')
local paths = require('mindforge.lsp.paths')
local say = require('mindforge.lsp.notify').say

local M = {}

local function server_names()
  return registry.names()
end

--- Sends every server with its status to MindForge's list panel.
local function push_list(open, filter)
  local rows = {}
  for _, name in ipairs(registry.names()) do
    local spec = registry.servers[name]
    local status = 'available'
    if installer.is_installing(name) then
      status = 'installing'
    elseif installer.is_installed(name) then
      status = #vim.lsp.get_clients({ name = name }) > 0 and 'running' or 'installed'
    end
    rows[#rows + 1] = { name, status, spec.desc or '', table.concat(spec.config.filetypes or {}, ' ') }
  end
  return pcall(vim.rpcnotify, 0, 'mindforge_lsp_list', rows, open and true or false, filter or '')
end

--- `refresh` re-registers servers after an install changes what can run.
function M.setup(refresh)
  local cmd = vim.api.nvim_create_user_command

  cmd('LspInstall', function(opts)
    local names = opts.fargs
    if #names == 0 then
      names = registry.for_filetype(vim.bo.filetype)
    end
    if #names == 0 then
      return say('no known server for filetype "' .. vim.bo.filetype .. '". Try :LspList')
    end
    for _, name in ipairs(names) do
      installer.install(name, function(ok)
        if ok then
          refresh()
        end
        push_list(false)
      end)
    end
    push_list(false)
  end, { nargs = '*', complete = server_names, desc = 'Install language servers' })

  cmd('LspUninstall', function(opts)
    for _, name in ipairs(opts.fargs) do
      installer.uninstall(name, function()
        pcall(vim.lsp.enable, name, false)
        push_list(false)
      end)
    end
  end, { nargs = '+', complete = server_names, desc = 'Uninstall language servers' })

  cmd('LspList', function(opts)
    if not push_list(true, opts.args) then
      local installed = {}
      for _, name in ipairs(registry.names()) do
        if installer.is_installed(name) then
          table.insert(installed, name)
        end
      end
      say(#installed .. ' installed of ' .. #registry.names() .. ': ' .. table.concat(installed, ', '))
    end
  end, { nargs = '?', desc = 'Browse language servers' })

  cmd('LspInfo', function()
    local names = {}
    for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
      table.insert(names, client.name)
    end
    say('attached: ' .. (#names > 0 and table.concat(names, ', ') or 'none') .. '  (filetype ' .. vim.bo.filetype .. ')')
  end, { desc = 'Show language servers attached to this buffer' })

  cmd('LspRestart', function()
    for _, client in ipairs(vim.lsp.get_clients({ bufnr = 0 })) do
      client:stop()
    end
    vim.defer_fn(function()
      vim.api.nvim_exec_autocmds('FileType', { buffer = 0 })
    end, 600)
  end, { desc = 'Restart language servers for this buffer' })

  cmd('LspConfig', function()
    paths.ensure(paths.nvim_dir())
    vim.cmd('edit ' .. vim.fn.fnameescape(paths.user_config()))
  end, { desc = 'Open your MindForge LSP configuration' })
end

return M
