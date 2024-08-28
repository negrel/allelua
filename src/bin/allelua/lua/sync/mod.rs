use mlua::Lua;

mod channel;
use channel::*;

mod waitgroup;
use waitgroup::*;

pub fn load_sync(lua: &'static Lua) -> mlua::Result<mlua::Table<'static>> {
    lua.load_from_function(
        "sync",
        lua.create_function(|_, ()| {
            let sync = lua.create_table()?;

            sync.set(
                "channel",
                lua.create_function(|_, cap: Option<usize>| Ok(lua_channel(cap.unwrap_or(0))))?,
            )?;

            sync.set(
                "waitgroup",
                lua.create_function(|_, ()| Ok(lua_waitgroup()))?,
            )?;

            Ok(sync)
        })?,
    )
}
