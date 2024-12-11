use std::ops::Deref;

use mlua::{Either, FromLua, Lua, MetaMethod, UserData, UserDataRef};
use num_bigint::BigInt;

use crate::include_lua;

pub fn load_math(lua: &Lua) -> mlua::Result<()> {
    let big_int = lua.create_table()?;
    big_int.set(
        "fromnumber",
        lua.create_function(|_, n: mlua::Integer| Ok(LuaBigInt(BigInt::from(n))))?,
    )?;

    lua.load(include_lua!("./math.lua"))
        .eval::<mlua::Function>()?
        .call(big_int)
}

#[derive(Debug, Clone)]
struct LuaBigInt(BigInt);

impl Deref for LuaBigInt {
    type Target = BigInt;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromLua for LuaBigInt {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let either = Either::<mlua::Integer, UserDataRef<LuaBigInt>>::from_lua(value, lua)?;
        match either {
            Either::Left(i) => Ok(Self(BigInt::from(i))),
            Either::Right(bi) => Ok(bi.to_owned()),
        }
    }
}

impl UserData for LuaBigInt {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "math.BigInt");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Unm, |_, bigint, ()| Ok(Self(-bigint.0.clone())));
        methods.add_meta_method(MetaMethod::Add, |_, bigint, rhs: Self| {
            Ok(Self(bigint.0.clone() + rhs.0))
        });
        methods.add_meta_method(MetaMethod::Sub, |_, bigint, rhs: Self| {
            Ok(Self(bigint.0.clone() - rhs.0))
        });
        methods.add_meta_method(MetaMethod::Div, |_, bigint, rhs: Self| {
            Ok(Self(bigint.0.clone() / rhs.0))
        });
        methods.add_meta_method(MetaMethod::Mul, |_, bigint, rhs: Self| {
            Ok(Self(bigint.0.clone() * rhs.0))
        });

        methods.add_meta_method(MetaMethod::Lt, |_, bigint, rhs: Self| Ok(bigint.0 < rhs.0));
        methods.add_meta_method(MetaMethod::Le, |_, bigint, rhs: Self| Ok(bigint.0 <= rhs.0));
        methods.add_meta_method(MetaMethod::Eq, |_, bigint, rhs: Self| Ok(bigint.0 == rhs.0));

        methods.add_meta_method(MetaMethod::ToString, |_, bigint, ()| {
            Ok(bigint.0.to_string())
        });
    }
}
