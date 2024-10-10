use std::{
    os::fd::{FromRawFd, IntoRawFd},
    process::Stdio,
};

use mlua::{MetaMethod, UserData};
use os_pipe::PipeReader;
use tokio::{fs::File, io::BufReader};

use crate::lua::{
    io::{
        add_io_buf_read_methods, add_io_close_methods, add_io_read_methods, Closable, MaybeBuffered,
    },
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

// We use File instead of [os_pipe::PipeReader] as it doesn't implements
// [tokio::io::AsyncRead].
#[derive(Debug)]
pub(super) struct LuaPipeReader<T: MaybeBuffered<File>>(Closable<T>);

impl LuaPipeReader<File> {
    pub fn new(pipe_reader: PipeReader) -> Self {
        // TODO: windows
        // Safety: We own the opened fd.
        let f = unsafe { File::from_raw_fd(pipe_reader.into_raw_fd()) };
        Self(Closable::new(f))
    }
}

impl LuaPipeReader<BufReader<File>> {
    pub fn new_buffered(pipe_reader: PipeReader, buffer_size: Option<usize>) -> Self {
        // TODO: windows
        // Safety: We own the opened fd.
        let f = unsafe { File::from_raw_fd(pipe_reader.into_raw_fd()) };
        let buf_reader = match buffer_size {
            Some(n) => BufReader::with_capacity(n, f),
            None => BufReader::new(f),
        };
        Self(Closable::new(buf_reader))
    }
}

impl<T: MaybeBuffered<File>> TryIntoStdio for LuaPipeReader<T> {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let file: File = self.0.into_inner()?.into_inner();
        let std_file = file.into_std().await;
        Ok(std_file.into())
    }
}

impl<T: MaybeBuffered<File>> AsRef<Closable<T>> for LuaPipeReader<T> {
    fn as_ref(&self) -> &Closable<T> {
        &self.0
    }
}

impl<T: MaybeBuffered<File>> AsMut<Closable<T>> for LuaPipeReader<T> {
    fn as_mut(&mut self) -> &mut Closable<T> {
        &mut self.0
    }
}

// LuaPipeReader<File> implements io.Reader, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaPipeReader<File> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

// LuaPipeReader<File> implements io.Reader, io.BufReader, io.Closer and
// os.TryIntoStdio.
impl LuaInterface for LuaPipeReader<BufReader<File>> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_buf_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl<T: MaybeBuffered<File> + 'static> UserData for LuaPipeReader<T>
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "PipeReader");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            if f.as_ref().is_closed() {
                Ok(format!("PipeReader(state=close) 0x{address:x}"))
            } else {
                Ok(format!("PipeReader(state=open) 0x{address:x}"))
            }
        });
    }
}
