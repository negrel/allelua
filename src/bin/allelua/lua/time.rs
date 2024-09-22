use core::time;
use std::{ops::Deref, time::Duration};

use mlua::{chunk, FromLua, Lua, UserData};
use tokio::{task::spawn_blocking, time::Instant};

#[derive(Clone, Copy, FromLua)]
struct LuaDuration(time::Duration);

impl Deref for LuaDuration {
    type Target = time::Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaDuration {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "Duration");
    }

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
        methods.add_meta_method(mlua::MetaMethod::Sub, |_lua, dur1, dur2: LuaDuration| {
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
            if dur.as_secs() < 1 {
                // Special case: if duration is smaller than a second,
                // use smaller units, like 1.2ms
                let ns = dur.as_nanos();
                let us = dur.as_micros();
                let ms = dur.as_millis();

                if dur.0 == Duration::ZERO {
                    Ok("0s".to_owned())
                } else if us == 0 {
                    // nanoseconds
                    Ok(format!("{ns}ns"))
                } else if ms == 0 {
                    // microseconds
                    let ns = ns % 1000;
                    if ns == 0 {
                        Ok(format!("{us}µs"))
                    } else {
                        Ok(format!("{us}.{ns:03}µs"))
                    }
                } else {
                    let ns = ns % 1_000_000;
                    if ns == 0 {
                        Ok(format!("{ms}ms"))
                    } else {
                        let (ns, zeros) = remove_trailing_zeros(ns);
                        let prec = 6 - zeros;
                        Ok(format!("{ms}.{ns:0prec$}ms"))
                    }
                }
            } else {
                let (ns, _) = remove_trailing_zeros(dur.as_nanos() % 1_000_000_000);

                let hours = dur.as_secs() / 3600;
                let minutes = (dur.as_secs() % 3600) / 60;
                let seconds = dur.as_secs() % 60;

                if ns != 0 {
                    match (hours, minutes, seconds) {
                        (0, 0, s) => Ok(format!("{s}.{ns}s")),
                        (0, m, s) => Ok(format!("{m}m{s}.{ns}s")),
                        (h, 0, s) => Ok(format!("{h}h{s}.{ns}s")),
                        (h, m, s) => Ok(format!("{h}h{m}m{s}.{ns}s")),
                    }
                } else {
                    match (hours, minutes, seconds) {
                        (0, 0, s) => Ok(format!("{s}s")),
                        (0, m, 0) => Ok(format!("{m}m")),
                        (0, m, s) => Ok(format!("{m}m{s}s")),
                        (h, 0, 0) => Ok(format!("{h}h")),
                        (h, 0, s) => Ok(format!("{h}h{s}s")),
                        (h, m, 0) => Ok(format!("{h}h{m}m")),
                        (h, m, s) => Ok(format!("{h}h{m}m{s}s")),
                    }
                }
            }
        })
    }
}

#[derive(Clone, Copy, FromLua)]
pub struct LuaInstant(Instant);

impl Deref for LuaInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaInstant {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "Instant");
    }

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(
            mlua::MetaMethod::Eq,
            |_lua, instant1, instant2: LuaInstant| Ok(instant1.0 == instant2.0),
        );

        methods.add_meta_method(
            mlua::MetaMethod::Lt,
            |_lua, instant1, instant2: LuaInstant| Ok(instant1.0 < instant2.0),
        );

        methods.add_meta_method(
            mlua::MetaMethod::Le,
            |_lua, instant1, instant2: LuaInstant| Ok(instant1.0 <= instant2.0),
        );

        methods.add_meta_method_mut(mlua::MetaMethod::Add, |_lua, instant, dur: LuaDuration| {
            Ok(LuaInstant(instant.0 + dur.0))
        });

        methods.add_meta_method_mut(mlua::MetaMethod::Sub, |_lua, instant, dur: LuaDuration| {
            Ok(LuaInstant(instant.0 - dur.0))
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, instant, ()| {
            let address = instant as *const _ as usize;
            Ok(format!("Instant Ox{address:x}"))
        });

        methods.add_method("elapsed", |_lua, instant, ()| {
            Ok(LuaDuration(instant.elapsed()))
        });

        methods.add_method("duration_since", |_lua, instant, other: LuaInstant| {
            Ok(LuaDuration(instant.duration_since(*other)))
        });
    }
}

pub fn load_time(lua: &'static Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "time",
        lua.create_function(|lua, ()| {
            let time = lua.create_table()?;

            time.set("nanosecond", LuaDuration(time::Duration::from_nanos(1)))?;
            time.set("microsecond", LuaDuration(time::Duration::from_micros(1)))?;
            time.set("millisecond", LuaDuration(time::Duration::from_millis(1)))?;
            time.set("second", LuaDuration(time::Duration::from_secs(1)))?;
            time.set("minute", LuaDuration(time::Duration::from_secs(60)))?;
            time.set("hour", LuaDuration(time::Duration::from_secs(60 * 60)))?;
            time.set(
                "sleep",
                lua.create_async_function(|_, dur: LuaDuration| async move {
                    tokio::time::sleep(dur.0).await;
                    Ok(())
                })?,
            )?;

            let instant = lua.create_table()?;
            instant.set(
                "now",
                lua.create_async_function(|_, ()| async {
                    let instant = spawn_blocking(Instant::now)
                        .await
                        .map_err(mlua::Error::runtime)?;
                    Ok(LuaInstant(instant))
                })?,
            )?;
            time.set("Instant", instant.clone())?;

            let sleep = time.get::<_, mlua::Function>("sleep")?;
            time.set(
                "after",
                lua.load(chunk! {
                    return function(dur)
                        local tx, rx = require("sync").channel();
                        return rx, go(function()
                            $sleep(dur)
                            tx:send(nil)
                        end)
                    end
                })
                .eval::<mlua::Function<'static>>()?,
            )?;

            Ok(time)
        })?,
    )
}

fn remove_trailing_zeros(mut n: u128) -> (u128, usize) {
    if n == 0 {
        return (0, 0);
    }
    let mut i = 0;
    while n % 10 == 0 {
        n /= 10;
        i += 1;
    }
    (n, i)
}
