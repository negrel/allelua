use core::fmt;

use mlua::{IntoLua, MetaMethod, UserData};

#[derive(Debug)]
pub struct LuaKeyEvent(crossterm::event::KeyEvent);

impl From<crossterm::event::KeyEvent> for LuaKeyEvent {
    fn from(value: crossterm::event::KeyEvent) -> Self {
        Self(value)
    }
}

impl UserData for LuaKeyEvent {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.KeyEvent");
        fields.add_field_method_get("code", |_, ev| Ok(LuaKeyCode::from(ev.0.code)));
        fields.add_field_method_get("modifiers", |_, ev| {
            Ok(LuaKeyModifiers::from(ev.0.modifiers))
        });
        fields.add_field_method_get("kind", |_, ev| Ok(LuaKeyEventKind::from(ev.0.kind)));
        fields.add_field_method_get("state", |_, ev| Ok(LuaKeyEventState::from(ev.0.state)));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, ev, ()| {
            let address = ev as *const _ as usize;
            Ok(format!(
                "term.KeyEvent(code={} modifiers={} kind={} state={}) 0x{address:x}",
                LuaKeyCode::from(ev.0.code),
                LuaKeyModifiers::from(ev.0.modifiers),
                LuaKeyEventKind::from(ev.0.kind),
                LuaKeyEventState::from(ev.0.state),
            ))
        });
    }
}

#[derive(Debug)]
pub struct LuaKeyCode(crossterm::event::KeyCode);

impl From<crossterm::event::KeyCode> for LuaKeyCode {
    fn from(value: crossterm::event::KeyCode) -> Self {
        Self(value)
    }
}

impl UserData for LuaKeyCode {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.KeyCode");
        fields.add_field_method_get("standard", |_, ev| Ok(ev.to_string()));
        fields.add_field_method_get("native", |_, ev| Ok(ev.0.to_string()))
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, code, ()| {
            let address = code as *const _ as usize;
            Ok(format!(
                "term.KeyCode(standard={} native={}) 0x{address:x}",
                code, code.0
            ))
        });
    }
}

impl fmt::Display for LuaKeyCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self.0 {
            crossterm::event::KeyCode::Backspace => "Backspace",
            crossterm::event::KeyCode::Enter => "Enter",
            crossterm::event::KeyCode::Left => "Left",
            crossterm::event::KeyCode::Right => "Right",
            crossterm::event::KeyCode::Up => "Up",
            crossterm::event::KeyCode::Down => "Down",
            crossterm::event::KeyCode::Home => "Home",
            crossterm::event::KeyCode::End => "End",
            crossterm::event::KeyCode::PageUp => "PageUp",
            crossterm::event::KeyCode::PageDown => "PageDown",
            crossterm::event::KeyCode::Tab => "Tab",
            crossterm::event::KeyCode::BackTab => "BackTab",
            crossterm::event::KeyCode::Delete => "Delete",
            crossterm::event::KeyCode::Insert => "Insert",
            crossterm::event::KeyCode::F(n) => return write!(f, "F{n}"),
            crossterm::event::KeyCode::Char(c) => return write!(f, "{c}"),
            crossterm::event::KeyCode::Null => "Null",
            crossterm::event::KeyCode::Esc => "Esc",
            crossterm::event::KeyCode::CapsLock => "CapsLock",
            crossterm::event::KeyCode::ScrollLock => "ScrollLock",
            crossterm::event::KeyCode::NumLock => "NumLock",
            crossterm::event::KeyCode::PrintScreen => "PrintScreen",
            crossterm::event::KeyCode::Pause => "Pause",
            crossterm::event::KeyCode::Menu => "Menu",
            crossterm::event::KeyCode::KeypadBegin => "KeypadBegin",
            crossterm::event::KeyCode::Media(m) => return LuaMediaKeyCode::from(m).fmt(f),
            crossterm::event::KeyCode::Modifier(_) => "Modifier",
        };

        f.write_str(str)
    }
}

