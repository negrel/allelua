use mlua::Lua;

mod channel;
use channel::*;

mod waitgroup;
use waitgroup::*;

use crate::LuaModule;

LuaModule!(LuaSyncModule,
    fields { WaitGroup = LuaWaitGroupConstructors },
    functions { channel(_lua, cap: Option<usize>) { Ok(lua_channel(cap.unwrap_or(0))) } },
    async functions {}
);

pub fn load_sync(lua: &'static Lua) -> mlua::Result<LuaSyncModule> {
    lua.load_from_function("sync", lua.create_function(|_, ()| Ok(LuaSyncModule))?)
}
