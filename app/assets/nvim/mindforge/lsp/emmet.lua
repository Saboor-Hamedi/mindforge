-- Small built-in Emmet engine: expands abbreviations such as
-- `ul.menu>li*3>a`, `div#app.card` or `h1+p` into LSP snippet text. It powers
-- HTML / JSX / TSX completion without any language server download.

local M = {}

local VOID = { img = true, input = true, br = true, hr = true, meta = true, link = true }

M.tags = {
  'a', 'abbr', 'article', 'aside', 'audio', 'b', 'blockquote', 'body', 'br', 'button', 'canvas', 'code',
  'div', 'em', 'footer', 'form', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'head', 'header', 'hr', 'html', 'i',
  'iframe', 'img', 'input', 'label', 'li', 'link', 'main', 'meta', 'nav', 'ol', 'option', 'p', 'pre',
  'script', 'section', 'select', 'small', 'span', 'strong', 'style', 'svg', 'table', 'tbody', 'td',
  'textarea', 'th', 'thead', 'title', 'tr', 'ul', 'video',
}

local KNOWN = {}
for _, tag in ipairs(M.tags) do
  KNOWN[tag] = true
end

local function parse_item(text)
  local item = { tag = text:match('^[%a][%w%-]*'), classes = {}, count = 1 }
  local rest = text:sub(#(item.tag or '') + 1)
  local count = rest:match('%*(%d+)$')
  if count then
    item.count = math.min(tonumber(count), 50)
    rest = rest:gsub('%*%d+$', '')
  end
  for sigil, name in rest:gmatch('([#%.])([%w_%-]+)') do
    if sigil == '#' then
      item.id = name
    else
      item.classes[#item.classes + 1] = name
    end
  end
  if rest:gsub('[#%.][%w_%-]+', '') ~= '' then
    return nil
  end
  if item.tag and not KNOWN[item.tag] then
    return nil -- avoids treating console.log or oo.bar as markup
  end
  if not item.tag then
    if not item.id and #item.classes == 0 then
      return nil
    end
    item.tag = 'div'
  end
  return item
end

-- `a>b+c` -> { a{children = {b}}, c }: `>` nests the rest, `+` adds a sibling.
local function parse(abbr)
  local pos = abbr:find('[>+]')
  local item = parse_item(pos and abbr:sub(1, pos - 1) or abbr)
  if not item then
    return nil
  end
  if not pos then
    return { item }
  end
  local rest = parse(abbr:sub(pos + 1))
  if not rest then
    return nil
  end
  if abbr:sub(pos, pos) == '>' then
    item.children = rest
    return { item }
  end
  table.insert(rest, 1, item)
  return rest
end

local function open_tag(item, jsx)
  local attrs = ''
  if item.id then
    attrs = attrs .. ' id="' .. item.id .. '"'
  end
  if #item.classes > 0 then
    attrs = attrs .. ' ' .. (jsx and 'className' or 'class') .. '="' .. table.concat(item.classes, ' ') .. '"'
  end
  return '<' .. item.tag .. attrs
end

local function render(nodes, depth, jsx, state, lines)
  local pad = string.rep('  ', depth)
  for _, item in ipairs(nodes) do
    for _ = 1, item.count do
      local open = open_tag(item, jsx)
      if VOID[item.tag] then
        lines[#lines + 1] = pad .. open .. (jsx and ' />' or '>')
      elseif item.children then
        lines[#lines + 1] = pad .. open .. '>'
        render(item.children, depth + 1, jsx, state, lines)
        lines[#lines + 1] = pad .. '</' .. item.tag .. '>'
      else
        if state.placed then
          lines[#lines + 1] = pad .. open .. '></' .. item.tag .. '>'
        else
          -- First empty element: caret on its own indented line between the tags.
          state.placed = true
          lines[#lines + 1] = pad .. open .. '>'
          lines[#lines + 1] = pad .. '  $0'
          lines[#lines + 1] = pad .. '</' .. item.tag .. '>'
        end
      end
    end
  end
end

--- Returns snippet text for `abbr`, or nil when it is not a valid abbreviation.
function M.expand(abbr, jsx)
  local nodes = parse(abbr)
  if not nodes then
    return nil
  end
  local lines = {}
  render(nodes, 0, jsx, {}, lines)
  return table.concat(lines, '\n')
end

--- The abbreviation immediately before the cursor in `line_before`, if any.
function M.abbreviation(line_before)
  return line_before:match('[%w%.#>+*%-_]+$')
end

--- True when `abbr` is exactly a known tag name.
function M.is_tag(abbr)
  return KNOWN[abbr] == true
end

--- One-line summary of an expansion for the completion detail column.
function M.preview(expanded)
  local text = (expanded or ''):gsub('%$%d', ''):gsub('%${%d:([^}]*)}', '%1')
  text = text:gsub('%s*\n%s*', ''):gsub('%s+', ' ')
  return #text > 48 and (text:sub(1, 47) .. '…') or text
end

return M