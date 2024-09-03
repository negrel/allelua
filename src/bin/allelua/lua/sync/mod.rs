use mlua::Lua;

mod channel;
use channel::*;

mod waitgroup;
use waitgroup::*;

pub fn load_sync(lua: &'static Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "sync",
        lua.create_function(|lua, ()| {
            let sync = lua.create_table()?;

            let wg_constructors = lua.create_table()?;
            wg_constructors.set("new", lua.create_function(|_, ()| Ok(LuaWaitGroup::new()))?)?;
            sync.set("WaitGroup", wg_constructors)?;

            sync.set(
                "channel",
                lua.create_function(|_lua, cap: Option<usize>| Ok(lua_channel(cap.unwrap_or(0))))?,
            )?;

            Ok(sync)
        })?,
    )
}
