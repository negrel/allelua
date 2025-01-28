use std::ops::Deref;

use mlua::{IntoLua, MetaMethod, UserData};
use tokio::{
    fs::{DirEntry, ReadDir},
    sync::Mutex,
};

use crate::lua::path::LuaMetadata;

#[derive(Debug)]
pub struct LuaReadDir(Mutex<ReadDir>);

impl Deref for LuaReadDir {
    type Target = Mutex<ReadDir>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<ReadDir> for LuaReadDir {
    fn from(value: ReadDir) -> Self {
        Self(Mutex::new(value))
    }
}

impl UserData for LuaReadDir {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.ReadDir")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_meta_method(MetaMethod::Call, |lua, d, ()| async move {
            let mut rd = d.lock().await;

            if let Some(entry) = rd.next_entry().await? {
                Ok(LuaDirEntry::from(entry).into_lua(&lua)?)
            } else {
                Ok(mlua::Value::Nil)
            }
        });

        methods.add_meta_method(MetaMethod::ToString, |_, d, ()| {
            let address = d as *const _ as usize;
            Ok(format!("os.ReadDir 0x{address:x}"))
        });
    }
}

#[derive(Debug)]
pub struct LuaDirEntry(DirEntry);

impl From<DirEntry> for LuaDirEntry {
    fn from(value: DirEntry) -> Self {
        Self(value)
    }
}

impl Deref for LuaDirEntry {
    type Target = DirEntry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl UserData for LuaDirEntry {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "os.DirEntry");

        fields.add_field_method_get("file_name", |_, e| Ok(e.file_name()));
        fields.add_field_method_get("ino", |_, e| Ok(e.ino()));
        fields.add_field_method_get("path", |_, e| Ok(e.path()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("metadata", |_, e, ()| async move {
            Ok(LuaMetadata(e.metadata().await?))
        });

        methods.add_meta_method(MetaMethod::ToString, |_, e, ()| {
            let address = e as *const _ as usize;
            Ok(format!(
                "os.DirEntry(file_name={:?}, path={:?}) 0x{address:x}",
                e.file_name().to_str().unwrap_or("???"),
                e.path().to_str().unwrap_or("???"),
            ))
        });
    }
}
