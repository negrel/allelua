use tokio::io::AsyncReadExt;

use super::{Closable, LuaJitBuffer, DEFAULT_BUFFER_SIZE};

pub fn add_io_read_methods<
    R: AsyncReadExt + Unpin,
    C: AsRef<Closable<R>> + 'static,
    M: mlua::UserDataMethods<C>,
>(
    methods: &mut M,
) {
    methods.add_async_method(
        "read",
        |_, reader, (buf, reserve): (LuaJitBuffer, Option<usize>)| async move {
            let mut reader = reader.as_ref().get().await?;
            let bytes = buf.reserve_bytes(reserve.unwrap_or(0))?;
            let read = reader.read(bytes).await?;
            buf.commit(read)?;

            Ok(read)
        },
    );

    methods.add_async_method("read_to_end", |_, reader, buf: LuaJitBuffer| async move {
        let mut reader = reader.as_ref().get().await?;
        let mut bytes = buf.reserve_bytes(0)?;

        loop {
            let read = reader.read(bytes).await?;
            if read == 0 {
                break;
            }
            buf.commit(read)?;
            // LuaJIT automatically grow to next power of 2, so we reserve at
            // least DEFAULT_BUFFER_SIZE but it reserve more.
            bytes = buf.reserve_bytes(DEFAULT_BUFFER_SIZE)?;
        }

        Ok(())
    });
}
