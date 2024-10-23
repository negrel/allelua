use mlua::{Lua, MetaMethod, UserData};
use tokio::task::{self, AbortHandle};

use crate::include_lua;

pub fn load_coroutine(lua: Lua) -> mlua::Result<()> {
    let go = lua.create_async_function(go)?;
    let run_until = lua.create_async_function(|_lua, func: mlua::Function| async move {
        let local = task::LocalSet::new();
        local.run_until(func.call_async::<()>(())).await?;
        Ok(())
    })?;

    lua.load(include_lua!("./coroutine.lua"))
        .eval::<mlua::Function>()?
        .call::<()>((run_until, go))
}

async fn go(
    _lua: Lua,
    (func, args): (mlua::Function, mlua::MultiValue),
) -> mlua::Result<LuaAbortHandle> {
    let handle = tokio::task::spawn_local(func.call_async::<()>(args));
    Ok(LuaAbortHandle(handle.abort_handle()))
}

#[derive(Debug)]
pub struct LuaAbortHandle(AbortHandle);

impl UserData for LuaAbortHandle {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "coroutine.AbortHandle");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |lua, _abort, ()| {
            lua.create_string("coroutine.AbortHandle")
        });

        methods.add_meta_method(MetaMethod::Call, |_lua, abort, ()| {
            abort.0.abort();
            Ok(())
        })
    }
}
