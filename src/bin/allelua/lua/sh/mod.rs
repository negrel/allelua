use mlua::Lua;

use crate::include_lua;

pub fn load_sh(lua: &Lua) -> mlua::Result<mlua::Table> {
    let sh = lua.load_from_function::<mlua::Table>(
        "sh",
        lua.load(include_lua!("./sh.lua"))
            .eval::<mlua::Function>()?,
    )?;
    lua.globals().set("sh", sh.clone())?;
    Ok(sh)
}
