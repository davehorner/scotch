
        return {
          name = "lua-test",
          matches = function(dir) return true end,
          collect_targets = function(dir)
            return '[{"name":"say_hello","metadata":null}]'
          end,
          build_command = function(dir, t)
            return '{"prog":"cmd","args":["/c","echo Lua:' .. t .. '"],"cwd":null}'
          end,
        }

