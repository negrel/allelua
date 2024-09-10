use std::{fmt::Display, io, ops::Deref, sync::Arc};

use mlua::{FromLua, UserData};

#[derive(Debug)]
pub struct LuaError(pub io::Error);

impl Deref for LuaError {
    type Target = io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<io::Error> for LuaError {
    fn from(value: io::Error) -> Self {
        Self(value)
    }
}

impl From<LuaError> for mlua::Error {
    fn from(val: LuaError) -> Self {
        mlua::Error::external(Box::new(Arc::new(val)))
    }
}

impl Display for LuaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for LuaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        #[allow(deprecated)]
        self.0.description()
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.0.source()
    }
}

impl UserData for LuaError {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "IoError")
    }

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_method("kind", |_lua, err, ()| Ok(LuaErrorKind(err.0.kind())))
    }
}

#[derive(Debug, Clone, Copy, FromLua)]
pub struct LuaErrorKind(pub io::ErrorKind);

impl From<io::ErrorKind> for LuaErrorKind {
    fn from(value: io::ErrorKind) -> Self {
        Self(value)
    }
}

impl UserData for LuaErrorKind {
    fn add_fields<'lua, F: mlua::prelude::LuaUserDataFields<'lua, Self>>(_fields: &mut F) {}

    fn add_methods<'lua, M: mlua::prelude::LuaUserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, errkind, ()| {
            Ok(io::Error::from(errkind.0).to_string())
        });

        methods.add_meta_method(mlua::MetaMethod::Eq, |_, errkind, other: LuaErrorKind| {
            Ok(errkind.0 == other.0)
        });
    }
}
