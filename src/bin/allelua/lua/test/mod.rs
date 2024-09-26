use mlua::Lua;

pub fn load_test(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "test",
        lua.create_function(|lua, ()| lua.load(include_str!("./test.lua")).eval::<mlua::Table>())?,
    )
}
