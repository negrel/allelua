use core::time;
use std::ops::Deref;

use mlua::{FromLua, Lua, UserData};

#[derive(Clone, Copy, FromLua)]
struct LuaDuration(time::Duration);

impl Deref for LuaDuration {
    type Target = time::Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaDuration {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::Eq, |_lua, dur1, dur2: LuaDuration| {
            Ok(dur1.0 == dur2.0)
        });
        methods.add_meta_method(mlua::MetaMethod::Lt, |_lua, dur1, dur2: LuaDuration| {
            Ok(dur1.0 < dur2.0)
        });
        methods.add_meta_method(mlua::MetaMethod::Le, |_lua, dur1, dur2: LuaDuration| {
            Ok(dur1.0 <= dur2.0)
        });
        methods.add_meta_method(mlua::MetaMethod::Add, |_lua, dur1, dur2: LuaDuration| {
            Ok(LuaDuration(dur1.0 + dur2.0))
        });
        methods.add_meta_method(mlua::MetaMethod::Unm, |_lua, dur1, dur2: LuaDuration| {
            Ok(LuaDuration(dur1.0 - dur2.0))
        });
        methods.add_meta_function(
            mlua::MetaMethod::Mul,
            |_lua, (lhs, rhs): (mlua::Value, mlua::Value)| match (lhs, rhs) {
                (mlua::Value::UserData(ud), mlua::Value::Integer(n))
                | (mlua::Value::Integer(n), mlua::Value::UserData(ud)) => {
                    let dur = *ud.borrow::<Self>()?;
                    let n = u32::try_from(n).map_err(mlua::Error::external)?;
                    Ok(LuaDuration(dur.0 * n))
                }
                (mlua::Value::UserData(ud), mlua::Value::Number(n))
                | (mlua::Value::Number(n), mlua::Value::UserData(ud)) => {
                    let dur = *ud.borrow::<Self>()?;
                    Ok(LuaDuration(dur.0.mul_f64(n)))
                }
                _ => Err(mlua::Error::external(
                    "Duration can only be multiplied with integers",
                )),
            },
        );
        methods.add_meta_method(mlua::MetaMethod::Div, |_lua, dur1, dur2: u32| {
            Ok(LuaDuration(dur1.0 / dur2))
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, dur, ()| {
            Ok(dur.0.as_secs_f64().to_string() + "s")
        })
    }
}

async fn sleep(_lua: &Lua, dur: LuaDuration) -> mlua::Result<()> {
    tokio::time::sleep(dur.0).await;
    Ok(())
}

pub fn load_time(lua: &'static Lua) -> mlua::Result<mlua::Table<'static>> {
    lua.load_from_function(
        "time",
        lua.create_function(|_, ()| {
            let time = lua.create_table()?;

            time.set("nanosecond", LuaDuration(time::Duration::from_nanos(1)))?;
            time.set("millisecond", LuaDuration(time::Duration::from_millis(1)))?;
            time.set("second", LuaDuration(time::Duration::from_secs(1)))?;
            time.set("minute", LuaDuration(time::Duration::from_secs(60)))?;
            time.set("hour", LuaDuration(time::Duration::from_secs(60 * 60)))?;

            time.set("sleep", lua.create_async_function(sleep)?)?;

            Ok(time)
        })?,
    )
}
