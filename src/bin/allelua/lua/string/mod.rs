use std::ops::Deref;

use mlua::{chunk, BorrowedBytes, FromLua, IntoLua, Lua};

use crate::include_lua;

mod big;
mod regex;

pub use big::*;
pub use regex::*;

pub fn load_string(lua: &Lua) -> mlua::Result<()> {
    let string_extra = lua.create_table()?;

    let slice = lua
        .load(chunk! {
            return require("string").sub
        })
        .eval::<mlua::Function>()?;

    string_extra.set("split", lua.create_function(lua_string_split::<MaybeBig>)?)?;
    string_extra.set(
        "splitn",
        lua.create_function(lua_string_splitn::<MaybeBig>)?,
    )?;
    {
        let slice = slice.clone();
        string_extra.set(
            "find",
            lua.create_function(
                move |lua, (str, pattern, at): (MaybeBig, LuaRegex, Option<usize>)| match str {
                    MaybeBig::Lua(str) => {
                        lua_string_find(lua, str, pattern, at, |str, start, end| {
                            slice.call((str, start, end))
                        })
                    }
                    MaybeBig::Big(big) => {
                        lua_string_find(lua, big, pattern, at, |big, start, end| {
                            Ok(big.slice(start..end))
                        })
                    }
                },
            )?,
        )?;
    }
    string_extra.set(
        "replace",
        lua.create_function(lua_string_replace::<MaybeBig>)?,
    )?;
    string_extra.set(
        "replace_all",
        lua.create_function(lua_string_replace_all::<MaybeBig>)?,
    )?;

    string_extra.set(
        "captures",
        lua.create_function(
            move |lua, (str, pattern, at): (MaybeBig, LuaRegex, Option<usize>)| {
                lua_string_captures(lua, str, pattern, at, |str, start, end| {
                    slice.call((str, start, end))
                })
            },
        )?,
    )?;

    string_extra.set(
        "eq",
        lua.create_function(|_, (lhs, rhs): (MaybeBig, MaybeBig)| {
            Ok(*lhs.as_bytes() == *rhs.as_bytes())
        })?,
    )?;

    let big_string_constructors = lua.create_table()?;
    big_string_constructors.set(
        "fromstring",
        lua.create_function(|_lua, str: MaybeBig| Ok(LuaBigString::from(str.as_bytes())))?,
    )?;

    let regex_constructors = lua.create_table()?;
    regex_constructors.set(
        "escape",
        lua.create_function(|_lua, str: mlua::String| Ok(::regex::escape(&str.to_str()?)))?,
    )?;
    regex_constructors.set(
        "new",
        lua.create_function(|_lua, str: mlua::String| {
            LuaRegex::new(&str.to_str()?).map_err(mlua::Error::external)
        })?,
    )?;

    let string_mt = lua
        .load(include_lua!("./string.lua"))
        .eval::<mlua::Function>()?
        .call::<mlua::Table>((regex_constructors, big_string_constructors, string_extra))?;

    lua.set_type_metatable::<mlua::String>(Some(string_mt));

    Ok(())
}

/// LuaString is a trait implemented by [LuaBigString] and [mlua::String] that
/// can be used for generic methods. [MaybeBig] also implements it and forward
/// calls to actual variant.
trait LuaString<'a>: IntoLua {
    type Bytes: Deref<Target = [u8]>;

    fn as_bytes(&'a self) -> Self::Bytes;

    fn create_string(&self, lua: &Lua, bytes: &[u8]) -> mlua::Result<Self>;
}

impl<'a> LuaString<'a> for mlua::String {
    type Bytes = BorrowedBytes<'a>;

    fn as_bytes(&'a self) -> Self::Bytes {
        self.as_bytes()
    }

    fn create_string(&self, lua: &Lua, bytes: &[u8]) -> mlua::Result<Self> {
        lua.create_string(bytes)
    }
}

#[derive(Debug, Clone)]
pub enum MaybeBig {
    Lua(mlua::String),
    Big(LuaBigString),
}

impl<'a> LuaString<'a> for MaybeBig {
    type Bytes = MaybeBigBytes<'a>;

    fn as_bytes(&'a self) -> Self::Bytes {
        match self {
            MaybeBig::Lua(str) => MaybeBigBytes::Lua(str.as_bytes()),
            MaybeBig::Big(str) => MaybeBigBytes::Big(str.as_bytes()),
        }
    }

    fn create_string(&self, lua: &Lua, bytes: &[u8]) -> mlua::Result<Self> {
        match self {
            MaybeBig::Lua(s) => s.create_string(lua, bytes).map(MaybeBig::Lua),
            MaybeBig::Big(b) => b.create_string(lua, bytes).map(MaybeBig::Big),
        }
    }
}

impl FromLua for MaybeBig {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        let either = mlua::Either::<mlua::String, LuaBigString>::from_lua(value, lua)?;
        match either {
            mlua::Either::Left(str) => Ok(Self::Lua(str)),
            mlua::Either::Right(str) => Ok(Self::Big(str)),
        }
    }
}

