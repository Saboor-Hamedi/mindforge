-- Busy indicator for the MindForge status line. Work in progress (installs,
-- language server indexing) is reported to the host with a `mindforge_progress`
-- notification; an empty label means "idle".

local M = {}

local jobs = {}
local order = 0

local function publish()
  local label = ''
  local newest = -1
  for _, job in pairs(jobs) do
    if job.order > newest then
      newest, label = job.order, job.label
    end
  end
  pcall(vim.rpcnotify, 0, 'mindforge_progress', label)
end

--- Marks `key` as running with a short `label`.
function M.start(key, label)
  order = order + 1
  jobs[key] = { label = label, order = order }
  publish()
end

function M.stop(key)
  if jobs[key] then
    jobs[key] = nil
    publish()
  end
end

--- Mirrors language server work ("indexing", "loading workspace" ...).
function M.watch_lsp(group)
  vim.api.nvim_create_autocmd('LspProgress', {
    group = group,
    callback = function(event)
      local data = event.data or {}
      local value = (data.params or {}).value or {}
      local client = data.client_id and vim.lsp.get_client_by_id(data.client_id)
      local key = 'lsp:' .. tostring(data.client_id) .. ':' .. tostring((data.params or {}).token)
      if value.kind == 'end' then
        M.stop(key)
      else
        local text = value.title or value.message or 'working'
        M.start(key, (client and client.name or 'lsp') .. ': ' .. text)
      end
    end,
  })
end

return M
