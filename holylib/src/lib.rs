pub mod types;

/// IncludeChunk is a helper type used by include_lua macro.
pub struct IncludeChunk {
    pub name: &'static str,
    pub source: &'static [u8],
}

impl<'a> mlua::AsChunk<'a> for IncludeChunk {
    fn source(self) -> std::io::Result<std::borrow::Cow<'a, [u8]>> {
        Ok(std::borrow::Cow::Borrowed(self.source))
    }

    fn name(&self) -> Option<String> {
        Some(self.name.to_string())
    }

    fn environment(
        &self,
        _lua: &mlua::Lua,
    ) -> mlua::prelude::LuaResult<Option<mlua::prelude::LuaTable>> {
        Ok(None)
    }

    fn mode(&self) -> Option<mlua::ChunkMode> {
        Some(mlua::ChunkMode::Text)
    }
}

#[macro_export]
macro_rules! include_lua {
    ($path:tt) => {{
        $crate::IncludeChunk {
            name: module_path!(),
            source: include_bytes!($path),
        }
    }};
}
