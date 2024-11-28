use std::process::Stdio;

use mlua::{IntoLua, Lua, MetaMethod, UserData};
use tokio::fs::{File, OpenOptions};

use crate::{
    lua::{
        io::{
            self, add_io_read_methods, add_io_seek_methods, add_io_write_close_methods, Closable,
        },
        path::LuaMetadata,
        LuaInterface,
    },
    lua_string_as_path,
};

use super::{add_os_try_into_stdio_methods, TryIntoStdio};

#[derive(Debug)]
pub(super) struct LuaFile(io::Closable<File>);

impl LuaFile {
    pub fn new(f: File) -> Self {
        Self(io::Closable::new(f))
    }
}

impl AsRef<Closable<File>> for LuaFile {
    fn as_ref(&self) -> &Closable<File> {
        &self.0
    }
}

impl AsMut<Closable<File>> for LuaFile {
    fn as_mut(&mut self) -> &mut Closable<File> {
        &mut self.0
    }
}

impl TryIntoStdio for LuaFile {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let file: File = self.0.into_inner()?;
        let std_file = file.into_std().await;
        Ok(std_file.into())
    }
}

// LuaFile<File> implements io.Reader, io.Seeker, io.WriteCloser and
// os.TryIntoStdio.
impl LuaInterface for LuaFile {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_seek_methods(methods);
        add_io_write_close_methods(methods);
        add_os_try_into_stdio_methods(methods);
    }
}

impl UserData for LuaFile
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "File")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        Self::add_interface_methods(methods);

        methods.add_async_method("metadata", |_lua, file, ()| async move {
            let file = file.as_ref().get().await?;
            let metadata = file.metadata().await?;

            Ok(LuaMetadata(metadata))
        });

        methods.add_async_method("sync", |_lua, file, ()| async move {
            let file = file.as_ref().get().await?;
            file.sync_all().await?;

            Ok(())
        });

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            if f.as_ref().is_closed() {
                Ok(format!("File(state=close) 0x{address:x}"))
            } else {
                Ok(format!("File(state=open) 0x{address:x}"))
            }
        });
    }
}

pub async fn open_file(
    lua: Lua,
    (path, opt_table): (mlua::String, Option<mlua::Table>),
) -> mlua::Result<mlua::Value> {
    lua_string_as_path!(path = path);
    let mut options = OpenOptions::new();

    if let Some(opt_table) = opt_table {
        if let Some(true) = opt_table.get::<Option<bool>>("create")? {
            options.create(true);
        }

        if let Some(true) = opt_table.get::<Option<bool>>("create_new")? {
            options.create_new(true);
        }

        if let Some(true) = opt_table.get::<Option<bool>>("read")? {
            options.read(true);
        }

        if let Some(true) = opt_table.get::<Option<bool>>("write")? {
            options.write(true);
        }

        if let Some(true) = opt_table.get::<Option<bool>>("append")? {
            options.append(true);
        }

        if let Some(true) = opt_table.get::<Option<bool>>("truncate")? {
            options.truncate(true);
        }
    }

    let file = options.open(path).await.map_err(io::LuaError)?;

    LuaFile::new(file).into_lua(&lua)
}
