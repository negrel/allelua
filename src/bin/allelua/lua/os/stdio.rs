use mlua::{AnyUserData, FromLua, MetaMethod, UserData};

/// TryIntoStdio is a trait implemented by [UserData] that can be converted to a
/// [std::process::Stdio].
pub trait TryIntoStdio {
    async fn try_into_stdio(self) -> mlua::Result<std::process::Stdio>;
}

pub fn add_io_try_into_stdio_methods<R: TryIntoStdio + 'static, M: mlua::UserDataMethods<R>>(
    methods: &mut M,
) {
    methods.add_async_function("try_into_stdio", |_lua, into: AnyUserData| async move {
        let stdio = into.take::<R>()?.try_into_stdio().await?;
        Ok(LuaStdio(stdio))
    });
}

/// Stdio is a wrapper around [std::process::Stdio] that implements [UserData].
#[derive(Debug)]
pub struct LuaStdio(std::process::Stdio);

impl UserData for LuaStdio {
    fn add_fields<F: mlua::prelude::LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "Stdio");
    }

    fn add_methods<M: mlua::prelude::LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, stdio, ()| {
            let address = stdio as *const _ as usize;
            Ok(format!("Stdio 0x{address:x}"))
        })
    }
}

impl From<LuaStdio> for std::process::Stdio {
    fn from(value: LuaStdio) -> Self {
        value.0
    }
}

impl From<std::process::Stdio> for LuaStdio {
    fn from(value: std::process::Stdio) -> Self {
        Self(value)
    }
}

impl FromLua for LuaStdio {
    fn from_lua(
        value: mlua::prelude::LuaValue,
        lua: &mlua::prelude::Lua,
    ) -> mlua::prelude::LuaResult<Self> {
        let udata = AnyUserData::from_lua(value, lua)?;
        udata.take()
    }
}