#[derive(Debug)]
pub struct LuaMediaKeyCode(crossterm::event::MediaKeyCode);

impl From<crossterm::event::MediaKeyCode> for LuaMediaKeyCode {
    fn from(value: crossterm::event::MediaKeyCode) -> Self {
        Self(value)
    }
}

impl fmt::Display for LuaMediaKeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            crossterm::event::MediaKeyCode::Play => f.write_str("play"),
            crossterm::event::MediaKeyCode::Pause => f.write_str("pause"),
            crossterm::event::MediaKeyCode::PlayPause => f.write_str("play_pause"),
            crossterm::event::MediaKeyCode::Reverse => f.write_str("reverse"),
            crossterm::event::MediaKeyCode::Stop => f.write_str("stop"),
            crossterm::event::MediaKeyCode::FastForward => f.write_str("fast_forward"),
            crossterm::event::MediaKeyCode::Rewind => f.write_str("rewind"),
            crossterm::event::MediaKeyCode::TrackNext => f.write_str("next_track"),
            crossterm::event::MediaKeyCode::TrackPrevious => f.write_str("previous_track"),
            crossterm::event::MediaKeyCode::Record => f.write_str("record"),
            crossterm::event::MediaKeyCode::LowerVolume => f.write_str("lower_volume"),
            crossterm::event::MediaKeyCode::RaiseVolume => f.write_str("raise_volume"),
            crossterm::event::MediaKeyCode::MuteVolume => f.write_str("mute_volume"),
        }
    }
}

#[derive(Debug)]
pub struct LuaModifiersKeyCode(crossterm::event::ModifierKeyCode);

impl From<crossterm::event::ModifierKeyCode> for LuaModifiersKeyCode {
    fn from(value: crossterm::event::ModifierKeyCode) -> Self {
        Self(value)
    }
}

impl fmt::Display for LuaModifiersKeyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            crossterm::event::ModifierKeyCode::LeftShift => f.write_str("left_shift"),
            crossterm::event::ModifierKeyCode::LeftHyper => f.write_str("left_hyper"),
            crossterm::event::ModifierKeyCode::LeftMeta => f.write_str("left_meta"),
            crossterm::event::ModifierKeyCode::RightShift => f.write_str("right_shift"),
            crossterm::event::ModifierKeyCode::RightHyper => f.write_str("right_hyper"),
            crossterm::event::ModifierKeyCode::RightMeta => f.write_str("right_meta"),
            crossterm::event::ModifierKeyCode::IsoLevel3Shift => f.write_str("iso_level_3_shift"),
            crossterm::event::ModifierKeyCode::IsoLevel5Shift => f.write_str("iso_level_5_shift"),
            crossterm::event::ModifierKeyCode::LeftControl => f.write_str("left_ctrl"),
            crossterm::event::ModifierKeyCode::LeftAlt => f.write_str("left_alt"),
            crossterm::event::ModifierKeyCode::LeftSuper => f.write_str("left_suprt"),
            crossterm::event::ModifierKeyCode::RightControl => f.write_str("right_ctrl"),
            crossterm::event::ModifierKeyCode::RightAlt => f.write_str("right_alt"),
            crossterm::event::ModifierKeyCode::RightSuper => f.write_str("right_super"),
        }
    }
}

#[derive(Debug)]
pub struct LuaKeyModifiers(crossterm::event::KeyModifiers);

impl From<crossterm::event::KeyModifiers> for LuaKeyModifiers {
    fn from(value: crossterm::event::KeyModifiers) -> Self {
        Self(value)
    }
}

