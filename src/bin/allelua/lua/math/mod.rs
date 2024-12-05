use mlua::Lua;

use crate::include_lua;

pub fn load_math(lua: &Lua) -> mlua::Result<()> {
    lua.load(include_lua!("./math.lua")).exec()
}
