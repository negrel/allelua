use mlua::{chunk, Lua};

pub fn load_string(lua: &'static Lua) -> mlua::Result<()> {
    // TODO: support mlua::String instead of String which are UTF-8 only.
    let contains =
        lua.create_function(|_lua, (str1, str2): (String, String)| Ok(str1.contains(&str2)))?;

    lua.load(chunk! {
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
    })
    .eval::<()>()
}
