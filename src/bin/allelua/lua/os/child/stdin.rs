use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::process::ChildStdin;

use crate::lua::{
    io::{add_io_write_close_methods, Closable},
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdin(Closable<ChildStdin>);

impl LuaChildStdin {
    pub fn new(stdin: ChildStdin) -> Self {
        Self(Closable::new(stdin))
    }
}

impl TryIntoStdio for LuaChildStdin {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stdin: ChildStdin = self.0.into_inner()?;
        Ok(stdin.try_into()?)
    }
}

// LuaChildStdin<ChildStdin> implements io.WriteCloser and os.TryIntoStdio.
impl LuaInterface for LuaChildStdin {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl AsRef<Closable<ChildStdin>> for LuaChildStdin {
    fn as_ref(&self) -> &Closable<ChildStdin> {
        &self.0
    }
}

impl AsMut<Closable<ChildStdin>> for LuaChildStdin {
    fn as_mut(&mut self) -> &mut Closable<ChildStdin> {
        &mut self.0
    }
}

impl UserData for LuaChildStdin
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdin");
        fields.add_field_method_get("closed", |_, stdin| Ok(stdin.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        Self::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdin, ()| {
            let address = stdin as *const _ as usize;
            Ok(format!(
                "ChildStdin(closed={}) 0x{address:x}",
                stdin.as_ref().is_closed()
            ))
        })
    }
}
