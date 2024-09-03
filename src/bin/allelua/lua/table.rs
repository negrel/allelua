use mlua::chunk;

pub fn load_table(lua: &'static mlua::Lua) -> mlua::Result<()> {
    lua.load(chunk! {
        local table = require("table")
        local M = table

        // Freeze table.
        M.frozen_table_key = "__freeze"
        M.frozen_table_mt = {
          __index = function(t, k)
            return t[M.frozen_table_key][k]
          end,
          __newindex = function()
            error("attempt to update a frozen table", 2)
          end,
          __len       = function(t) return #t[M.frozen_table_key]          end,
          __tostring  = function(t) return tostring(t[M.frozen_table_key]) end,
          __pairs     = function(t) return pairs(t[M.frozen_table_key])    end,
          __ipairs    = function(t) return ipairs(t[M.frozen_table_key])   end,
          __iter      = function(t) return iter(t[M.frozen_table_key])     end,
          __metatable = false, // protect metatable
        }
        M.freeze = function(t)
          assert(type(t) == "table", "invalid input: " .. tostring(t) .. " is not a table")
          if t[M.frozen_table_key] ~= nil then return t end
          local proxy = { [M.frozen_table_key] = t }
          setmetatable(proxy, M.frozen_table_mt)
          return proxy
        end
        M.is_frozen = function(t)
            return t[M.frozen_table_key] ~= nil
        end

        M.push = function(t, ...)
            local args = {...}
            for v in ipairs(args) do
                t[#t + 1] = v
            end
        end

        M.map = function(t, map_fn)
            local result = {}
            for k, v in pairs(t) do
                t[k] = map_fn(k, v)
            end
            setmetatable(result, getmetatable(t))
            return result
        end
    })
    .eval::<()>()?;

    Ok(())
}
