use connection::LuaConnection;

mod connection;
pub mod error;

pub fn load_dbus(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "dbus",
        lua.create_function(|lua, ()| {
            let dbus = lua.create_table()?;

            let conn_constructors = lua.create_table()?;
            conn_constructors.set(
                "new_session",
                lua.create_function(|_, ()| LuaConnection::new_session())?,
            )?;
            conn_constructors.set(
                "new_system",
                lua.create_function(|_, ()| LuaConnection::new_system())?,
            )?;
            dbus.set("Connection", conn_constructors)?;

            Ok(dbus)
        })?,
    )
}
