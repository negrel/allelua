use mlua::{MetaMethod, UserData};

mod key;
mod mouse;
mod paste;
mod resize;
mod stream;

pub use key::*;
pub use mouse::*;
pub use paste::*;
pub use resize::*;
pub use stream::*;

#[derive(Debug)]
pub struct LuaFocusGainedEvent;

impl UserData for LuaFocusGainedEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.FocusGainedEvent");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, stream, ()| {
            let address = stream as *const _ as usize;
            Ok(format!("term.FocusGainedEvent 0x{address:x}",))
        });
    }
}

#[derive(Debug)]
pub struct LuaFocusLostEvent;

impl UserData for LuaFocusLostEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.FocusLostEvent");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, stream, ()| {
            let address = stream as *const _ as usize;
            Ok(format!("term.FocusLostEvent 0x{address:x}",))
        });
    }
}
