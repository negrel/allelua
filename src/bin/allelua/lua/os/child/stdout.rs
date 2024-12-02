use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::process::ChildStdout;

use crate::lua::{
    io::{add_io_close_methods, add_io_read_methods, Closable},
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdout(Closable<ChildStdout>);

impl LuaChildStdout {
    pub fn new(stdout: ChildStdout) -> Self {
        Self(Closable::new(stdout))
    }
}

impl TryIntoStdio for LuaChildStdout {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stdout: ChildStdout = self.0.into_inner()?;
        Ok(stdout.try_into()?)
    }
}

// LuaChildStdout<ChildStdout> implements io.Reader and io.Closer.
impl LuaInterface for LuaChildStdout {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl AsRef<Closable<ChildStdout>> for LuaChildStdout {
    fn as_ref(&self) -> &Closable<ChildStdout> {
        &self.0
    }
}

impl AsMut<Closable<ChildStdout>> for LuaChildStdout {
    fn as_mut(&mut self) -> &mut Closable<ChildStdout> {
        &mut self.0
    }
}
impl UserData for LuaChildStdout
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStdout");
        fields.add_field_method_get("closed", |_, stdout| Ok(stdout.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdout, ()| {
            let address = stdout as *const _ as usize;
            Ok(format!(
                "ChildStdout(closed={}) 0x{address:x}",
                stdout.as_ref().is_closed()
            ))
        })
    }
}
