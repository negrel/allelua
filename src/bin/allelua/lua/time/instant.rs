use std::ops::Deref;

use mlua::{FromLua, UserData};
use tokio::time::Instant;

use super::LuaDuration;

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaInstant(Instant);

impl From<Instant> for LuaInstant {
    fn from(value: Instant) -> Self {
        Self(value)
    }
}

impl Deref for LuaInstant {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaInstant {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "time.Instant");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
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
            Ok(LuaInstant(instant.0 + *dur))
        });

        methods.add_meta_method_mut(mlua::MetaMethod::Sub, |_lua, instant, dur: LuaDuration| {
            Ok(LuaInstant(instant.0 - *dur))
        });

        methods.add_meta_method(mlua::MetaMethod::ToString, |_lua, instant, ()| {
            let address = instant as *const _ as usize;
            Ok(format!("Instant 0x{address:x}"))
        });

        methods.add_method("elapsed", |_lua, instant, ()| {
            Ok(LuaDuration::from(instant.elapsed()))
        });

        methods.add_method("duration_since", |_lua, instant, other: LuaInstant| {
            Ok(LuaDuration::from(instant.duration_since(*other)))
        });
    }
}
