use std::{io, ops::Deref};

use thiserror::Error;

use crate::lua::error::{self, AlleluaError};

#[derive(Debug, Error)]
#[error("io.Error(kind={})", self.kind())]
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
            io::ErrorKind::NotFound => "not_found",
            io::ErrorKind::PermissionDenied => "permission_denied",
            io::ErrorKind::ConnectionRefused => "connection_refused",
            io::ErrorKind::ConnectionReset => "connection_reset",
            io::ErrorKind::ConnectionAborted => "connection_aborted",
            io::ErrorKind::NotConnected => "not_connected",
            io::ErrorKind::AddrInUse => "addr_in_use",
            io::ErrorKind::AddrNotAvailable => "addr_not_available",
            io::ErrorKind::BrokenPipe => "broken_pipe",
            io::ErrorKind::AlreadyExists => "already_exists",
            io::ErrorKind::WouldBlock => "would_block",
            io::ErrorKind::InvalidInput => "invalid_input",
            io::ErrorKind::InvalidData => "invalid_data",
            io::ErrorKind::TimedOut => "timed_out",
            io::ErrorKind::WriteZero => "write_zero",
            io::ErrorKind::Interrupted => "interrupted",
            io::ErrorKind::Unsupported => "unsupported",
            io::ErrorKind::UnexpectedEof => "unexpected_eof",
            io::ErrorKind::OutOfMemory => "out_of_memory",
            io::ErrorKind::Other => "other",
            _ => "unknown",
        }
    }
}
