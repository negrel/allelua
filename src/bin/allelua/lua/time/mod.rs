use mlua::Lua;
use tokio::time::{self, Duration, Instant};

use crate::include_lua;

mod duration;
mod instant;

use duration::*;
use instant::*;

pub fn load_time(lua: Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "time",
        lua.create_function(|lua, ()| {
            let time = lua.create_table()?;
            lua.globals().set("time", time.clone())?;

            time.set("nanosecond", LuaDuration::from(Duration::from_nanos(1)))?;
            time.set("microsecond", LuaDuration::from(Duration::from_micros(1)))?;
            time.set("millisecond", LuaDuration::from(Duration::from_millis(1)))?;
            time.set("second", LuaDuration::from(Duration::from_secs(1)))?;
            time.set("minute", LuaDuration::from(Duration::from_secs(60)))?;
            time.set("hour", LuaDuration::from(Duration::from_secs(60 * 60)))?;
            time.set(
                "sleep",
                lua.create_async_function(|_, dur: LuaDuration| async move {
                    time::sleep(*dur).await;
                    Ok(())
                })?,
            )?;

            let instant = lua.create_table()?;
            instant.set(
                "now",
                lua.create_function(|_, ()| {
                    let instant = Instant::now();
                    Ok(LuaInstant::from(instant))
                })?,
            )?;
            time.set("Instant", instant.clone())?;

            lua.load(include_lua!("./time.lua"))
                .eval::<mlua::Function>()?
                .call::<()>(time.clone())?;

            Ok(time)
        })?,
    )
}
