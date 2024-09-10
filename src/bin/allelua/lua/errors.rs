use mlua::{chunk, Lua};

pub trait LuaError: std::error::Error {
    fn kind(&self) -> &'static str;
}

pub fn load_errors(lua: &'static Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "errors",
        lua.load(chunk! {
            return function()
                local table = require("table")
                local M = {}

                M.protect = function(func)
                    return function(...)
                        local results = { pcall(func, ...) }
                        if results[1] then
                            table.remove(results, 1)
                            return table.unpack(results)
                        else
                            return nil, results[2]
                        end
                    end
                end

                M.unprotect = function(func)
                    return function(...)
                        local results = { func(...) }
                        if results[1] == nil then
                            local err = results[#results]
                            if #results > 1 and err ~= "nil" then
                                error(err)
                            end
                        end

                        return table.unpack(results)
                    end
                end

                return M
            end
        })
        .eval::<mlua::Function>()?,
    )
}
