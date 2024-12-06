use core::fmt;

use mlua::{IntoLua, MetaMethod, UserData};

use super::LuaKeyModifiers;

#[derive(Debug)]
pub struct LuaMouseEvent(crossterm::event::MouseEvent);

impl From<crossterm::event::MouseEvent> for LuaMouseEvent {
    fn from(value: crossterm::event::MouseEvent) -> Self {
        Self(value)
    }
}

impl UserData for LuaMouseEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.MouseEvent");
        fields.add_field_method_get("kind", |_, ev| Ok(LuaMouseEventKind::from(ev.0.kind)));
        fields.add_field_method_get("button", |lua, ev| {
            ev.button()
                .map(|btn| btn.into_lua(lua))
                .unwrap_or(Ok(mlua::Value::Nil))
        });
        fields.add_field_method_get("column", |_lua, ev| Ok(ev.0.column));
        fields.add_field_method_get("row", |_lua, ev| Ok(ev.0.row));
        fields.add_field_method_get("modifiers", |_lua, ev| {
            Ok(LuaKeyModifiers::from(ev.0.modifiers))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, mouse, ()| {
            let address = mouse as *const _ as usize;
            Ok(format!(
                "term.MouseEvent(kind={} button={}) 0x{address:x}",
                LuaMouseEventKind::from(mouse.0.kind),
                mouse
                    .button()
                    .map(|btn| btn.to_string())
                    .unwrap_or("nil".to_string())
            ))
        });
    }
}

impl LuaMouseEvent {
    fn button(&self) -> Option<LuaMouseButton> {
        match self.0.kind {
            crossterm::event::MouseEventKind::Down(btn)
            | crossterm::event::MouseEventKind::Up(btn)
            | crossterm::event::MouseEventKind::Drag(btn) => Some(LuaMouseButton::from(btn)),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct LuaMouseEventKind(crossterm::event::MouseEventKind);

impl From<crossterm::event::MouseEventKind> for LuaMouseEventKind {
    fn from(value: crossterm::event::MouseEventKind) -> Self {
        Self(value)
    }
}

impl fmt::Display for LuaMouseEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self.0 {
            crossterm::event::MouseEventKind::Down(_) => "down",
            crossterm::event::MouseEventKind::Up(_) => "up",
            crossterm::event::MouseEventKind::Drag(_) => "drag",
            crossterm::event::MouseEventKind::Moved => "moved",
            crossterm::event::MouseEventKind::ScrollDown => "scroll_down",
            crossterm::event::MouseEventKind::ScrollUp => "scroll_up",
            crossterm::event::MouseEventKind::ScrollLeft => "scroll_left",
            crossterm::event::MouseEventKind::ScrollRight => "scroll_right",
        };
        f.write_str(str)
    }
}

impl IntoLua for LuaMouseEventKind {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_string(self.to_string()).map(mlua::Value::String)
    }
}

#[derive(Debug)]
pub struct LuaMouseButton(crossterm::event::MouseButton);

impl From<crossterm::event::MouseButton> for LuaMouseButton {
    fn from(value: crossterm::event::MouseButton) -> Self {
        Self(value)
    }
}

impl fmt::Display for LuaMouseButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self.0 {
            crossterm::event::MouseButton::Left => "left",
            crossterm::event::MouseButton::Right => "right",
            crossterm::event::MouseButton::Middle => "middle",
        };
        f.write_str(str)
    }
}

impl IntoLua for LuaMouseButton {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_string(self.to_string()).map(mlua::Value::String)
    }
}
