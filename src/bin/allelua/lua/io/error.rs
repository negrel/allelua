use std::{io, ops::Deref};

use thiserror::Error;

use crate::lua::error::{self, AlleluaError};

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
        error::LuaError::from(val).into()
    }
}

impl AlleluaError for LuaError {
    fn type_name(&self) -> &'static str {
        "io.Error"
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
