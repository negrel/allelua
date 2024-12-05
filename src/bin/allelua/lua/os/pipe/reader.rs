use std::{
    os::fd::{FromRawFd, IntoRawFd},
    process::Stdio,
};

use mlua::{MetaMethod, UserData};
use os_pipe::PipeReader;
use tokio::fs::File;

use crate::lua::{
    io::{self, add_io_close_methods, add_io_read_methods, Closable},
    os::{add_os_try_as_stdio_methods, TryAsStdio},
    LuaInterface,
};

// We use File instead of [os_pipe::PipeReader] as it doesn't implements
// [tokio::io::AsyncRead].
#[derive(Debug)]
pub(super) struct LuaPipeReader(Closable<File>);

impl LuaPipeReader {
    pub fn new(pipe_reader: PipeReader) -> Self {
        // TODO: windows
        // Safety: We own the opened fd.
        let f = unsafe { File::from_raw_fd(pipe_reader.into_raw_fd()) };
        Self(Closable::new(f))
    }
}

impl TryAsStdio for LuaPipeReader {
    async fn try_as_stdio(&self) -> mlua::Result<Stdio> {
        let file = self
            .0
            .get()
            .await?
            .try_clone()
            .await
            .map_err(io::LuaError::from)?;
        let std_file = file.into_std().await;
        Ok(std_file.into())
    }
}

impl AsRef<Closable<File>> for LuaPipeReader {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaPipeReader {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}

// LuaPipeReader<File> implements io.Reader, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaPipeReader {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_as_stdio_methods(methods);
    }
}

impl UserData for LuaPipeReader
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.PipeReader");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            Ok(format!(
                "os.PipeReader(closed={}) 0x{address:x}",
                f.as_ref().is_closed()
            ))
        });
    }
}
