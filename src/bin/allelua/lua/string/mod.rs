use std::ops::Deref;

use mlua::{BorrowedBytes, FromLua, IntoLua, Lua};

use crate::include_lua;

mod big;
mod regex;

pub use big::*;
pub use regex::*;

pub fn load_string(lua: &Lua) -> mlua::Result<()> {
    let string_extra = lua.create_table()?;
    string_extra.set(
        "split",
        lua.create_function(lua_string_split::<mlua::String>)?,
    )?;
    string_extra.set(
        "splitn",
        lua.create_function(lua_string_splitn::<mlua::String>)?,
    )?;
    string_extra.set(
        "find",
        lua.create_function(lua_string_find::<mlua::String>)?,
    )?;
    string_extra.set(
        "replace",
        lua.create_function(lua_string_replace::<mlua::String>)?,
    )?;
    string_extra.set(
        "replace_all",
        lua.create_function(lua_string_replace_all::<mlua::String>)?,
    )?;

    // {
    //     let slice = slice.clone();
    //     string_extra.set(
    //         "captures",
    //         lua.create_function(
    //             move |lua,
    //                   (str, pattern, at): (
    //                 mlua::String,
    //                 Either<mlua::String, Regex>,
    //                 Option<usize>,
    //             )| {
    //                 let pattern = regex_or_escaped_regex(pattern)?;
    //
    //                 match pattern.captures_at(&str.as_bytes(), at.unwrap_or(0)) {
    //                     Some(capture) => {
    //                         let result = lua.create_table()?;
    //                         let mut iter = pattern.capture_names().enumerate();
    //                         iter.next();
    //
    //                         for (i, name) in iter {
    //                             let m = match name {
    //                                 Some(name) => capture.name(name),
    //                                 None => capture.get(i),
    //                             };
    //
    //                             if let Some(m) = m {
    //                                 let start = m.start() + 1;
    //                                 let end = m.end();
    //                                 let substr = slice.call::<mlua::Value>((
    //                                     &str,
    //                                     m.start() + 1,
    //                                     m.end(),
    //                                 ))?;
    //
    //                                 let tab = lua.create_table()?;
    //                                 tab.push(&substr)?;
    //                                 tab.push(start)?;
    //                                 tab.push(end)?;
    //                                 tab.push(name)?;
    //
    //                                 tab.set("match", substr)?;
    //                                 tab.set("start", start)?;
    //                                 tab.set("end", end)?;
    //                                 tab.set("name", name)?;
    //
    //                                 result.push(&tab)?;
    //                                 result.set(name, tab)?;
    //                             }
    //                         }
    //
    //                         result.into_lua(lua)
    //                     }
    //                     None => Ok(mlua::Value::Nil),
    //                 }
    //             },
    //         )?,
    //     )?;
    // }

    let big_string_constructors = lua.create_table()?;
    big_string_constructors.set(
        "fromstring",
        lua.create_function(|_lua, str: mlua::String| Ok(LuaBigString::from(str.as_bytes())))?,
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
/// can be used for generic methods. [MaybeBig] also implements it and forword
/// calls to appropriate variant.
trait LuaString<'a>: IntoLua {
    type Bytes: Deref<Target = [u8]>;

    fn as_bytes(&'a self) -> Self::Bytes;

    fn create_string(lua: &Lua, bytes: &[u8]) -> mlua::Result<Self>;
}

impl<'a> LuaString<'a> for mlua::String {
    type Bytes = BorrowedBytes<'a>;

    fn as_bytes(&'a self) -> Self::Bytes {
        self.as_bytes()
    }

    fn create_string(lua: &Lua, bytes: &[u8]) -> mlua::Result<Self> {
        lua.create_string(bytes)
    }
}

#[derive(Debug)]
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

    fn create_string(_lua: &Lua, _bytes: &[u8]) -> mlua::Result<Self> {
        panic!()
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
        result.push(T::create_string(lua, part)?)?;
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
        result.push(lua.create_string(part)?)?;
    }

    Ok(result)
}

/// string:find(pattern, at) lua method.
fn lua_string_find<T>(
    lua: &Lua,
    (str, pattern, at): (T, LuaRegex, Option<usize>),
) -> mlua::Result<(mlua::Value, mlua::Value)>
where
    T: for<'a> LuaString<'a>,
{
    match pattern.find_at(&str.as_bytes(), at.unwrap_or(0)) {
        Some(m) => Ok(((m.start() + 1).into_lua(lua)?, (m.end()).into_lua(lua)?)),
        None => Ok((mlua::Value::Nil, mlua::Value::Nil)),
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
    T::create_string(
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

    T::create_string(lua, &pattern.replace_all(&str.as_bytes(), replacement))
}
