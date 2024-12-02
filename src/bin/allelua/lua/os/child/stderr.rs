use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::process::ChildStderr;

use crate::lua::{
    io::{add_io_close_methods, add_io_read_methods, Closable},
    os::{add_os_try_into_stdio_methods, TryIntoStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStderr(Closable<ChildStderr>);

impl LuaChildStderr {
    pub fn new(stderr: ChildStderr) -> Self {
        Self(Closable::new(stderr))
    }
}

impl TryIntoStdio for LuaChildStderr {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let stderr: ChildStderr = self.0.into_inner()?;
        Ok(stderr.try_into()?)
    }
}

// LuaChildStderr<ChildStderr> implements io.Reader, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaChildStderr {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl AsRef<Closable<ChildStderr>> for LuaChildStderr {
    fn as_ref(&self) -> &Closable<ChildStderr> {
        &self.0
    }
}

impl AsMut<Closable<ChildStderr>> for LuaChildStderr {
    fn as_mut(&mut self) -> &mut Closable<ChildStderr> {
        &mut self.0
    }
}

impl UserData for LuaChildStderr
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "ChildStderr");
        fields.add_field_method_get("closed", |_, stderr| Ok(stderr.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stderr, ()| {
            let address = stderr as *const _ as usize;
            Ok(format!(
                "ChildStderr(closed={}) 0x{address:x}",
                stderr.as_ref().is_closed()
            ))
        })
    }
}
