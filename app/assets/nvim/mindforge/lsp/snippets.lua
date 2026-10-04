-- In-process "language server" that offers built-in snippets (HTML skeleton,
-- React components, Python/Rust boilerplate ...). It works with no download,
-- and shows up in the completion popup exactly like a real server.

local emmet = require('mindforge.lsp.emmet')

local M = {}

local KIND_SNIPPET = 15

local html5 = table.concat({
  '<!DOCTYPE html>',
  '<html lang="${1:en}">',
  '<head>',
  '  <meta charset="UTF-8">',
  '  <meta name="viewport" content="width=device-width, initial-scale=1.0">',
  '  <title>${2:Document}</title>',
  '</head>',
  '<body>',
  '  $0',
  '</body>',
  '</html>',
}, '\n')

-- Plain-text body of a snippet for the popup's preview pane.
local function plain(text)
  return (text:gsub('%$%{%d+:([^}]*)%}', '%1'):gsub('%$%{%d+%}', ''):gsub('%$%d+', ''))
end

local skeletons = {
  { 'HTML5 document skeleton', html5 },
  {
    'HTML5 + stylesheet + script',
    table.concat({
      '<!DOCTYPE html>', '<html lang="en">', '<head>', '  <meta charset="UTF-8">',
      '  <meta name="viewport" content="width=device-width, initial-scale=1.0">',
      '  <title>${1:Document}</title>', '  <link rel="stylesheet" href="style.css">', '</head>',
      '<body>', '  $0', '  <script src="main.js"></script>', '</body>', '</html>',
    }, '\n'),
  },
  { 'Minimal HTML', '<!DOCTYPE html>\n<html>\n<head>\n  <title>${1:Document}</title>\n</head>\n<body>\n  $0\n</body>\n</html>' },
}

local js_snippets = {
  { 'clg', 'console.log($0)', 'console.log' },
  { 'console', 'console.log($0)', 'console.log' },
  { 'log', 'console.log($0)', 'console.log' },
  { 'fn', 'function ${1:name}(${2:args}) {\n  $0\n}', 'function declaration' },
  { 'afn', 'const ${1:name} = (${2:args}) => {\n  $0\n}', 'arrow function' },
  { 'imp', "import ${1:name} from '${2:module}'", 'import default' },
  { 'impn', "import { $1 } from '${2:module}'", 'import named' },
  { 'try', 'try {\n  $1\n} catch (${2:error}) {\n  $0\n}', 'try / catch' },
  { 'forof', 'for (const ${1:item} of ${2:items}) {\n  $0\n}', 'for...of' },
}

local react_snippets = {
  {
    'rfc',
    'export default function ${1:Component}(${2:props}) {\n  return (\n    <div>\n      $0\n    </div>\n  )\n}',
    'React function component',
  },
  { 'useState', 'const [${1:state}, set${2:State}] = useState(${3:null})', 'useState hook' },
  { 'useEffect', 'useEffect(() => {\n  $1\n  return () => {\n    $2\n  }\n}, [${3}])', 'useEffect hook' },
}

local ts_extra = {
  { 'int', 'interface ${1:Name} {\n  $0\n}', 'interface' },
  { 'type', 'type ${1:Name} = $0', 'type alias' },
}

local function concat(...)
  local out = {}
  for _, list in ipairs({ ... }) do
    vim.list_extend(out, list)
  end
  return out
end

