use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{
    io::{AsyncWriteExt, BufWriter},
    process::ChildStdin,
};

use crate::lua::{
    io::{add_io_write_close_methods, Closable, MaybeBuffered},
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdin<T: MaybeBuffered<ChildStdin>>(Closable<T>);

impl LuaChildStdin<ChildStdin> {
    pub fn new(stdin: ChildStdin) -> Self {
        Self(Closable::new(stdin))
    }
}

impl LuaChildStdin<BufWriter<ChildStdin>> {
    pub fn new_buffered(stdin: ChildStdin, buf_size: Option<usize>) -> Self {
        let buf_writer = match buf_size {
            Some(n) => BufWriter::with_capacity(n, stdin),
            None => BufWriter::new(stdin),
        };

        Self(Closable::new(buf_writer))
    }
}

impl<T: MaybeBuffered<ChildStdin>> TryIntoStdio for LuaChildStdin<T> {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stdin: ChildStdin = self.0.into_inner()?.into_inner();
        Ok(stdin.try_into()?)
    }
}

// LuaChildStdin<T> implements io.WriteCloser and os.TryIntoStdio.
impl<T: MaybeBuffered<ChildStdin> + AsyncWriteExt + Unpin + 'static> LuaInterface
    for LuaChildStdin<T>
{
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl<T: MaybeBuffered<ChildStdin>> AsRef<Closable<T>> for LuaChildStdin<T> {
    fn as_ref(&self) -> &Closable<T> {
        &self.0
    }
}

impl<T: MaybeBuffered<ChildStdin>> AsMut<Closable<T>> for LuaChildStdin<T> {
    fn as_mut(&mut self) -> &mut Closable<T> {
        &mut self.0
    }
}

impl<T: MaybeBuffered<ChildStdin> + 'static> UserData for LuaChildStdin<T>
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdin")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        Self::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdin, ()| {
            let address = stdin as *const _ as usize;
            Ok(format!("ChildStdin 0x{address:x}"))
        })
    }
}
