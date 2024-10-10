use mlua::{AnyUserData, ObjectLike};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

use super::{Closable, LuaBuffer, LuaJitBuffer};

pub fn add_io_read_methods<
    W: AsyncReadExt + Unpin,
    R: AsRef<Closable<W>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method(
        "read",
        |_, reader, (buf, reserve): (LuaJitBuffer, Option<usize>)| async move {
            let mut reader = reader.as_ref().get().await?;

            let bytes = buf.reserve_bytes(reserve.unwrap_or(4096))?;

            let read = reader.read(bytes).await?;

            buf.commit(read)?;

            Ok(read)
        },
    );

    methods.add_async_method("read_to_end", |lua, reader, ()| async move {
        let mut reader = reader.as_ref().get().await?;

        let mut buf = Vec::new();

        reader.read_to_end(&mut buf).await?;

        lua.create_string(buf)
    });
}

pub fn add_io_buf_read_methods<
    W: AsyncBufReadExt + Unpin,
    R: AsRef<Closable<W>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method("write_to", |_, reader, writer: AnyUserData| async move {
        let mut reader = reader.as_ref().get().await?;

        let buf = reader.fill_buf().await?;
        if buf.is_empty() {
            return Ok(0);
        }

        // Safety: this is safe as buf won't be dropped until end of
        // function but mlua required static args.
        let buf = unsafe { LuaBuffer::new_static(buf) };
        let write = buf.as_bytes().len();

        // write_buf is implemented by all writers.
        writer
            .get::<mlua::Function>("write_buf")?
            .call_async::<()>((writer, buf))
            .await?;

        // Move buf reader internal cursor.
        reader.consume(write);

        Ok(write)
    });

    methods.add_async_method_mut("read_until", |lua, reader, byte: u8| async move {
        let mut reader = reader.as_ref().get().await?;

        let mut buf = Vec::with_capacity(4096);
        let read = reader.read_until(byte, &mut buf).await?;

        let slice = &buf[..];

        if read == 0 {
            Ok(mlua::Value::Nil)
        } else {
            Ok(mlua::Value::String(lua.create_string(slice)?))
        }
    });

    methods.add_async_method_mut("read_line", |lua, reader, ()| async move {
        let mut reader = reader.as_ref().get().await?;
        let mut buf = Vec::with_capacity(4096);
        let read = reader.read_until(b'\n', &mut buf).await?;

        let mut slice = &buf[..];

        // Remove LF (\n).
        if buf.last().is_some() {
            slice = &slice[..slice.len() - 1];
        }

        // Remove CR from CRLF.
        if let Some(b'\r') = buf.last() {
            slice = &slice[..slice.len() - 1];
        }

        if read == 0 {
            Ok(mlua::Value::Nil)
        } else {
            Ok(mlua::Value::String(lua.create_string(slice)?))
        }
    });
}
