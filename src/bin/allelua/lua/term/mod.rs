use mlua::{IntoLua, Lua, UserDataRef};

mod cmds;
mod colors;
mod event;

use cmds::*;
use event::*;
use rustyline::DefaultEditor;

use super::{
    error::{AlleluaError, LuaError},
    io,
    os::LuaFile,
};

pub fn load_term(lua: &Lua) -> mlua::Result<mlua::Table> {
    lua.load_from_function(
        "term",
        lua.create_function(|lua, ()| {
            let term = lua.create_table()?;
            lua.globals().set("term", term.clone())?;

            // crossterm::terminal
            {
                term.set(
                    "mode",
                    lua.create_function(|lua, ()| {
                        if crossterm::terminal::is_raw_mode_enabled()? {
                            "raw".into_lua(lua)
                        } else {
                            "cooked".into_lua(lua)
                        }
                    })?,
                )?;

                term.set(
                    "size",
                    lua.create_function(|_, ()| Ok(crossterm::terminal::size()?))?,
                )?;

                term.set(
                    "window_size",
                    lua.create_function(|lua, ()| {
                        let wsize = crossterm::terminal::window_size()?;
                        let table = lua.create_table_with_capacity(0, 4)?;

                        table.set("rows", wsize.rows)?;
                        table.set("columns", wsize.columns)?;
                        table.set("width", wsize.width)?;
                        table.set("height", wsize.height)?;

                        Ok(table)
                    })?,
                )?;

                term.set(
                    "enable_raw_mode",
                    lua.create_function(|_lua, ()| {
                        crossterm::terminal::enable_raw_mode()?;
                        Ok(())
                    })?,
                )?;
                term.set(
                    "disable_raw_mode",
                    lua.create_function(|_lua, ()| {
                        crossterm::terminal::disable_raw_mode()?;
                        Ok(())
                    })?,
                )?;
                term.set(
                    "is_raw_mode_enabled",
                    lua.create_function(
                        |_lua, ()| Ok(crossterm::terminal::is_raw_mode_enabled()?),
                    )?,
                )?;

                term.set(
                    "supports_keyboard_enhancement",
                    lua.create_function(|_lua, ()| {
                        Ok(crossterm::terminal::supports_keyboard_enhancement()?)
                    })?,
                )?;
            }

            // crossterm::cursor
            {
                term.set(
                    "position",
                    lua.create_async_function(|_lua, ()| async move {
                        tokio::task::spawn_blocking(|| {
                            Ok(crossterm::cursor::position().map_err(io::LuaError::from)?)
                        })
                        .await
                        .map_err(mlua::Error::external)?
                        .map(|(col, row)| (col + 1, row + 1))
                    })?,
                )?;
            }

            // crossterm::event
            {
                let event = lua.create_table()?;

                event.set("stream", LuaEventStream::default())?;

                term.set("event", event)?;
            }

            let queue = lua.create_table()?;
            queue.set(
                "new",
                lua.create_async_function(|_, f: UserDataRef<LuaFile>| async move {
                    LuaQueue::from_lua_file(&f).await
                })?,
            )?;
            term.set("Queue", queue)?;

            term.set(
                "read_line",
                lua.create_function(|_lua, prompt: Option<mlua::String>| {
                    let cfg = rustyline::Config::builder().build();
                    let mut ed = DefaultEditor::with_config(cfg)
                        .map_err(LuaReadlineError::from)
                        .map_err(LuaError::from)?;

                    if let Some(prompt) = prompt {
                        Ok(ed
                            .readline(prompt.to_str()?.as_ref())
                            .map_err(LuaReadlineError::from)
                            .map_err(LuaError::from)?)
                    } else {
                        Ok(ed
                            .readline("> ")
                            .map_err(LuaReadlineError::from)
                            .map_err(LuaError::from)?)
                    }
                })?,
            )?;

            Ok(term)
        })?,
    )
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
