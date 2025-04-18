local plugin = {}

plugin.name = "npm"

function plugin.matches(dir)
  local f = io.open(dir .. "/package.json")
  if not f then return false end
  f:close()
  return true
end

function plugin.collect_targets(dir)
  return '[{"name":"npm.lua-build!","metadata":null}]'
end

function plugin.build_command(dir, target_name)
  return '{"prog":"echo","args":["Hello from","build"],"cwd":null}'
end

return {
  name = "npm",
  matches = function(dir)
    return true
  end,
  collect_targets = function(dir)
    return '[{"name": "npm-lua-echo", "metadata": null}]'
  end,
  build_command = function(dir, target_name)
    return '{"prog":"cmd","args":["/c echo Hello from", "' .. target_name .. '"],"cwd":null}'
  end
}