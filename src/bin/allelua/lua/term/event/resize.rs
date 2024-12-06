use mlua::{MetaMethod, UserData};

#[derive(Debug)]
pub struct LuaEventResize(u16, u16);

impl From<(u16, u16)> for LuaEventResize {
    fn from(value: (u16, u16)) -> Self {
        Self(value.0, value.1)
    }
}

impl UserData for LuaEventResize {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.ResizeEvent");
        fields.add_field_method_get("columns", |_, k| Ok(k.0));
        fields.add_field_method_get("rows", |_, k| Ok(k.1));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, key, ()| {
            let address = key as *const _ as usize;
            Ok(format!(
                "term.ResizeEvent(columns={} rows={}) 0x{address:x}",
                key.0, key.1
            ))
        });
    }
}
