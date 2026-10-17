-- Built-in language server catalogue.
--
-- Every entry describes how to install a server and how Neovim should start
-- it. Users can override or extend any entry from
-- %APPDATA%\mindforge\mindforge\nvim\lsp.lua (see brain/lsp.txt).

local M = {}

-- `npm`    : packages installed into <home>/nvim/servers/npm
-- `bin`    : executable name (resolved from the managed npm dir, then PATH)
-- `config` : fields forwarded to vim.lsp.config()
M.servers = {
  html = {
    desc = 'HTML (tags, attributes, embedded CSS/JS)',
    npm = { 'vscode-langservers-extracted' },
    bin = 'vscode-html-language-server',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'html' },
      init_options = {
        provideFormatter = true,
        embeddedLanguages = { css = true, javascript = true },
        configurationSection = { 'html', 'css', 'javascript' },
      },
    },
  },
  cssls = {
    desc = 'CSS / SCSS / Less',
    npm = { 'vscode-langservers-extracted' },
    bin = 'vscode-css-language-server',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'css', 'scss', 'less' },
      init_options = { provideFormatter = true },
    },
  },
  jsonls = {
    desc = 'JSON',
    npm = { 'vscode-langservers-extracted' },
    bin = 'vscode-json-language-server',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'json', 'jsonc' },
      init_options = { provideFormatter = true },
    },
  },
  eslint = {
    desc = 'ESLint diagnostics for JS / TS / JSX',
    npm = { 'vscode-langservers-extracted' },
    bin = 'vscode-eslint-language-server',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'javascript', 'javascriptreact', 'typescript', 'typescriptreact' },
      -- Only runs inside a project that actually configures ESLint; elsewhere
      -- the server has no workspace and fails every diagnostic request.
      root_dir = function(bufnr, on_dir)
        local found = vim.fs.root(bufnr, {
          'eslint.config.js', 'eslint.config.mjs', 'eslint.config.cjs', 'eslint.config.ts',
          '.eslintrc', '.eslintrc.json', '.eslintrc.js', '.eslintrc.cjs', '.eslintrc.yml', '.eslintrc.yaml',
        })
        if found and vim.uv.fs_stat(found .. '/node_modules/eslint') then
          on_dir(found)
        end
      end,
      on_new_config = nil,
      before_init = function(_, config)
        config.settings = vim.tbl_deep_extend('force', config.settings or {}, {
          workspaceFolder = { uri = vim.uri_from_fname(config.root_dir), name = vim.fn.fnamemodify(config.root_dir, ':t') },
        })
      end,
      settings = { validate = 'on', run = 'onType', nodePath = vim.NIL, workingDirectory = { mode = 'auto' } },
    },
  },
  ts_ls = {
    desc = 'JavaScript / TypeScript / JSX / TSX',
    -- typescript 7 ships no tsserver.js, which the language server requires
    npm = { 'typescript-language-server', 'typescript@5' },
    bin = 'typescript-language-server',
    tsserver = true,
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'javascript', 'javascriptreact', 'typescript', 'typescriptreact' },
      root_markers = { 'tsconfig.json', 'jsconfig.json', 'package.json', '.git' },
    },
  },
  emmet_ls = {
    desc = 'Emmet abbreviations for CSS',
    npm = { 'emmet-ls' },
    bin = 'emmet-ls',
    config = {
      cmd_args = { '--stdio' },
      -- HTML / JSX / TSX abbreviations are built in (see emmet.lua).
      filetypes = { 'css', 'scss', 'less' },
    },
  },
  tailwindcss = {
    desc = 'Tailwind CSS class completion',
    npm = { '@tailwindcss/language-server' },
    bin = 'tailwindcss-language-server',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'html', 'css', 'javascriptreact', 'typescriptreact' },
      root_markers = { 'tailwind.config.js', 'tailwind.config.ts', 'package.json', '.git' },
    },
  },
  svelte = {
    desc = 'Svelte',
    npm = { 'svelte-language-server' },
    bin = 'svelteserver',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'svelte' },
      root_markers = { 'svelte.config.js', 'package.json', '.git' },
    },
  },
  bashls = {
    desc = 'Bash / shell scripts',
    npm = { 'bash-language-server' },
    bin = 'bash-language-server',
    config = { cmd_args = { 'start' }, filetypes = { 'sh', 'bash' } },
  },
  dockerls = {
    desc = 'Dockerfile',
    npm = { 'dockerfile-language-server-nodejs' },
    bin = 'docker-langserver',
    config = { cmd_args = { '--stdio' }, filetypes = { 'dockerfile' } },
  },
  pyright = {
    desc = 'Python',
    npm = { 'pyright' },
    bin = 'pyright-langserver',
    config = {
      cmd_args = { '--stdio' },
      filetypes = { 'python' },
      root_markers = { 'pyproject.toml', 'setup.py', 'requirements.txt', 'pyrightconfig.json', '.git' },
    },
  },
  yamlls = {
    desc = 'YAML',
    npm = { 'yaml-language-server' },
    bin = 'yaml-language-server',
    config = { cmd_args = { '--stdio' }, filetypes = { 'yaml' } },
  },
  intelephense = {
    desc = 'PHP',
    npm = { 'intelephense' },
    bin = 'intelephense',
    config = { cmd_args = { '--stdio' }, filetypes = { 'php' }, root_markers = { 'composer.json', '.git' } },
  },
  rust_analyzer = {
    desc = 'Rust (installed through rustup)',
    rustup = 'rust-analyzer',
    bin = 'rust-analyzer',
    config = { filetypes = { 'rust' }, root_markers = { 'Cargo.toml', 'rust-project.json', '.git' } },
  },
  sql = {
    desc = 'SQL (SQLite / PostgreSQL / MySQL / MariaDB)',
    npm = { 'sql-language-server', 'sql-formatter' },
    bin = 'sql-language-server',
    config = {
      cmd_args = { 'up', '--method=stdio' },
      filetypes = { 'sql', 'mysql', 'mariadb', 'sqlite', 'pgsql', 'plsql' },
    },
  },
  sql_formatter = {
    desc = 'SQL Formatter (sql-formatter CLI)',
    npm = { 'sql-formatter' },
    bin = 'sql-formatter',
    config = {
      filetypes = { 'sql', 'mysql', 'mariadb', 'sqlite', 'pgsql', 'plsql' },
    },
  },
  clangd = {
    desc = 'C / C++ / Objective-C (clangd)',
    bin = 'clangd',
    scoop = 'llvm',
    winget = 'LLVM.LLVM',
    config = {
      cmd_args = { '--background-index', '--clang-tidy', '--header-insertion=iwyu' },
      filetypes = { 'c', 'cpp', 'objc', 'objcpp', 'cuda' },
      root_markers = { 'compile_commands.json', 'compile_flags.txt', '.clangd', 'CMakeLists.txt', '.git' },
    },
  },
  lua_ls = {
    desc = 'Lua & Neovim init.lua (lua-language-server)',
    bin = 'lua-language-server',
    win_zip = 'https://github.com/LuaLS/lua-language-server/releases/download/3.13.6/lua-language-server-3.13.6-win32-x64.zip',
    scoop = 'lua-language-server',
    winget = 'LuaLS.lua-language-server',
    config = {
      filetypes = { 'lua' },
      root_markers = { '.luarc.json', '.luarc.jsonc', '.stylua.toml', 'init.lua', '.git' },
      settings = {
        Lua = {
          runtime = { version = 'LuaJIT' },
          diagnostics = { globals = { 'vim' } },
          workspace = {
            checkThirdParty = false,
            library = { vim.env.VIMRUNTIME },
          },
          telemetry = { enable = false },
        },
      },
    },
  },
  gopls = {
    desc = 'Go (gopls)',
    bin = 'gopls',
    go = 'golang.org/x/tools/gopls@latest',
    scoop = 'gopls',
    config = {
      filetypes = { 'go', 'gomod', 'gowork', 'gotmpl' },
      root_markers = { 'go.work', 'go.mod', '.git' },
    },
  },
  cmake = {
    desc = 'CMake (cmake-language-server)',
    npm = { 'cmake-language-server' },
    bin = 'cmake-language-server',
    config = {
      filetypes = { 'cmake' },
      root_markers = { 'CMakeLists.txt', '.git' },
    },
  },
  taplo = {
    desc = 'TOML (taplo)',
    npm = { '@taplo/cli' },
    bin = 'taplo',
    config = { cmd_args = { 'lsp', 'stdio' }, filetypes = { 'toml' } },
  },
  marksman = {
    desc = 'Markdown (marksman)',
    bin = 'marksman',
    scoop = 'marksman',
    winget = 'Artempyanykh.Marksman',
    config = { cmd_args = { 'server' }, filetypes = { 'markdown', 'md' } },
  },
  texlab = {
    desc = 'LaTeX / BibTeX (texlab)',
    bin = 'texlab',
    cargo = 'texlab',
    scoop = 'texlab',
    config = {
      filetypes = { 'tex', 'plaintex', 'bib' },
      root_markers = { '.latexmkrc', 'latexmkrc', '.git' },
    },
  },
}

