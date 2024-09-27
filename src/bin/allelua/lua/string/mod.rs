use mlua::Lua;

use crate::include_lua;

pub fn load_string(lua: Lua) -> mlua::Result<()> {
    // TODO: support mlua::String instead of String which are UTF-8 only.
    let contains =
        lua.create_function(|_lua, (str1, str2): (String, String)| Ok(str1.contains(&str2)))?;

    lua.globals().set("__contains", contains)?;
    let string_mt = lua
        .load(include_lua!("./string.lua"))
        .eval::<mlua::Table>()?;
    lua.globals().set("__contains", mlua::Value::Nil)?;

    lua.set_type_metatable::<mlua::String>(Some(string_mt));

    Ok(())
}