impl IntoLua for MaybeBig {
    fn into_lua(self, lua: &Lua) -> mlua::prelude::LuaResult<mlua::prelude::LuaValue> {
        match self {
            MaybeBig::Lua(l) => l.into_lua(lua),
            MaybeBig::Big(big) => big.into_lua(lua),
        }
    }
}

/// LuaString::Bytes for [MaybeBig].
enum MaybeBigBytes<'a> {
    Lua(mlua::BorrowedBytes<'a>),
    Big(&'a [u8]),
}

impl<'a> AsRef<[u8]> for MaybeBigBytes<'a> {
    fn as_ref(&self) -> &[u8] {
        match self {
            MaybeBigBytes::Lua(l) => l.as_ref(),
            MaybeBigBytes::Big(b) => b,
        }
    }
}

impl<'a> Deref for MaybeBigBytes<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match self {
            MaybeBigBytes::Lua(l) => l,
            MaybeBigBytes::Big(b) => b,
        }
    }
}

/// string:split(sep) lua method.
fn lua_string_split<T>(lua: &Lua, (str, sep): (T, LuaRegex)) -> mlua::Result<mlua::Table>
where
    T: for<'a> LuaString<'a>,
{
    let result = lua.create_table()?;

    let bytes = str.as_bytes();
    for part in sep.split(&bytes) {
        result.push(str.create_string(lua, part)?)?;
    }

    Ok(result)
}

/// string:splitn(sep, n) lua method.
fn lua_string_splitn<T>(lua: &Lua, (str, sep, n): (T, LuaRegex, usize)) -> mlua::Result<mlua::Table>
where
    T: for<'a> LuaString<'a>,
{
    let result = lua.create_table()?;

    for part in sep.splitn(&str.as_bytes(), n) {
        result.push(str.create_string(lua, part)?)?;
    }

    Ok(result)
}

/// string:find(pattern, at) lua method.
fn lua_string_find<T>(
    lua: &Lua,
    str: T,
    pattern: LuaRegex,
    at: Option<usize>,
    slice: impl Fn(T, usize, usize) -> mlua::Result<T>,
) -> mlua::Result<(mlua::Value, mlua::Value, mlua::Value)>
where
    T: for<'a> LuaString<'a> + Clone,
{
    match pattern.find_at(&str.clone().as_bytes(), at.unwrap_or(1).saturating_sub(1)) {
        Some(m) => Ok((
            slice(str, m.start() + 1, m.end())?.into_lua(lua)?,
            (m.start() + 1).into_lua(lua)?,
            (m.end()).into_lua(lua)?,
        )),

        None => Ok((mlua::Value::Nil, mlua::Value::Nil, mlua::Value::Nil)),
    }
}

fn lua_string_replace<T>(
    lua: &Lua,
    (str, pattern, replacement, n): (T, LuaRegex, MaybeBig, Option<usize>),
) -> mlua::Result<T>
where
    T: for<'a> LuaString<'a>,
{
    let bytes: &[u8] = &replacement.as_bytes();
    str.create_string(
        lua,
        &pattern.replacen(&str.as_bytes(), n.unwrap_or(1), bytes),
    )
}

fn lua_string_replace_all<T>(
    lua: &Lua,
    (str, pattern, replacement): (T, LuaRegex, MaybeBig),
) -> mlua::Result<T>
where
    T: for<'a> LuaString<'a>,
{
    let replacement: &[u8] = &replacement.as_bytes();

    str.create_string(lua, &pattern.replace_all(&str.as_bytes(), replacement))
}

fn lua_string_captures<T>(
    lua: &Lua,
    str: T,
    pattern: LuaRegex,
    at: Option<usize>,
    slice: impl Fn(T, usize, usize) -> mlua::Result<T>,
) -> mlua::Result<mlua::Value>
where
    T: for<'a> LuaString<'a> + Clone,
{
    match pattern.captures_at(&str.as_bytes(), at.unwrap_or(0)) {
        Some(capture) => {
            let result = lua.create_table()?;
            let mut iter = pattern.capture_names().enumerate();
            iter.next();

            for (i, name) in iter {
                let m = match name {
                    Some(name) => capture.name(name),
                    None => capture.get(i),
                };

                if let Some(m) = m {
                    let start = m.start() + 1;
                    let end = m.end();
                    let substr = slice(str.clone(), start, end)?;

                    let tab = lua.create_table()?;
                    tab.push(substr.clone())?;
                    tab.push(start)?;
                    tab.push(end)?;
                    tab.push(name)?;

                    tab.set("match", substr)?;
                    tab.set("start", start)?;
                    tab.set("end", end)?;
                    tab.set("name", name)?;

                    result.push(&tab)?;
                    result.set(name, tab)?;
                }
            }

            result.into_lua(lua)
        }
        None => Ok(mlua::Value::Nil),
    }
}
