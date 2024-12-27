use mlua::{IntoLua, Lua, UserDataRef};

mod cmds;
mod colors;
mod event;
mod read_line;

use cmds::*;
use event::*;
use read_line::LuaReadLine;

use super::{io, os::LuaFile};

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

            let read_line_constructors = lua.create_table()?;
            read_line_constructors.set(
                "new",
                lua.create_function(|_, ()| Ok(LuaReadLine::<(), _>::new()?))?,
            )?;
            term.set("ReadLine", read_line_constructors)?;

            Ok(term)
        })?,
    )
}
