use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{fs::File, process::ChildStderr};

use crate::lua::{
    io::{self, add_io_close_methods, add_io_read_methods, Closable},
    os::{add_os_try_as_stdio_methods, TryAsStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStderr(Closable<File>);

impl LuaChildStderr {
    pub fn new(stderr: ChildStderr) -> mlua::Result<Self> {
        let fd = stderr.into_owned_fd().map_err(io::LuaError)?;
        Ok(Self(Closable::new(File::from(std::fs::File::from(fd)))))
    }
}

impl TryAsStdio for LuaChildStderr {
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

// LuaChildStderr<ChildStderr> implements io.Reader, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaChildStderr {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_close_methods(methods);
        add_os_try_as_stdio_methods(methods);
    }
}

impl AsRef<Closable<File>> for LuaChildStderr {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaChildStderr {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}

impl UserData for LuaChildStderr
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.ChildStderr");
        fields.add_field_method_get("closed", |_, stderr| Ok(stderr.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        LuaInterface::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stderr, ()| {
            let address = stderr as *const _ as usize;
            Ok(format!(
                "os.ChildStderr(closed={}) 0x{address:x}",
                stderr.as_ref().is_closed()
            ))
        })
    }
}
