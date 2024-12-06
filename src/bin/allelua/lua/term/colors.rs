use crossterm::style::Color;
use mlua::{Either, FromLua};

/// LuaColor define a terminal color Lua object.
#[derive(Debug)]
pub struct LuaColor(crossterm::style::Color);

impl From<crossterm::style::Color> for LuaColor {
    fn from(value: crossterm::style::Color) -> Self {
        Self(value)
    }
}

impl From<LuaColor> for crossterm::style::Color {
    fn from(value: LuaColor) -> Self {
        value.0
    }
}

impl FromLua for LuaColor {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let either = Either::<u32, mlua::String>::from_lua(value, lua)?;
        match either {
            Either::Left(rgb) => Ok(Self::from(crossterm::style::Color::Rgb {
                r: (rgb >> 16 & 0xFF) as u8,
                g: (rgb >> 8 & 0xFF) as u8,
                b: (rgb & 0xFF) as u8,
            })),
            Either::Right(str) => {
                let color = match str.as_bytes().as_ref() {
                    b"reset" => Color::Reset,
                    b"black" => Color::Black,
                    b"dark_grey" => Color::DarkGrey,
                    b"red" => Color::Red,
                    b"dark_red" => Color::DarkRed,
                    b"green" => Color::Green,
                    b"dark_green" => Color::DarkGreen,
                    b"yellow" => Color::Yellow,
                    b"dark_yellow" => Color::DarkYellow,
                    b"blue" => Color::Blue,
                    b"dark_blue" => Color::DarkBlue,
                    b"magenta" => Color::Magenta,
                    b"dark_magenta" => Color::DarkMagenta,
                    b"cyan" => Color::Cyan,
                    b"dark_cyan" => Color::DarkCyan,
                    b"white" => Color::White,
                    b"grey" => Color::Grey,
                    _ => {
                        return Err(mlua::Error::FromLuaConversionError {
                            from: "string",
                            to: "color".to_owned(),
                            message: Some(format!(
                                "{:?} is not a valid hex color",
                                str.to_string_lossy()
                            )),
                        })
                    }
                };

                Ok(Self::from(color))
            }
        }
    }
}
