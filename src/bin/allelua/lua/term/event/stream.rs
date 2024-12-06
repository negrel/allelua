use crossterm::event::EventStream;
use futures_util::StreamExt;
use mlua::{AnyUserData, IntoLua, Lua, MetaMethod, UserData, UserDataRef};
use tokio::sync::Mutex;

use crate::lua::io;

use super::{
    LuaEventResize, LuaFocusGainedEvent, LuaFocusLostEvent, LuaKeyEvent, LuaMouseEvent,
    LuaPasteEvent,
};

#[derive(Debug, Default)]
pub struct LuaEventStream(Mutex<EventStream>);

impl UserData for LuaEventStream {
    fn add_fields<F: mlua::UserDataFields<Self>>(_fields: &mut F) {}

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "read",
            |lua, stream, ()| async move { stream.read(&lua).await },
        );

        methods.add_function("iter", |lua, stream: AnyUserData| {
            let closure =
                lua.create_async_function(|lua, stream: UserDataRef<Self>| async move {
                    stream.read(&lua).await
                })?;

            Ok((closure, stream, mlua::Value::Nil))
        });

        methods.add_meta_method(MetaMethod::ToString, |_, stream, ()| {
            let address = stream as *const _ as usize;
            Ok(format!("term.EventStream 0x{address:x}",))
        });
    }
}

impl LuaEventStream {
    pub async fn read(&self, lua: &Lua) -> mlua::Result<mlua::Value> {
        let mut stream = self.0.lock().await;

        match stream.next().await {
            Some(Ok(ev)) => event_to_lua_event(lua, ev),
            Some(Err(err)) => Err(io::LuaError::from(err).into()),
            None => Ok(mlua::Value::Nil),
        }
    }
}

fn event_to_lua_event(lua: &Lua, ev: crossterm::event::Event) -> mlua::Result<mlua::Value> {
    match ev {
        crossterm::event::Event::FocusGained => LuaFocusGainedEvent.into_lua(lua),
        crossterm::event::Event::FocusLost => LuaFocusLostEvent.into_lua(lua),
        crossterm::event::Event::Key(ev) => LuaKeyEvent::from(ev).into_lua(lua),
        crossterm::event::Event::Mouse(ev) => LuaMouseEvent::from(ev).into_lua(lua),
        crossterm::event::Event::Paste(ev) => LuaPasteEvent::from(ev).into_lua(lua),
        crossterm::event::Event::Resize(cols, rows) => {
            LuaEventResize::from((cols, rows)).into_lua(lua)
        }
    }
}
