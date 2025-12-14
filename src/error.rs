use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    LuaError(mlua::Error),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::LuaError(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::LuaError(error) => Some(error),
        }
    }
}

impl From<mlua::Error> for Error {
    fn from(value: mlua::Error) -> Self {
        Self::LuaError(value)
    }
}
