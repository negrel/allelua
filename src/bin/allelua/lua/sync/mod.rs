use mlua::{IntoLuaMulti, Lua};

mod channel;
pub use channel::*;

mod waitgroup;
use waitgroup::*;

pub fn load_sync(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "sync",
        lua.create_function(|lua, ()| {
            let sync = lua.create_table()?;
            lua.globals().set("sync", sync.clone())?;

            let wg_constructors = lua.create_table()?;
            wg_constructors.set("new", lua.create_function(|_, ()| Ok(LuaWaitGroup::new()))?)?;
            sync.set("WaitGroup", wg_constructors)?;

            sync.set(
                "channel",
                lua.create_function(|lua, cap: Option<usize>| {
                    if let Some(cap) = cap {
                        if cap > 0 {
                            return lua_buffered_channel(cap).into_lua_multi(lua);
                        }
                    }

                    lua_unbuffered_channel().into_lua_multi(lua)
                })?,
            )?;

            Ok(sync)
        })?,
    )
}
