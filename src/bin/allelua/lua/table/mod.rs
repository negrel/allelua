use mlua::Lua;

pub fn load_table(lua: Lua) -> mlua::Result<()> {
    let is_empty = lua.create_function(|_lua, t: mlua::Table| Ok(t.is_empty()))?;

    lua.load(include_str!("./table.lua"))
        .eval::<mlua::Function>()?
        .call::<()>(is_empty)?;

    Ok(())
}
