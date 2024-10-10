use std::process::Stdio;

use mlua::{IntoLua, Lua, MetaMethod, UserData};
use tokio::{
    fs::{File, OpenOptions},
    io::BufStream,
};

use crate::{
    lua::{
        io::{
            self, add_io_buf_read_methods, add_io_close_methods, add_io_read_methods,
            add_io_seek_methods, add_io_write_methods, Closable, MaybeBuffered,
        },
        LuaInterface,
    },
    lua_string_as_path,
};

use super::{add_io_try_into_stdio_methods, TryIntoStdio};

#[derive(Debug)]
pub(super) struct LuaFile<T: MaybeBuffered<File>>(io::Closable<T>);

impl LuaFile<File> {
    pub fn new(f: File) -> Self {
        Self(io::Closable::new(f))
    }
}

impl<T: MaybeBuffered<File>> TryIntoStdio for LuaFile<T> {
    async fn try_into_stdio(self) -> mlua::Result<Stdio> {
        let file: File = self.0.into_inner()?.into_inner();
        let std_file = file.into_std().await;
        Ok(std_file.into())
    }
}

impl LuaFile<BufStream<File>> {
    pub fn new_buffered(f: File, buf_size: Option<usize>) -> Self {
        let buf_stream = match buf_size {
            Some(n) => BufStream::with_capacity(n, n, f),
            None => BufStream::new(f),
        };

        Self(io::Closable::new(buf_stream))
    }
}

impl<T: MaybeBuffered<File>> AsRef<Closable<T>> for LuaFile<T> {
    fn as_ref(&self) -> &Closable<T> {
        &self.0
    }
}

impl<T: MaybeBuffered<File>> AsMut<Closable<T>> for LuaFile<T> {
    fn as_mut(&mut self) -> &mut Closable<T> {
        &mut self.0
    }
}

// LuaFile<File> implements io.Reader, io.Seeker, io.Writer, io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaFile<File> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_seek_methods(methods);
        add_io_write_methods(methods);
        add_io_close_methods(methods);
        add_io_try_into_stdio_methods(methods);
    }
}

// LuaFile<File> implements io.Reader, io.BufReader, io.Seeker, io.Writer,
// io.Closer and os.TryIntoStdio.
impl LuaInterface for LuaFile<BufStream<File>> {
    fn add_interface_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_read_methods(methods);
        add_io_buf_read_methods(methods);
        add_io_seek_methods(methods);
        add_io_write_methods(methods);
        add_io_close_methods(methods);
        add_io_try_into_stdio_methods(methods);
    }
}

impl<T: MaybeBuffered<File> + 'static> UserData for LuaFile<T>
where
    Self: LuaInterface,
{
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "File")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        Self::add_interface_methods(methods);

        methods.add_async_method("sync", |_lua, file, ()| async move {
            let mut file = file.as_ref().get().await?;
            let t: &mut T = &mut file;
            let file = MaybeBuffered::<File>::get_mut(t);

            file.get_ref().sync_all().await?;

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
    let mut buffer_size = None;

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

        buffer_size = opt_table.get::<Option<usize>>("buffer_size")?;
    }

    let file = options.open(path).await?;

    match buffer_size {
        Some(0) => LuaFile::new(file).into_lua(&lua),
        None | Some(_) => LuaFile::new_buffered(file, buffer_size).into_lua(&lua),
    }
}
