use std::{io, ops::Deref, sync::Arc};

use mlua::{FromLua, UserData};
use thiserror::Error;

use super::errors::AlleluaError;

#[derive(Debug, Error)]
#[error(transparent)]
pub struct LuaError(#[from] pub io::Error);

impl Deref for LuaError {
    type Target = io::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<LuaError> for mlua::Error {
    fn from(val: LuaError) -> Self {
        mlua::Error::external(Box::new(Arc::new(val)))
    }
}

impl AlleluaError for LuaError {
    fn type_name(&self) -> &'static str {
        "IoError"
    }

    fn kind(&self) -> &'static str {
        match self.0.kind() {
            io::ErrorKind::NotFound => "NotFound",
            io::ErrorKind::PermissionDenied => "PermissionDenied",
            io::ErrorKind::ConnectionRefused => "ConnectionRefused",
            io::ErrorKind::ConnectionReset => "ConnectionReset",
            io::ErrorKind::ConnectionAborted => "ConnectionAborted",
            io::ErrorKind::NotConnected => "NotConnected",
            io::ErrorKind::AddrInUse => "AddrInUse",
            io::ErrorKind::AddrNotAvailable => "AddrNotAvailable",
            io::ErrorKind::BrokenPipe => "BrokenPipe",
            io::ErrorKind::AlreadyExists => "AlreadyExists",
            io::ErrorKind::WouldBlock => "WouldBlock",
            io::ErrorKind::InvalidInput => "InvalidInput",
            io::ErrorKind::InvalidData => "InvalidData",
            io::ErrorKind::TimedOut => "TimedOut",
            io::ErrorKind::WriteZero => "WriteZero",
            io::ErrorKind::Interrupted => "Interrupted",
            io::ErrorKind::Unsupported => "Unsupported",
            io::ErrorKind::UnexpectedEof => "UnexpectedEof",
            io::ErrorKind::OutOfMemory => "OutOfMemory",
            io::ErrorKind::Other => "Other",
            _ => "Unknown",
        }
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
