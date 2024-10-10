use mlua::{IntoLua, Lua};

mod reader;
mod writer;

use reader::*;
use writer::*;

pub(super) async fn pipe(
    lua: Lua,
    (reader_buffer_size, writer_buffer_size): (Option<usize>, Option<usize>),
) -> mlua::Result<(mlua::Value, mlua::Value)> {
    let (reader, writer) = tokio::task::spawn_blocking(os_pipe::pipe)
        .await
        .map_err(mlua::Error::external)??;

    let reader = match reader_buffer_size {
        Some(0) => LuaPipeReader::new(reader).into_lua(&lua)?,
        None | Some(_) => LuaPipeReader::new_buffered(reader, reader_buffer_size).into_lua(&lua)?,
    };
    let writer = match writer_buffer_size {
        Some(0) => LuaPipeWriter::new(writer).into_lua(&lua)?,
        None | Some(_) => LuaPipeWriter::new_buffered(writer, writer_buffer_size).into_lua(&lua)?,
    };

    Ok((reader, writer))
}
