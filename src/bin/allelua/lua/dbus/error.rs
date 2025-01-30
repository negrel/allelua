use std::ops::Deref;

use crate::lua::error::{self, AlleluaError};

#[derive(Debug, thiserror::Error)]
#[error("dbus.Error(kind={})", self.kind())]
pub struct LuaError(#[from] dbus::Error);

impl Deref for LuaError {
    type Target = dbus::Error;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<LuaError> for mlua::Error {
    fn from(val: LuaError) -> Self {
        error::LuaError::from(val).into()
    }
}

impl error::AlleluaError for LuaError {
    fn type_name(&self) -> &str {
        "dbus.Error"
    }

    fn kind(&self) -> &str {
        self.0.name().unwrap_or("unknown")
    }
}
