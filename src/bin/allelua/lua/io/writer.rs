use mlua::{UserDataRef, UserDataRefMut};
use tokio::io::AsyncWriteExt;

use super::{Closable, Close, LuaBuffer, LuaJitBuffer};

pub fn add_io_write_methods<
    W: AsyncWriteExt + Unpin,
    R: AsRef<Closable<W>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_async_method("write", |_, writer, buf: LuaJitBuffer| async move {
        let mut writer = writer.as_ref().get().await?;

        let write = writer.write(buf.as_bytes()?).await?;

        buf.skip(write)?;

        Ok(write)
    });

    methods.add_async_method("write_all", |_, writer, buf: LuaJitBuffer| async move {
        let mut writer = writer.as_ref().get().await?;

        let bytes = buf.as_bytes()?;

        writer.write_all(bytes).await?;

        buf.skip(bytes.len())?;
        Ok(bytes.len())
    });

    methods.add_async_method("flush", |_, writer, ()| async move {
        let mut writer = writer.as_ref().get().await?;
        writer.flush().await?;
        Ok(())
    });

    methods.add_async_method("write_buf", |_, writer, buf: LuaBuffer| async move {
        let mut writer = writer.as_ref().get().await?;
        writer.write_all(buf.as_bytes()).await?;
        Ok(())
    });
}

pub fn add_io_write_close_methods<
    T: AsyncWriteExt + Unpin + 'static,
    R: AsRef<Closable<T>> + AsMut<Closable<T>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    add_io_write_methods(methods);

    methods.add_async_function("close", |_, writer_closer: mlua::AnyUserData| async move {
        {
            let writer_closer: UserDataRef<R> = writer_closer.borrow()?;

            let mut writer_closer = writer_closer.as_ref().get().await?;
            writer_closer.flush().await?;
        }

        let mut writer_closer: UserDataRefMut<R> = writer_closer.borrow_mut()?;
        let closer = writer_closer.as_mut();

        closer.close()?;

        Ok(())
    });
}
