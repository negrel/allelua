use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{io::BufReader, process::ChildStderr};

use crate::lua::{
    io::{
        add_io_buf_read_methods, add_io_close_methods, add_io_read_methods, Closable, MaybeBuffered,
    },
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStderr<T: MaybeBuffered<ChildStderr>>(Closable<T>);

impl LuaChildStderr<ChildStderr> {
    pub fn new(stderr: ChildStderr) -> Self {
        Self(Closable::new(stderr))
    }
}

impl LuaChildStderr<BufReader<ChildStderr>> {
    pub fn new_buffered(stderr: ChildStderr, buf_size: Option<usize>) -> Self {
        let buf_reader = match buf_size {
            Some(n) => BufReader::with_capacity(n, stderr),
            None => BufReader::new(stderr),
        };

        Self(Closable::new(buf_reader))
    }
}

impl<T: MaybeBuffered<ChildStderr>> TryIntoStdio for LuaChildStderr<T> {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stderr: ChildStderr = self.0.into_inner()?.into_inner();
        Ok(stderr.try_into()?)
    }
}

// LuaChildStderr<ChildStderr> implements io.Reader, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaChildStderr<ChildStderr> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

// LuaChildStderr<ChildStderr> implements io.Reader, io.BufReader, io.Closer
// and os.TryIntoStdio.
impl LuaInterface for LuaChildStderr<BufReader<ChildStderr>> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_buf_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl<T: MaybeBuffered<ChildStderr>> AsRef<Closable<T>> for LuaChildStderr<T> {
    fn as_ref(&self) -> &Closable<T> {
        &self.0
    }
}

impl<T: MaybeBuffered<ChildStderr>> AsMut<Closable<T>> for LuaChildStderr<T> {
    fn as_mut(&mut self) -> &mut Closable<T> {
        &mut self.0
    }
}

impl<T: MaybeBuffered<ChildStderr> + 'static> UserData for LuaChildStderr<T>
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStderr");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stderr, ()| {
            let address = stderr as *const _ as usize;
            Ok(format!("ChildStderr 0x{address:x}"))
        })
    }
}
