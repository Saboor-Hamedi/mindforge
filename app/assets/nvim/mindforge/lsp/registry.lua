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
      root_markers = { 'eslint.config.js', '.eslintrc.json', '.eslintrc.js', 'package.json', '.git' },
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

return M
