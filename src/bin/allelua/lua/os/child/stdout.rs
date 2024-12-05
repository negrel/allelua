use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{fs::File, process::ChildStdout};

use crate::lua::{
    io::{self, add_io_close_methods, add_io_read_methods, Closable},
    os::{add_os_try_as_stdio_methods, TryAsStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdout(Closable<File>);

impl LuaChildStdout {
    pub fn new(stdout: ChildStdout) -> mlua::Result<Self> {
        let fd = stdout.into_owned_fd().map_err(io::LuaError)?;
        Ok(Self(Closable::new(File::from(std::fs::File::from(fd)))))
    }
}

impl TryAsStdio for LuaChildStdout {
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

// LuaChildStdout<ChildStdout> implements io.Reader and io.Closer.
impl LuaInterface for LuaChildStdout {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_as_stdio_methods(methods);
    }
}

impl AsRef<Closable<File>> for LuaChildStdout {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaChildStdout {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}
impl UserData for LuaChildStdout
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.ChildStdout");
        fields.add_field_method_get("closed", |_, stdout| Ok(stdout.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdout, ()| {
            let address = stdout as *const _ as usize;
            Ok(format!(
                "os.ChildStdout(closed={}) 0x{address:x}",
                stdout.as_ref().is_closed()
            ))
        })
    }
}
