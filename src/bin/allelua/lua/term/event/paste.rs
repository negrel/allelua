use mlua::{MetaMethod, UserData};

#[derive(Debug)]
pub struct LuaPasteEvent(String);

impl From<String> for LuaPasteEvent {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl UserData for LuaPasteEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.PasteEvent");
        fields.add_field_method_get("content", |lua, ev| lua.create_string(&ev.0));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, key, ()| {
            let address = key as *const _ as usize;
            Ok(format!("term.PasteEvent 0x{address:x}",))
        });
    }
}
