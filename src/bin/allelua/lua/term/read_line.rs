use std::ops::{Deref, DerefMut};

use mlua::{MetaMethod, UserData};
use rustyline::{
    history::{FileHistory, History},
    Config, Editor, Helper,
};

use crate::{
    lua::error::{AlleluaError, LuaError},
    lua_string_as_path,
};

#[derive(Debug)]
pub struct LuaReadLine<H: Helper, I: History>(rustyline::Editor<H, I>);

impl<H: Helper, I: History> Deref for LuaReadLine<H, I> {
    type Target = rustyline::Editor<H, I>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<H: Helper, I: History> DerefMut for LuaReadLine<H, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<H: Helper, I: History> UserData for LuaReadLine<H, I> {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.ReadLine");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("read_line", |_, rl, prompt: Option<mlua::String>| {
            let line = if let Some(prompt) = prompt {
                rl.readline(prompt.to_str()?.as_ref())
            } else {
                rl.readline("> ")
            };

            Ok(line
                .map_err(LuaReadlineError::from)
                .map_err(LuaError::from)?)
        });

        methods.add_method_mut("load_history", |_, rl, str: mlua::String| {
            lua_string_as_path!(path = str);
            rl.load_history(path)
                .map_err(LuaReadlineError::from)
                .map_err(LuaError::from)?;
            Ok(())
        });

        methods.add_method_mut("save_history", |_, rl, str: mlua::String| {
            lua_string_as_path!(path = str);
            rl.save_history(path)
                .map_err(LuaReadlineError::from)
                .map_err(LuaError::from)?;
            Ok(())
        });

        methods.add_method_mut("clear_history", |_, rl, ()| {
            rl.clear_history()
                .map_err(LuaReadlineError::from)
                .map_err(LuaError::from)?;
            Ok(())
        });

        methods.add_meta_method(MetaMethod::ToString, |_, q, ()| {
            let address = q as *const _ as usize;
            Ok(format!("term.ReadLine() 0x{address:x}"))
        });
    }
}

impl<H: Helper> LuaReadLine<H, FileHistory> {
    pub fn new() -> Result<Self, LuaError> {
        Ok(Self(
            Editor::with_config(Config::builder().auto_add_history(true).build())
                .map_err(LuaReadlineError::from)
                .map_err(LuaError::from)?,
        ))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("term.ReadLineError(kind={})", self.kind())]
pub struct LuaReadlineError(rustyline::error::ReadlineError);

impl From<rustyline::error::ReadlineError> for LuaReadlineError {
    fn from(value: rustyline::error::ReadlineError) -> Self {
        Self(value)
    }
}

impl From<LuaReadlineError> for mlua::Error {
    fn from(val: LuaReadlineError) -> Self {
        LuaError::from(val).into()
    }
}

impl AlleluaError for LuaReadlineError {
    fn type_name(&self) -> &str {
        "term.ReadlineError"
    }

    fn kind(&self) -> &str {
        match self.0 {
            rustyline::error::ReadlineError::Io(_) => "io",
            rustyline::error::ReadlineError::Eof => "eof",
            rustyline::error::ReadlineError::Interrupted => "interrupted",
            rustyline::error::ReadlineError::WindowResized => "window_resized",
            rustyline::error::ReadlineError::Errno(_) => "uncategorized",
            _ => "uncategorized",
        }
    }
}