M.snippets = {
  html = {
    { 'html5', html5, 'HTML5 document skeleton' },
    { 'html', html5, 'HTML5 document skeleton' },
    { 'doctype', '<!DOCTYPE html>', 'HTML5 doctype' },
    { 'meta:vp', '<meta name="viewport" content="width=device-width, initial-scale=1.0">', 'viewport meta' },
    { 'link:css', '<link rel="stylesheet" href="${1:style.css}">', 'stylesheet link' },
    { 'script:src', '<script src="${1:main.js}"></script>', 'script tag' },
    { 'style', '<style>\n  $0\n</style>', 'style block' },
    { 'form', '<form action="${1}" method="${2:post}">\n  $0\n</form>', 'form' },
    { 'table', '<table>\n  <thead>\n    <tr>\n      <th>$1</th>\n    </tr>\n  </thead>\n  <tbody>\n    <tr>\n      <td>$0</td>\n    </tr>\n  </tbody>\n</table>', 'table' },
    { 'nav', '<nav>\n  <ul>\n    <li><a href="${1:#}">${2:Link}</a></li>\n  </ul>\n</nav>', 'navigation' },
  },
  javascript = js_snippets,
  javascriptreact = concat(js_snippets, react_snippets),
  typescript = concat(js_snippets, ts_extra),
  typescriptreact = concat(js_snippets, ts_extra, react_snippets),
  css = {
    { 'flex', 'display: flex;\nalign-items: ${1:center};\njustify-content: ${2:center};', 'flex container' },
    { 'grid', 'display: grid;\ngrid-template-columns: ${1:repeat(3, 1fr)};\ngap: ${2:1rem};', 'grid container' },
  },
  python = {
    { 'def', 'def ${1:name}(${2:args}):\n    ${0:pass}', 'function' },
    { 'class', 'class ${1:Name}:\n    def __init__(self${2}):\n        ${0:pass}', 'class' },
    { 'ifmain', "if __name__ == '__main__':\n    ${0:main()}", 'main guard' },
    { 'try', 'try:\n    $1\nexcept ${2:Exception} as ${3:e}:\n    ${0:raise}', 'try / except' },
  },
  rust = {
    { 'main', 'fn main() {\n    $0\n}', 'main function' },
    { 'fn', 'fn ${1:name}(${2}) {\n    $0\n}', 'function' },
    { 'struct', 'struct ${1:Name} {\n    $0\n}', 'struct' },
    { 'impl', 'impl ${1:Type} {\n    $0\n}', 'impl block' },
    { 'test', '#[test]\nfn ${1:name}() {\n    $0\n}', 'test function' },
    { 'println', 'println!("${1}"$2);', 'println!' },
  },
  php = {
    { 'php', '<?php\n\n$0', 'PHP open tag' },
    { 'func', 'function ${1:name}(${2}) {\n    $0\n}', 'function' },
  },
}

local MARKUP = { html = false, javascriptreact = true, typescriptreact = true }

local function snippet_item(label, text, detail, sort)
  return {
    label = label,
    kind = KIND_SNIPPET,
    detail = detail,
    insertText = text,
    insertTextFormat = 2,
    documentation = plain(text),
    sortText = sort or ('~' .. label), -- real server results rank before snippets
  }
end

-- Emmet abbreviations and tag snippets, replacing the typed abbreviation.
local function has_client(bufnr, name)
  return #vim.lsp.get_clients({ bufnr = bufnr, name = name }) > 0
end

