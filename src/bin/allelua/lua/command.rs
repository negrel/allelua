use mlua::Lua;

pub fn load_command(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "command",
        lua.create_function(|lua, ()| {
            let command = lua.create_table()?;

            Ok(command)
        })?,
    )
}
