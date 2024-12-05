use tokio::sync::{Mutex, MutexGuard};

use crate::lua::error::{AlleluaError, LuaError};

/// Closes previously opened resource.
pub trait Close {
    fn close(&mut self) -> Result<(), LuaError>;
}

/// IO resource is closed. This error is returned by [Closable].
#[derive(Debug, thiserror::Error)]
#[error("io.Error(kind={})", self.kind())]
pub struct LuaIoClosedError;

impl AlleluaError for LuaIoClosedError {
    fn type_name(&self) -> &'static str {
        "io.Error"
    }

    fn kind(&self) -> &'static str {
        "closed"
    }
}

impl From<LuaIoClosedError> for mlua::Error {
    fn from(value: LuaIoClosedError) -> Self {
        LuaError::from(value).into()
    }
}

/// Closable is a wrapper around T that prevent access to T once `close()` was
/// called.
#[derive(Debug)]
pub struct Closable<T>(Option<Mutex<T>>);

impl<T> Drop for Closable<T> {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl<T> Closable<T> {
    pub fn new(inner: T) -> Self {
        Self(Some(Mutex::new(inner)))
    }

    pub async fn get(&self) -> Result<MutexGuard<T>, LuaIoClosedError> {
        match &self.0 {
            Some(mutex) => {
                let guard = mutex.lock().await;
                Ok(guard)
            }
            None => Err(LuaIoClosedError),
        }
    }

    pub fn is_closed(&self) -> bool {
        self.0.is_none()
    }
}

impl<T> Close for Closable<T> {
    fn close(&mut self) -> Result<(), LuaError> {
        match self.0.take() {
            Some(_) => Ok(()),
            None => Err(LuaIoClosedError.into()),
        }
    }
}

pub fn add_io_close_methods<
    T: Unpin + 'static,
    R: AsMut<Closable<T>> + 'static,
    M: mlua::UserDataMethods<R>,
>(
    methods: &mut M,
) {
    methods.add_method_mut("close", |_, closer, ()| {
        closer.as_mut().close()?;
        Ok(())
    });
}
