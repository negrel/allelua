mod error;
mod seek_from;

use std::io::SeekFrom;

use mlua::Lua;

pub use error::*;
pub use seek_from::*;

pub fn load_io(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "io",
        lua.create_function(|lua, ()| {
            let io = lua.create_table()?;

            let seek_from_constructors = lua.create_table()?;
            seek_from_constructors.set(
                "start",
                lua.create_function(|_lua, offset: u64| Ok(LuaSeekFrom(SeekFrom::Start(offset))))?,
            )?;
            seek_from_constructors.set(
                "end",
                lua.create_function(|_lua, offset: i64| Ok(LuaSeekFrom(SeekFrom::End(offset))))?,
            )?;
            seek_from_constructors.set(
                "current",
                lua.create_function(|_lua, offset: i64| {
                    Ok(LuaSeekFrom(SeekFrom::Current(offset)))
                })?,
            )?;
            io.set("SeekFrom", seek_from_constructors)?;

            Ok(io)
        })?,
    )
}
