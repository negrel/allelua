use std::rc::Rc;

use mlua::{FromLua, UserData};

mod queue;
pub use queue::{BufferedQueue, Queue, UnbufferedQueue};

pub enum ChannelReceiver {
    Buffered(LuaChannelReceiver<BufferedQueue>),
    Unbuffered(LuaChannelReceiver<UnbufferedQueue>),
}

impl ChannelReceiver {
    pub async fn recv(&self) -> (mlua::Value, bool) {
        match self {
            ChannelReceiver::Buffered(ch) => ch.recv().await,
            ChannelReceiver::Unbuffered(ch) => ch.recv().await,
        }
    }
}

pub(super) fn lua_unbuffered_channel() -> (
    LuaChannelSender<UnbufferedQueue>,
    LuaChannelReceiver<UnbufferedQueue>,
) {
    let queue = Rc::new(UnbufferedQueue::default());
    (LuaChannelSender(queue.clone()), LuaChannelReceiver(queue))
}

pub(super) fn lua_buffered_channel(
    cap: usize,
) -> (
    LuaChannelSender<BufferedQueue>,
    LuaChannelReceiver<BufferedQueue>,
) {
    let queue = Rc::new(BufferedQueue::new(cap));
    (LuaChannelSender(queue.clone()), LuaChannelReceiver(queue))
}

#[derive(Clone, FromLua)]
pub struct LuaChannelSender<T: Queue>(Rc<T>);

impl<T: Queue + 'static> UserData for LuaChannelSender<T> {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, sender, ()| {
            let address = sender as *const _ as usize;
            Ok(format!("ChannelSender 0x{address:x}"))
        });

        methods.add_async_method("send", |_, sender, val: mlua::Value| async move {
            sender.0.push(val).await.map_err(mlua::Error::external)?;
            Ok(())
        });

        methods.add_method("close", |_, sender, ()| Ok(sender.0.close()));
        methods.add_method("is_closed", |_, sender, ()| Ok(sender.0.is_closed()));
    }
}

#[derive(FromLua)]
pub struct LuaChannelReceiver<T: Queue + 'static>(Rc<T>);

impl<T: Queue + 'static> Clone for LuaChannelReceiver<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Queue + 'static> LuaChannelReceiver<T> {
    pub async fn recv(&self) -> (mlua::Value, bool) {
        match self.0.pop().await {
            Ok(v) => (v, true),
            Err(queue::QueueError::Closed) => (mlua::Value::Nil, false),
        }
    }
}

impl<T: Queue + 'static> UserData for LuaChannelReceiver<T> {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChannelSender")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, receiver, ()| {
            let address = receiver as *const _ as usize;
            Ok(format!("ChannelReceiver 0x{address:x}"))
        });

        methods.add_async_method("recv", |_, receiver, ()| async move {
            Ok(receiver.recv().await)
        });
    }
}
