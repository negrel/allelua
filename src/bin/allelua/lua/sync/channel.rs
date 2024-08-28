use std::ffi::c_void;

use kanal::{AsyncReceiver, AsyncSender};
use mlua::UserData;

pub fn lua_channel(cap: usize) -> (LuaChannelSender, LuaChannelReceiver) {
    let (tx, rx) = kanal::bounded_async::<mlua::Value<'static>>(cap);
    (LuaChannelSender(tx), LuaChannelReceiver(rx))
}

pub(crate) struct LuaChannelSender(AsyncSender<mlua::Value<'static>>);

impl UserData for LuaChannelSender {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, sender, ()| {
            let address = sender as *const _ as usize;
            Ok(format!("ChannelSender 0x{address:x}"))
        });

        methods.add_async_method("send", |_, sender, val: mlua::Value<'lua>| async move {
            let tx = &sender.0;
            // This is safe because this block was blessed by programming gods,
            // (also lua VM is static).
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

pub(crate) struct LuaChannelReceiver(AsyncReceiver<mlua::Value<'static>>);

impl UserData for LuaChannelReceiver {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, receiver, ()| {
            let address = receiver as *const _ as usize;
            Ok(format!("ChannelReceiver 0x{address:x}"))
        });

        methods.add_async_method("recv", |_, receiver, ()| async {
            receiver
                .0
                .recv()
                .await
                .map_err(|err| mlua::Error::RuntimeError(err.to_string()))
        });

        methods.add_method("close", |_, receiver, ()| {
            receiver.0.close();
            Ok(())
        });
    }
}