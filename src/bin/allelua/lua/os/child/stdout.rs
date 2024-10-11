use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{io::BufReader, process::ChildStdout};

use crate::lua::{
    io::{
        add_io_buf_read_methods, add_io_close_methods, add_io_read_methods, Closable, MaybeBuffered,
    },
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdout<T: MaybeBuffered<ChildStdout>>(Closable<T>);

impl LuaChildStdout<ChildStdout> {
    pub fn new(stdout: ChildStdout) -> Self {
        Self(Closable::new(stdout))
    }
}

impl LuaChildStdout<BufReader<ChildStdout>> {
    pub fn new_buffered(stdout: ChildStdout, buf_size: Option<usize>) -> Self {
        let buf_reader = match buf_size {
            Some(n) => BufReader::with_capacity(n, stdout),
            None => BufReader::new(stdout),
        };

        Self(Closable::new(buf_reader))
    }
}

impl<T: MaybeBuffered<ChildStdout>> TryIntoStdio for LuaChildStdout<T> {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stdout: ChildStdout = self.0.into_inner()?.into_inner();
        Ok(stdout.try_into()?)
    }
}

// LuaChildStdout<ChildStdout> implements io.Reader and io.Closer.
impl LuaInterface for LuaChildStdout<ChildStdout> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

// LuaChildStdout<ChildStdout> implements io.Reader, io.BufReader and io.Closer.
impl LuaInterface for LuaChildStdout<BufReader<ChildStdout>> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_buf_read_methods(methods);
        add_io_close_methods(methods);
    }
}

impl<T: MaybeBuffered<ChildStdout>> AsRef<Closable<T>> for LuaChildStdout<T> {
    fn as_ref(&self) -> &Closable<T> {
        &self.0
    }
}

impl<T: MaybeBuffered<ChildStdout>> AsMut<Closable<T>> for LuaChildStdout<T> {
    fn as_mut(&mut self) -> &mut Closable<T> {
        &mut self.0
    }
}
impl<T: MaybeBuffered<ChildStdout> + 'static> UserData for LuaChildStdout<T>
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdout");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdout, ()| {
            let address = stdout as *const _ as usize;
            Ok(format!("ChildStdout 0x{address:x}"))
        })
    }
}
