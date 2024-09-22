use std::{ffi::c_void, ops::Deref};

use flume::{Receiver, Sender};
use mlua::{FromLua, UserData};

pub(super) fn lua_channel(cap: usize) -> (LuaChannelSender, LuaChannelReceiver) {
    let (tx, rx) = flume::bounded::<mlua::Value<'static>>(cap);
    (LuaChannelSender(tx), LuaChannelReceiver(rx))
}

pub struct LuaChannelSender(Sender<mlua::Value<'static>>);

impl Deref for LuaChannelSender {
    type Target = Sender<mlua::Value<'static>>;

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
            sender.send_async(val).await.map_err(mlua::Error::runtime)?;
            Ok(())
        });
    }
}

#[derive(Clone, FromLua)]
pub struct LuaChannelReceiver(pub Receiver<mlua::Value<'static>>);

impl Deref for LuaChannelReceiver {
    type Target = Receiver<mlua::Value<'static>>;

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
            receiver.recv_async().await.map_err(mlua::Error::external)
        });
    }
}
