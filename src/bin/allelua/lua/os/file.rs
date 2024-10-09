use mlua::{MetaMethod, UserData};
use tokio::{fs::File, io::BufStream};

use crate::lua::io::{
    self, add_io_read_methods, add_io_seek_methods, add_io_write_close_methods, Closable,
};

#[derive(Debug)]
pub(super) struct LuaFile(io::Closable<BufStream<File>>);

impl LuaFile {
    pub fn new(f: File, buf_size: Option<usize>) -> Self {
        let buf_stream = match buf_size {
            Some(n) => BufStream::with_capacity(n, n, f),
            None => BufStream::new(f),
        };

        Self(io::Closable::new(buf_stream))
    }
}

impl AsRef<Closable<BufStream<File>>> for LuaFile {
    fn as_ref(&self) -> &Closable<BufStream<File>> {
        &self.0
    }
}

impl AsMut<Closable<BufStream<File>>> for LuaFile {
    fn as_mut(&mut self) -> &mut Closable<BufStream<File>> {
        &mut self.0
    }
}

impl UserData for LuaFile {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "File")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_write_close_methods(methods);
        add_io_read_methods(methods);
        add_io_seek_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            if f.0.is_closed() {
                Ok(format!("File(closed) 0x{address:x}"))
            } else {
                Ok(format!("File 0x{address:x}"))
            }
        });

        methods.add_async_method("sync", |_lua, file, ()| async move {
            let file = file.0.get().await?;

            file.get_ref().sync_all().await?;

            Ok(())
        })
    }
}
