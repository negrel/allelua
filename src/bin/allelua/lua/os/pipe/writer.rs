use std::{
    os::fd::{FromRawFd, IntoRawFd},
    process::Stdio,
};

use mlua::{MetaMethod, UserData};
use os_pipe::PipeWriter;
use tokio::fs::File;

use crate::lua::{
    io::{add_io_write_close_methods, Closable},
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

// We use File instead of [os_pipe::PipeWriter] as it doesn't implements
// [tokio::io::AsyncRead].
#[derive(Debug)]
pub(super) struct LuaPipeWriter(Closable<File>);

impl LuaPipeWriter {
    pub fn new(pipe_writer: PipeWriter) -> Self {
        // TODO: windows
        // Safety: We own the opened fd.
        let f = unsafe { File::from_raw_fd(pipe_writer.into_raw_fd()) };
        Self(Closable::new(f))
    }
}

impl TryIntoStdio for LuaPipeWriter {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let file: File = self.0.into_inner()?;
        let std_file = file.into_std().await;
        Ok(std_file.into())
    }
}

impl AsRef<Closable<File>> for LuaPipeWriter {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaPipeWriter {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}

// LuaPipeWriter<T> implements io.WriteCloser and os.TryIntoStdio.
impl LuaInterface for LuaPipeWriter {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl UserData for LuaPipeWriter
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.PipeWriter");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            if f.as_ref().is_closed() {
                Ok(format!("PipeWriter(state=close) 0x{address:x}"))
            } else {
                Ok(format!("PipeWriter(state=open) 0x{address:x}"))
            }
        });
    }
}
