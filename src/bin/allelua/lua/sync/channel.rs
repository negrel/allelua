use std::{ffi::c_void, ops::Deref};

use kanal::{AsyncReceiver, AsyncSender};
use mlua::UserData;

pub(super) fn lua_channel(cap: usize) -> (LuaChannelSender, LuaChannelReceiver) {
    let (tx, rx) = kanal::bounded_async::<mlua::Value<'static>>(cap);
    (LuaChannelSender(tx), LuaChannelReceiver(rx))
}

pub(super) struct LuaChannelSender(AsyncSender<mlua::Value<'static>>);

impl Deref for LuaChannelSender {
    type Target = AsyncSender<mlua::Value<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaChannelSender {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, sender, ()| {
            let address = sender as *const _ as usize;
            Ok(format!("ChannelSender 0x{address:x}"))
        });

        methods.add_async_method("send", |_, sender, val: mlua::Value<'lua>| async move {
            // This is safe because this block was blessed by programming gods,
            // (also lua VM is static).
            let val: mlua::Value<'static> = unsafe {
                let ptr = &val as *const _ as *const c_void;
                let val_ref: &mlua::Value<'static> = &*(ptr as *const mlua::Value<'static>);
                val_ref.to_owned()
            };
            sender.send(val).await.map_err(mlua::Error::external)?;
            Ok(())
        });
    }
}

pub(super) struct LuaChannelReceiver(AsyncReceiver<mlua::Value<'static>>);

impl Deref for LuaChannelReceiver {
    type Target = AsyncReceiver<mlua::Value<'static>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaChannelReceiver {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, receiver, ()| {
            let address = receiver as *const _ as usize;
            Ok(format!("ChannelReceiver 0x{address:x}"))
        });

        methods.add_async_method("recv", |_, receiver, ()| async {
            receiver.recv().await.map_err(mlua::Error::external)
        });

        methods.add_method("close", |_, receiver, ()| {
            receiver.close();
            Ok(())
        });
    }
}
