-- Language server installer: npm packages are installed into a MindForge-owned
-- prefix (<home>/nvim/servers/npm) so nothing is written to the user's global setup.

local registry = require('mindforge.lsp.registry')
local paths = require('mindforge.lsp.paths')
local say = require('mindforge.lsp.notify').say
local progress = require('mindforge.lsp.progress')

local M = {}

local is_windows = vim.fn.has('win32') == 1
local installing = {}

function M.npm_prefix()
  return paths.npm_prefix()
end

--- Command line for npm. On Windows `npm.cmd` goes through cmd.exe, which breaks
--- on paths containing spaces ("C:\Program Files"), so run npm-cli.js with node.
local function npm_command(args)
  local npm = vim.fn.exepath('npm')
  if npm == '' then
    return nil
  end
  local cmd = { npm }
  if is_windows then
    local cli = vim.fs.dirname(npm) .. '/node_modules/npm/bin/npm-cli.js'
    local node = vim.fn.exepath('node')
    if node ~= '' and vim.uv.fs_stat(cli) then
      cmd = { node, cli }
    end
  end
  return vim.list_extend(cmd, args)
end

--- Creates the managed prefix plus a package.json so npm treats it as its own
--- project and never walks up to another one.
local function prepare_prefix()
  local prefix = paths.npm_prefix()
  if not paths.ensure(prefix) then
    return nil, 'cannot create ' .. prefix
  end
  local manifest = prefix .. '/package.json'
  if not vim.uv.fs_stat(manifest) then
    pcall(vim.fn.writefile, { '{ "name": "mindforge-lsp-servers", "private": true }' }, manifest)
  end
  return prefix
end

local function tail(text)
  text = vim.trim(text or ''):gsub('%s+', ' ')
  return #text > 160 and ('...' .. text:sub(-160)) or text
end

--- Absolute path of the server executable, or nil when it is not installed.
function M.resolve(spec)
  if spec.rustup then
    local rustup = vim.fn.exepath('rustup')
    if rustup ~= '' then
      local ok, res = pcall(function()
        return vim.system({ rustup, 'which', spec.bin }, { text = true }):wait()
      end)
      if ok and res and res.code == 0 and res.stdout and vim.trim(res.stdout) ~= '' then
        local p = vim.trim(res.stdout)
        if vim.uv.fs_stat(p) then
          return p
        end
      end
      return nil
    end
    local found = vim.fn.exepath(spec.bin)
    return found ~= '' and found or nil
  end
  local managed = paths.npm_prefix() .. '/node_modules/.bin/' .. spec.bin .. (is_windows and '.cmd' or '')
  if vim.uv.fs_stat(managed) then
    return managed
  end
  local found = vim.fn.exepath(spec.bin)
  return found ~= '' and found or nil
end

--- Command prefix that starts the server. On Windows npm installs `.cmd` shims
--- which libuv cannot spawn directly and which add a cmd.exe hop, so the shim is
--- resolved to `node <script>` instead.
function M.command(spec)
  local exe = M.resolve(spec)
  if not exe then
    return nil
  end
  if is_windows and exe:lower():match('%.cmd$') then
    local lines = vim.fn.readfile(exe)
    local script
    for _, line in ipairs(lines) do
      script = line:match('"%%dp0%%[\\/]+([^"]+%.[mc]?js)"') or script
    end
    local node = vim.fn.exepath('node')
    if script and node ~= '' then
      local dir = vim.fs.dirname(exe)
      return { node, dir .. '/' .. script:gsub('\\', '/') }
    end
  end
  return { exe }
end

--- Folder holding tsserver.js. typescript-language-server needs it but only
--- searches the workspace, not our managed prefix, so it is passed explicitly.
function M.typescript_lib(spec)
  local candidates = { paths.npm_prefix() .. '/node_modules/typescript/lib' }
  local exe = M.resolve(spec)
  if exe then
    table.insert(candidates, vim.fs.dirname(exe) .. '/node_modules/typescript/lib')
    table.insert(candidates, vim.fs.dirname(exe) .. '/../typescript/lib')
  end
  for _, dir in ipairs(candidates) do
    if vim.uv.fs_stat(dir .. '/tsserver.js') then
      return vim.fs.normalize(dir)
    end
  end
end

function M.is_installed(name)
  local spec = registry.servers[name]
  return spec ~= nil and M.resolve(spec) ~= nil
end

local function finish(name, ok, detail, on_done)
  installing[name] = nil
  progress.stop('install:' .. name)
  if ok then
    say(name .. ' installed')
  else
    say(name .. ' install failed' .. (detail and detail ~= '' and (': ' .. detail) or ''))
  end
  if on_done then
    on_done(ok)
  end
end

--- Installs a registry server asynchronously.
function M.install(name, on_done)
  local spec = registry.servers[name]
  if not spec then
    return say('unknown server "' .. tostring(name) .. '". Try :LspList')
  end
  if installing[name] then
    return say(name .. ' is already installing')
  end

  local cmd, cwd
  if spec.rustup then
    local rustup = vim.fn.exepath('rustup')
    if rustup == '' then
      return say('rustup is required to install ' .. name .. ' (https://rustup.rs)')
    end
    cmd = { rustup, 'component', 'add', spec.rustup }
  else
    local prefix, problem = prepare_prefix()
    if not prefix then
      return say(problem)
    end
    local args = { 'install', '--prefix', prefix, '--no-audit', '--no-fund', '--loglevel=error' }
    cmd = npm_command(vim.list_extend(args, spec.npm))
    if not cmd then
      return say('Node.js / npm is required to install ' .. name .. ' (https://nodejs.org)')
    end
    cwd = prefix
  end

  installing[name] = true
  progress.start('install:' .. name, 'Installing ' .. name)
  say('installing ' .. name .. ' ...')
  local ok, err = pcall(vim.system, cmd, { text = true, cwd = cwd }, vim.schedule_wrap(function(result)
    local success = result.code == 0 and M.is_installed(name)
    local detail = not success and tail((result.stderr or '') .. ' ' .. (result.stdout or '')) or nil
    finish(name, success, detail, on_done)
  end))
  if not ok then
    finish(name, false, tostring(err), on_done)
  end
end

function M.uninstall(name, on_done)
  local spec = registry.servers[name]
  if not spec or spec.rustup then
    return say(name .. ' is not managed by MindForge (use rustup / your package manager)')
  end
  local prefix = paths.npm_prefix()
  if not vim.uv.fs_stat(prefix) then
    return say(name .. ' is not installed')
  end
  local args = { 'uninstall', '--prefix', prefix, '--no-audit', '--no-fund', '--loglevel=error' }
  local cmd = npm_command(vim.list_extend(args, spec.npm))
  if not cmd then
    return say('npm is required to uninstall ' .. name)
  end
  vim.system(cmd, { text = true, cwd = prefix }, vim.schedule_wrap(function(result)
    say(name .. (result.code == 0 and ' removed' or ' uninstall failed'))
    if on_done then
      on_done()
    end
  end))
end

function M.is_installing(name)
  return installing[name] == true
end

return M
