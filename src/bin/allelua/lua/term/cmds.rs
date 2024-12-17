use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crossterm::{
    cursor::{
        DisableBlinking, EnableBlinking, Hide, MoveDown, MoveLeft, MoveRight, MoveTo, MoveToColumn,
        MoveToNextLine, MoveToPreviousLine, MoveToRow, RestorePosition, SavePosition,
        SetCursorStyle, Show,
    },
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    style::{Attribute, Print, SetAttribute, SetBackgroundColor, SetForegroundColor},
    terminal::{
        BeginSynchronizedUpdate, Clear, ClearType, DisableLineWrap, EnableLineWrap,
        EndSynchronizedUpdate, EnterAlternateScreen, LeaveAlternateScreen, ScrollDown, ScrollUp,
        SetSize, SetTitle,
    },
    QueueableCommand,
};
use mlua::{FromLua, IntoLua, MetaMethod, UserData};

use crate::lua::{io, os::LuaFile};

use super::colors::LuaColor;

/// LuaQueue define a [crossterm::Command] queue.
#[derive(Debug, Clone)]
pub struct LuaQueue(Arc<Mutex<std::fs::File>>);

impl LuaQueue {
    pub async fn from_lua_file(f: &LuaFile) -> mlua::Result<Self> {
        let f = f.as_ref().get().await?;
        let f = f
            .try_clone()
            .await
            .map_err(io::LuaError::from)?
            .into_std()
            .await;

        Ok(Self(Arc::new(Mutex::new(f))))
    }

    async fn flush(&self) -> mlua::Result<()> {
        let f = self.0.clone();
        tokio::task::spawn_blocking(move || {
            f.lock().unwrap().flush().map_err(io::LuaError::from)?;
            Ok::<_, mlua::Error>(f)
        })
        .await
        .map_err(mlua::Error::external)??;
        Ok(())
    }

    async fn queue<C: crossterm::Command>(
        &self,
        cmd: impl FnOnce() -> C + Send + 'static,
    ) -> mlua::Result<()> {
        let f = self.0.clone();
        tokio::task::spawn_blocking(move || {
            Self::queue_sync(&mut f.lock().unwrap(), cmd())?;
            Ok::<_, mlua::Error>(f)
        })
        .await
        .map_err(mlua::Error::external)??;
        Ok(())
    }

    fn queue_sync(f: &mut std::fs::File, command: impl crossterm::Command) -> mlua::Result<()> {
        f.queue(command).map(|_| ()).map_err(io::LuaError::from)?;
        Ok(())
    }
}

