use mlua::Lua;

use crate::include_lua;

pub fn load_sh(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "sh",
        lua.load(include_lua!("./sh.lua"))
            .eval::<mlua::Function>()?,
    )
}
