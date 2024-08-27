use std::ffi::c_void;

use mlua::{Lua, UserData};
use tokio::sync;

struct LuaMpscSender(sync::mpsc::Sender<mlua::Value<'static>>);

impl UserData for LuaMpscSender {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method("send", |_, sender, val: mlua::Value<'lua>| async move {
            let tx = &sender.0;
            // This is safe because lua VM is static.
            let val: mlua::Value<'static> = unsafe {
                let ptr = &val as *const _ as *const c_void;
                let val_ref: &mlua::Value<'static> = &*(ptr as *const mlua::Value<'static>);
                val_ref.to_owned()
            };
            tx.send(val)
                .await
                .map_err(|err| mlua::Error::RuntimeError(err.to_string()))?;
            Ok(())
        });
    }
}

struct LuaMpscReceiver(sync::mpsc::Receiver<mlua::Value<'static>>);

impl UserData for LuaMpscReceiver {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_async_method_mut("recv", |_, receiver, ()| async {
            Ok(receiver.0.recv().await)
        });

        methods.add_method_mut("close", |_, receiver, ()| {
            receiver.0.close();
            Ok(())
        });
    }
}

pub fn load_sync(lua: &'static Lua) -> mlua::Result<mlua::Table<'static>> {
    lua.load_from_function(
        "sync",
        lua.create_function(|_, ()| {
            let sync = lua.create_table()?;

            sync.set(
                "mpsc",
                lua.create_function(|_, cap: usize| {
                    let (tx, rx) = sync::mpsc::channel::<mlua::Value<'static>>(cap);
                    Ok((LuaMpscSender(tx), LuaMpscReceiver(rx)))
                })?,
            )?;

            Ok(sync)
        })?,
    )
}
