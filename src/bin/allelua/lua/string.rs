use mlua::{chunk, Lua};

pub fn load_string(lua: Lua) -> mlua::Result<()> {
    // TODO: support mlua::String instead of String which are UTF-8 only.
    let contains =
        lua.create_function(|_lua, (str1, str2): (String, String)| Ok(str1.contains(&str2)))?;

    let string_mt = lua
        .load(chunk! {
            local string = require("string");
            local M = string

            M.slice = M.sub
            M.sub = nil

            M.has_prefix = function(str, prefix)
                return string.slice(str, 0, #prefix) == prefix
            end

            M.has_suffix = function(str, suffix)
                return string.slice(str, -#suffix) == suffix
            end

            M.contains = $contains

            return {
                __index = M
            }
        })
        .eval::<mlua::Table>()?;

    lua.set_type_metatable::<mlua::String>(Some(string_mt));

    Ok(())
}
