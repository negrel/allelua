use dbus::nonblock::SyncConnection;
use dbus_tokio::connection;
use mlua::UserData;
use tokio::sync::Mutex;

use crate::lua::io;

use super::error::LuaError;

/// Connected is a wrapper around T that prevent access to T if it is not connected.
#[derive(Debug)]
pub struct Connected<T, E>(Result<Mutex<T>, E>);

pub struct LuaConnection(Connected<SyncConnection, mlua::Error>);

impl LuaConnection {
    pub async fn new_session() -> mlua::Result<Self> {
        let (resource, conn) = connection::new_session_sync().map_err(LuaError::from)?;
        let lc = LuaConnection(conn);
        tokio::spawn(async {
            let err: mlua::Error = match resource.await {
                connection::IOResourceError::Dbus(err) => LuaError::from(err).into(),
                connection::IOResourceError::Io(err) => io::LuaError::from(err).into(),
                _ => unreachable!(),
            };
        });

        Ok(lc)
    }
}

impl UserData for LuaConnection {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "dbus.Connection");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {}
}
