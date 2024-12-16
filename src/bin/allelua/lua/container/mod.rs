use mlua::Lua;

use crate::include_lua;

pub fn load_container(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "container",
        lua.load(include_lua!("./container.lua"))
            .eval::<mlua::Function>()?,
    )
}
