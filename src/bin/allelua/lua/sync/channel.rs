use std::{ffi::c_void, ops::Deref};

use flume::{Receiver, Sender};
use mlua::{FromLua, UserData};

pub(super) fn lua_channel(cap: usize) -> (LuaChannelSender, LuaChannelReceiver) {
    let (tx, rx) = flume::bounded::<mlua::Value>(cap);
    (LuaChannelSender(tx), LuaChannelReceiver(rx))
}

pub struct LuaChannelSender(Sender<mlua::Value>);

impl Deref for LuaChannelSender {
    type Target = Sender<mlua::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaChannelSender {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, sender, ()| {
            let address = sender as *const _ as usize;
            Ok(format!("ChannelSender 0x{address:x}"))
        });

        methods.add_async_method("send", |_, sender, val: mlua::Value| async move {
            // This is safe because this block was blessed by programming gods,
            // (also lua VM is static).
            let val: mlua::Value = unsafe {
                let ptr = &val as *const _ as *const c_void;
                let val_ref: &mlua::Value = &*(ptr as *const mlua::Value);
                val_ref.to_owned()
            };
            sender.send_async(val).await.map_err(mlua::Error::runtime)?;
            Ok(())
        });
    }
}

#[derive(Clone, FromLua)]
pub struct LuaChannelReceiver(pub Receiver<mlua::Value>);

impl Deref for LuaChannelReceiver {
    type Target = Receiver<mlua::Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaChannelReceiver {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, receiver, ()| {
            let address = receiver as *const _ as usize;
            Ok(format!("ChannelReceiver 0x{address:x}"))
        });

        methods.add_async_method("iter", |lua, receiver, ()| async move {
            let next =
                lua.create_async_function(|_lua, receiver: LuaChannelReceiver| async move {
                    receiver.recv_async().await.map_err(mlua::Error::external)
                })?;
            Ok((next, receiver.to_owned()))
        });

        methods.add_async_method("recv", |_, receiver, ()| async move {
            receiver.recv_async().await.map_err(mlua::Error::external)
        });
    }
}
