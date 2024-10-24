use std::ops::Deref;

use mlua::{FromLua, UserData};
use tokio::time::Duration;

#[derive(Clone, Copy, FromLua)]
pub struct LuaDuration(Duration);

impl From<Duration> for LuaDuration {
    fn from(value: Duration) -> Self {
        Self(value)
    }
}

impl Deref for LuaDuration {
    type Target = Duration;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaDuration {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "time.Duration");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
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
                let (ms, zeros) = remove_trailing_zeros(dur.as_millis() % 1_000);
                let prec = 3 - zeros;

                let hours = dur.as_secs() / 3600;
                let minutes = (dur.as_secs() % 3600) / 60;
                let seconds = dur.as_secs() % 60;

                if ms != 0 {
                    match (hours, minutes, seconds) {
                        (0, 0, s) => Ok(format!("{s}.{ms:0prec$}s")),
                        (0, m, s) => Ok(format!("{m}m{s}.{ms:0prec$}s")),
                        (h, 0, s) => Ok(format!("{h}h{s}.{ms:0prec$}s")),
                        (h, m, s) => Ok(format!("{h}h{m}m{s}.{ms:0prec$}s")),
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
