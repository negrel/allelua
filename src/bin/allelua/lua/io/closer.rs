/// Close a previously opened resource.
pub trait Close {
    fn close(&mut self) -> mlua::Result<()>;
}

impl<T> Close for Option<T> {
    fn close(&mut self) -> mlua::Result<()> {
        self.ok_or_broken_pipe()?;
        drop(self.take());
        Ok(())
    }
}

/// MaybeClosed define a trait auto implemented.
pub trait MaybeClosed<T>
where
    Self: Sized,
{
    fn ok_or_broken_pipe(&mut self) -> mlua::Result<&mut T>;
}

impl<T> MaybeClosed<T> for Option<T> {
    fn ok_or_broken_pipe(&mut self) -> mlua::Result<&mut T> {
        match self {
            Some(t) => Ok(t),
            None => Err(mlua::Error::external(super::LuaError::from(
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "closed resource"),
            ))),
        }
    }
}

pub fn add_io_closer_methods<
    T,
    C: MaybeClosed<T> + Close,
    R: AsMut<C> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_method_mut("close", |_lua, closer, ()| closer.as_mut().close())
}
