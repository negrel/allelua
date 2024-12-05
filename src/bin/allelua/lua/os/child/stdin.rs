use std::process::Stdio;

use mlua::{MetaMethod, UserData};
use tokio::{fs::File, process::ChildStdin};

use crate::lua::{
    io::{self, add_io_write_close_methods, Closable},
    os::{add_os_try_as_stdio_methods, TryAsStdio},
    LuaInterface,
};

#[derive(Debug)]
pub struct LuaChildStdin(Closable<File>);

impl LuaChildStdin {
    pub fn new(stdin: ChildStdin) -> mlua::Result<Self> {
        let fd = stdin.into_owned_fd().map_err(io::LuaError)?;
        Ok(Self(Closable::new(File::from(std::fs::File::from(fd)))))
    }
}

impl TryAsStdio for LuaChildStdin {
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

// LuaChildStdin<ChildStdin> implements io.WriteCloser and os.TryIntoStdio.
impl LuaInterface for LuaChildStdin {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
        add_os_try_as_stdio_methods(methods);
    }
}

impl AsRef<Closable<File>> for LuaChildStdin {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaChildStdin {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}

impl UserData for LuaChildStdin
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.ChildStdin");
        fields.add_field_method_get("closed", |_, stdin| Ok(stdin.as_ref().is_closed()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        Self::add_interface_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_lua, stdin, ()| {
            let address = stdin as *const _ as usize;
            Ok(format!(
                "os.ChildStdin(closed={}) 0x{address:x}",
                stdin.as_ref().is_closed()
            ))
        })
    }
}