--- Sorted list of server names.
function M.names()
  local names = vim.tbl_keys(M.servers)
  table.sort(names)
  return names
end

--- Server names whose filetypes include `ft`.
function M.for_filetype(ft)
  local found = {}
  for _, name in ipairs(M.names()) do
    if vim.tbl_contains(M.servers[name].config.filetypes or {}, ft) then
      table.insert(found, name)
    end
  end
  return found
end

local aliases = {
  lua = 'lua_ls',
  luals = 'lua_ls',
  ['lua-language-server'] = 'lua_ls',
  py = 'pyright',
  python = 'pyright',
  rs = 'rust_analyzer',
  rust = 'rust_analyzer',
  ['rust-analyzer'] = 'rust_analyzer',
  js = 'ts_ls',
  javascript = 'ts_ls',
  ts = 'ts_ls',
  typescript = 'ts_ls',
  jsx = 'ts_ls',
  tsx = 'ts_ls',
  c = 'clangd',
  cpp = 'clangd',
  md = 'marksman',
  markdown = 'marksman',
  yml = 'yamlls',
  yaml = 'yamlls',
  json = 'jsonls',
  css = 'cssls',
  sh = 'bashls',
  bash = 'bashls',
  php = 'intelephense',
  go = 'gopls',
  toml = 'taplo',
}

--- Resolves user arguments (server names or language/filetype aliases) into registry server names.
function M.resolve_targets(fargs, current_ft)
  if not fargs or #fargs == 0 then
    local ft = (current_ft and current_ft ~= '') and current_ft or ''
    if ft == '' then
      local name = vim.api.nvim_buf_get_name(0):lower()
      if name:match('%.lua$') then ft = 'lua' end
    end
    return M.for_filetype(ft)
  end
  local out = {}
  local seen = {}
  local function add(n)
    if n and M.servers[n] and not seen[n] then
      seen[n] = true
      table.insert(out, n)
    end
  end
  for _, raw in ipairs(fargs) do
    local key = raw:lower()
    if M.servers[raw] then
      add(raw)
    elseif M.servers[key] then
      add(key)
    elseif aliases[key] then
      add(aliases[key])
    else
      for _, by_ft in ipairs(M.for_filetype(key)) do
        add(by_ft)
      end
    end
  end
  return out
end

return M