impl UserData for LuaKeyModifiers {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "event.KeyModifiers");
        fields.add_field_method_get("standard", |_, ev| Ok(ev.to_string()));
        fields.add_field_method_get("native", |_, ev| Ok(ev.0.to_string()));
        fields.add_field_method_get("shift", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::SHIFT))
        });
        fields.add_field_method_get("ctrl", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::CONTROL))
        });
        fields.add_field_method_get("alt", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::ALT))
        });
        fields.add_field_method_get("super", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::SUPER))
        });
        fields.add_field_method_get("hyper", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::HYPER))
        });
        fields.add_field_method_get("meta", |_, ev| {
            Ok(ev.0.contains(crossterm::event::KeyModifiers::META))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, modifiers, ()| {
            let address = modifiers as *const _ as usize;
            Ok(format!(
                "term.KeyModifiers(standard={} native={} shift={} control={} alt={} super={} hyper={} meta={}) 0x{address:x}",
                modifiers, modifiers.0,
                modifiers.0.contains(crossterm::event::KeyModifiers::SHIFT),
                modifiers.0.contains(crossterm::event::KeyModifiers::CONTROL),
                modifiers.0.contains(crossterm::event::KeyModifiers::ALT),
                modifiers.0.contains(crossterm::event::KeyModifiers::SUPER),
                modifiers.0.contains(crossterm::event::KeyModifiers::HYPER),
                modifiers.0.contains(crossterm::event::KeyModifiers::META),
            ))
        });
    }
}

impl fmt::Display for LuaKeyModifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut first = true;
        for modifier in self.0.iter() {
            if !first {
                f.write_str("+")?;
                first = false;
            }
            match modifier {
                crossterm::event::KeyModifiers::SHIFT => f.write_str("shift")?,
                crossterm::event::KeyModifiers::CONTROL => f.write_str("ctrl")?,
                crossterm::event::KeyModifiers::ALT => f.write_str("alt")?,
                crossterm::event::KeyModifiers::SUPER => f.write_str("super")?,
                crossterm::event::KeyModifiers::HYPER => f.write_str("hyper")?,
                crossterm::event::KeyModifiers::META => f.write_str("meta")?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LuaKeyEventKind(crossterm::event::KeyEventKind);

impl From<crossterm::event::KeyEventKind> for LuaKeyEventKind {
    fn from(value: crossterm::event::KeyEventKind) -> Self {
        Self(value)
    }
}

impl IntoLua for LuaKeyEventKind {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.create_string(self.to_string()).map(mlua::Value::String)
    }
}

impl fmt::Display for LuaKeyEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self.0 {
            crossterm::event::KeyEventKind::Press => "press",
            crossterm::event::KeyEventKind::Repeat => "repeat",
            crossterm::event::KeyEventKind::Release => "release",
        };

        f.write_str(kind)
    }
}

#[derive(Debug)]
pub struct LuaKeyEventState(crossterm::event::KeyEventState);

impl From<crossterm::event::KeyEventState> for LuaKeyEventState {
    fn from(value: crossterm::event::KeyEventState) -> Self {
        Self(value)
    }
}

impl UserData for LuaKeyEventState {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.KeyEventState");
        fields.add_field_method_get("keypad", |_, state| {
            Ok(state.0.contains(crossterm::event::KeyEventState::KEYPAD))
        });
        fields.add_field_method_get("caps_lock", |_, state| {
            Ok(state.0.contains(crossterm::event::KeyEventState::CAPS_LOCK))
        });
        fields.add_field_method_get("num_lock", |_, state| {
            Ok(state.0.contains(crossterm::event::KeyEventState::NUM_LOCK))
        });
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, state, ()| Ok(state.to_string()));
    }
}

impl fmt::Display for LuaKeyEventState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let address = self as *const _ as usize;
        write!(
            f,
            "term.KeyState(keypad={} caps_lock={} num_lock={}) 0x{address:x}",
            self.0.contains(crossterm::event::KeyEventState::KEYPAD),
            self.0.contains(crossterm::event::KeyEventState::CAPS_LOCK),
            self.0.contains(crossterm::event::KeyEventState::NUM_LOCK),
        )
    }
}
