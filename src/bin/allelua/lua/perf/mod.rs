use mlua::Lua;

use crate::include_lua;

pub fn load_perf(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "perf",
        lua.load(include_lua!("./perf.lua"))
            .eval::<mlua::Function>()?,
    )
}
