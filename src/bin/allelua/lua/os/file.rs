use std::os::fd::AsRawFd;

use mlua::{MetaMethod, UserData};
use tokio::fs::File;
use tokio::io::BufStream;

use crate::lua::io::{
    add_io_buf_reader_methods, add_io_closer_methods, add_io_seeker_methods, add_io_writer_methods,
};

#[derive(Debug)]
pub(super) struct LuaFile(Option<BufStream<File>>);

impl LuaFile {
    pub fn new(f: File) -> Self {
        Self(Some(BufStream::new(f)))
    }
}

impl AsMut<Option<BufStream<File>>> for LuaFile {
    fn as_mut(&mut self) -> &mut Option<BufStream<File>> {
        &mut self.0
    }
}

impl UserData for LuaFile {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "File")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        add_io_buf_reader_methods(methods);
        add_io_writer_methods(methods);
        add_io_closer_methods(methods);
        add_io_seeker_methods(methods);

        methods.add_meta_method(MetaMethod::ToString, |_, f, ()| {
            let address = f as *const _ as usize;
            match &f.0 {
                Some(f) => {
                    let fd = f.get_ref().as_raw_fd();
                    Ok(format!("File(fd={fd}) 0x{address:x}"))
                }
                None => Ok(format!("File(fd=closed, 0x{address:x})")),
            }
        });
    }
}
