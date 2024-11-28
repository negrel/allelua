use mlua::{IntoLua, Lua};

mod reader;
mod writer;

use reader::*;
use writer::*;

pub(super) async fn pipe(lua: Lua, _: ()) -> mlua::Result<(mlua::Value, mlua::Value)> {
    let (reader, writer) = tokio::task::spawn_blocking(os_pipe::pipe)
        .await
        .map_err(mlua::Error::external)??;

    let reader = LuaPipeReader::new(reader).into_lua(&lua)?;
    let writer = LuaPipeWriter::new(writer).into_lua(&lua)?;

    Ok((reader, writer))
}