local function markup_items(filetype, line_before, params, bufnr)
  local items = {}
  local jsx = MARKUP[filetype]
  local bang = line_before:match('!$')
  local abbr = bang and '!' or emmet.abbreviation(line_before)
  if not abbr then
    return items
  end
  local range = {
    start = { line = params.position.line, character = params.position.character - #abbr },
    ['end'] = params.position,
  }
  local function add(label, text, detail, sort)
    local item = snippet_item(label, text, detail, sort)
    item.filterText = label
    item.textEdit = { range = range, newText = text }
    items[#items + 1] = item
  end
  if bang then
    if not jsx then
      for index, skeleton in ipairs(skeletons) do
        add('!' .. (index > 1 and ' ' .. index or ''), skeleton[2], skeleton[1], tostring(index))
        items[#items].filterText = '!'
      end
    end
    return items
  end
  -- One entry for the abbreviation being typed, previewed on the right.
  local expanded = emmet.expand(abbr, jsx)
  if expanded and (abbr:find('[>+*.#]') or emmet.is_tag(abbr)) then
    add(abbr, expanded, emmet.preview(expanded), '0' .. abbr)
  end
  -- Plain tag names, unless the HTML language server already offers them.
  local after_lt = line_before:sub(1, #line_before - #abbr):match('<$') ~= nil
  if abbr:match('^%a[%w]*$') and not (after_lt and has_client(bufnr, 'html')) then
    local shown = 0
    for _, tag in ipairs(emmet.tags) do
      if tag ~= abbr and vim.startswith(tag, abbr) and shown < 8 then
        shown = shown + 1
        add(tag, emmet.expand(tag, jsx), emmet.preview(emmet.expand(tag, jsx)), '1' .. tag)
      end
    end
  end
  return items
end

local function items_for(filetype, params, bufnr)
  local items = {}
  for _, s in ipairs(M.snippets[filetype] or {}) do
    items[#items + 1] = snippet_item(s[1], s[2], s[3])
  end
  if MARKUP[filetype] ~= nil and params then
    local line = vim.api.nvim_buf_get_lines(bufnr, params.position.line, params.position.line + 1, false)[1] or ''
    local before = line:sub(1, params.position.character)
    local markup = markup_items(filetype, before, params, bufnr)
    if before:match('!$') then
      return markup -- mixing in plain snippets would shift the replaced range
    end
    vim.list_extend(items, markup)
  end
  return items
end
local function start_server(dispatchers)
  local closing = false
  local server = {}

  function server.request(method, params, callback)
    -- This server runs inside Neovim's LSP client. Keep malformed/stale
    -- completion requests from escaping as Lua callback errors: Neovim can
    -- issue a request while a buffer is being renamed or detached.
    if type(callback) ~= 'function' then
      return false
    end
    if method == 'initialize' then
      callback(nil, { capabilities = { completionProvider = { resolveProvider = false, triggerCharacters = {} } } })
    elseif method == 'textDocument/completion' then
      local result = { isIncomplete = false, items = {} }
      local ok, err = pcall(function()
        if type(params) ~= 'table'
          or type(params.textDocument) ~= 'table'
          or type(params.textDocument.uri) ~= 'string'
          or type(params.position) ~= 'table'
          or type(params.position.line) ~= 'number'
          or type(params.position.character) ~= 'number' then
          return
        end
        local bufnr = vim.uri_to_bufnr(params.textDocument.uri)
        if type(bufnr) ~= 'number' or not vim.api.nvim_buf_is_valid(bufnr) then
          return
        end
        local filetype = vim.bo[bufnr].filetype
        result.items = items_for(filetype, params, bufnr)
      end)
      if not ok then
        vim.schedule(function()
          vim.notify('MindForge completion callback failed: ' .. tostring(err), vim.log.levels.WARN)
        end)
      end
      callback(nil, result)
    elseif method == 'shutdown' then
      callback(nil, nil)
    else
      callback(nil, nil)
    end
    return true, 1
  end

  function server.notify(method)
    if method == 'exit' then
      closing = true
      dispatchers.on_exit(0, 0)
    end
    return true
  end

  function server.is_closing()
    return closing
  end

  function server.terminate()
    closing = true
  end

  return server
end

--- Attaches the snippet server to `bufnr` when its filetype has snippets.
function M.attach(bufnr)
  local filetype = vim.bo[bufnr].filetype
  -- `!` and the Emmet operators must count as word characters so accepting
  -- `!` or `ul>li*2` replaces the whole abbreviation.
  local extra = ',!,>,+,*,.,#'
  local keyword = vim.bo[bufnr].iskeyword:gsub(vim.pesc(extra), '')
  vim.bo[bufnr].iskeyword = MARKUP[filetype] ~= nil and (keyword .. extra) or keyword
  if not M.snippets[filetype] and MARKUP[filetype] == nil then
    return
  end
  vim.lsp.start({
    name = 'mindforge-snippets',
    cmd = start_server,
    root_dir = vim.fn.getcwd(),
  }, { bufnr = bufnr })
end

return M
