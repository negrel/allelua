use mlua::Lua;

mod channel;
use channel::*;

pub fn load_sync(lua: &'static Lua) -> mlua::Result<mlua::Table<'static>> {
    lua.load_from_function(
        "sync",
        lua.create_function(|_, ()| {
            let sync = lua.create_table()?;

            sync.set(
                "channel",
                lua.create_function(|_, cap: Option<usize>| Ok(lua_channel(cap.unwrap_or(0))))?,
            )?;

            Ok(sync)
        })?,
    )
}
