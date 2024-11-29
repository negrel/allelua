use std::ops::Deref;

use mlua::{FromLua, MetaMethod, UserData, UserDataRef};

/// LuaRegex define a Lua regular expression.
#[derive(Debug, Clone)]
pub struct LuaRegex(regex::bytes::Regex);

impl Deref for LuaRegex {
    type Target = regex::bytes::Regex;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaRegex {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "string.Regex")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, re, ()| {
            Ok(format!("string.Regex({})", re.0.as_str()))
        });
    }
}

impl FromLua for LuaRegex {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let either = mlua::Either::<mlua::String, UserDataRef<LuaRegex>>::from_lua(value, lua)?;

        match either {
            mlua::Either::Left(str) => Ok(LuaRegex(
                regex::bytes::Regex::new(&regex::escape(&str.to_str()?))
                    .map_err(mlua::Error::external)?,
            )),
            mlua::Either::Right(re) => Ok(re.to_owned()),
        }
    }
}

impl LuaRegex {
    pub fn new(re: &str) -> Result<Self, regex::Error> {
        Ok(Self(regex::bytes::Regex::new(re)?))
    }
}