impl UserData for LuaQueue {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "term.Queue");
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("begin_sync", |lua, q, ()| async move {
            q.queue(|| BeginSynchronizedUpdate).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("end_sync", |lua, q, ()| async move {
            q.queue(|| EndSynchronizedUpdate).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("line_wrap", |lua, q, enable: Option<bool>| async move {
            if matches!(enable, Some(false)) {
                q.queue(|| DisableLineWrap).await?;
            } else {
                q.queue(|| EnableLineWrap).await?;
            }
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("alt_screen", |lua, q, ()| async move {
            q.queue(|| EnterAlternateScreen).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("main_screen", |lua, q, ()| async move {
            q.queue(|| LeaveAlternateScreen).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("scroll_down", |lua, q, n: Option<u16>| async move {
            q.queue(move || ScrollDown(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("scroll_up", |lua, q, n: Option<u16>| async move {
            q.queue(move || ScrollUp(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("size", |lua, q, (cols, rows): (u16, u16)| async move {
            q.queue(move || SetSize(cols, rows)).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("title", |lua, q, title: mlua::String| async move {
            let title = title.to_string_lossy();
            q.queue(move || SetTitle(title)).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("purge", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::Purge)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("clear", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::All)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("clear_line", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::CurrentLine)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("clear_until_new_line", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::UntilNewLine)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("clear_cursor_up", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::FromCursorUp)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("clear_cursor_down", |lua, q, ()| async move {
            q.queue(|| Clear(ClearType::FromCursorDown)).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("cursor_to", |lua, q, (col, row): (u16, u16)| async move {
            q.queue(move || MoveTo(col.saturating_sub(1), row.saturating_sub(1)))
                .await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_down", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveDown(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_left", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveLeft(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_right", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveRight(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_up", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveRight(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_next_line", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveToNextLine(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_prev_line", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveToPreviousLine(n.unwrap_or(1))).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_row", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveToRow(n.unwrap_or(1).saturating_sub(1)))
                .await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_col", |lua, q, n: Option<u16>| async move {
            q.queue(move || MoveToColumn(n.unwrap_or(1).saturating_sub(1)))
                .await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("cursor_save_position", |lua, q, ()| async move {
            q.queue(|| SavePosition).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_restore_position", |lua, q, ()| async move {
            q.queue(|| RestorePosition).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("cursor_default", |lua, q, ()| async move {
            q.queue(|| SetCursorStyle::DefaultUserShape).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("cursor_bar", |lua, q, blinking: Option<bool>| async move {
            if matches!(blinking, Some(true)) {
                q.queue(|| SetCursorStyle::BlinkingBar).await?;
            } else {
                q.queue(|| SetCursorStyle::SteadyBar).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method(
            "cursor_block",
            |lua, q, blinking: Option<bool>| async move {
                if matches!(blinking, Some(true)) {
                    q.queue(|| SetCursorStyle::BlinkingBlock).await?;
                } else {
                    q.queue(|| SetCursorStyle::SteadyBlock).await?;
                }
                q.clone().into_lua(&lua)
            },
        );
        methods.add_async_method(
            "cursor_underscore",
            |lua, q, blinking: Option<bool>| async move {
                if matches!(blinking, Some(true)) {
                    q.queue(|| SetCursorStyle::BlinkingUnderScore).await?;
                } else {
                    q.queue(|| SetCursorStyle::SteadyUnderScore).await?;
                }
                q.clone().into_lua(&lua)
            },
        );

        methods.add_async_method(
            "cursor_blinking",
            |lua, q, enable: Option<bool>| async move {
                if matches!(enable, Some(false)) {
                    q.queue(|| DisableBlinking).await?;
                } else {
                    q.queue(|| EnableBlinking).await?;
                }
                q.clone().into_lua(&lua)
            },
        );
        methods.add_async_method(
            "cursor_visible",
            |lua, q, visible: Option<bool>| async move {
                if matches!(visible, Some(false)) {
                    q.queue(|| Hide).await?;
                } else {
                    q.queue(|| Show).await?;
                }
                q.clone().into_lua(&lua)
            },
        );

        methods.add_async_method("foreground", |lua, q, color: LuaColor| async move {
            q.queue(move || SetForegroundColor(color.into())).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("background", |lua, q, color: LuaColor| async move {
            q.queue(move || SetBackgroundColor(color.into())).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("reset", |lua, q, ()| async move {
            q.queue(|| SetAttribute(Attribute::Reset)).await?;
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("bold", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoBold)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Bold)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("dim", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NormalIntensity)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Dim)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("italic", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoItalic)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Italic)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("underline", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoUnderline)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Underlined)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("slow_blink", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoBlink)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::SlowBlink)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("rapid_blink", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoBlink)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::RapidBlink)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("reverse", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoReverse)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Reverse)).await?;
            }
            q.clone().into_lua(&lua)
        });
        methods.add_async_method("hidden", |lua, q, enabled: Option<bool>| async move {
            if matches!(enabled, Some(false)) {
                q.queue(|| SetAttribute(Attribute::NoHidden)).await?;
            } else {
                q.queue(|| SetAttribute(Attribute::Hidden)).await?;
            }
            q.clone().into_lua(&lua)
        });

        methods.add_async_method(
            "push_keyboard_enhancement",
            |lua, q, flags: LuaKeyboardEnhancementFlags| async move {
                q.queue(|| PushKeyboardEnhancementFlags(flags.into()))
                    .await?;
                q.clone().into_lua(&lua)
            },
        );

        methods.add_async_method("pop_keyboard_enhancement", |lua, q, ()| async move {
            q.queue(|| PopKeyboardEnhancementFlags).await?;
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("write", |lua, q, args: mlua::MultiValue| async move {
            let tostring = lua.globals().get::<mlua::Function>("tostring")?;
            for arg in args {
                let arg = tostring.call::<mlua::String>(arg)?.to_string_lossy();
                q.queue(move || Print(arg)).await?;
            }
            q.clone().into_lua(&lua)
        });

        methods.add_async_method("flush", |lua, q, ()| async move {
            q.flush().await?;
            q.clone().into_lua(&lua)
        });

        methods.add_meta_method(MetaMethod::ToString, |_, q, ()| {
            let address = q as *const _ as usize;
            Ok(format!("term.Queue() 0x{address:x}"))
        });
    }
}

#[derive(Debug, Clone)]
struct LuaKeyboardEnhancementFlags(KeyboardEnhancementFlags);

impl FromLua for LuaKeyboardEnhancementFlags {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let table = mlua::Table::from_lua(value, lua)?;

        let mut flags = KeyboardEnhancementFlags::empty();

        if matches!(table.get("report_event_kind")?, Some(true)) {
            flags.insert(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
        }
        if matches!(table.get("disambiguate_escape_codes")?, Some(true)) {
            flags.insert(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        }
        if matches!(table.get("report_alternate_keys")?, Some(true)) {
            flags.insert(KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS)
        }
        if matches!(table.get("report_all_keys_as_escape_codes")?, Some(true)) {
            flags.insert(KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES)
        }

        Ok(Self(flags))
    }
}

impl From<LuaKeyboardEnhancementFlags> for KeyboardEnhancementFlags {
    fn from(val: LuaKeyboardEnhancementFlags) -> Self {
        val.0
    }
}
