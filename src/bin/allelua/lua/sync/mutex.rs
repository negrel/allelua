use std::{cell::RefCell, sync::Arc};

use mlua::{MetaMethod, UserData};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

#[derive(Debug)]
pub struct LuaMutex {
    sem: Arc<Semaphore>,
    permits: RefCell<Option<OwnedSemaphorePermit>>,
    value: mlua::Value,
}

impl LuaMutex {
    pub fn new(value: mlua::Value) -> Self {
        Self {
            sem: Arc::new(Semaphore::new(1)),
            value,
            permits: RefCell::new(None),
        }
    }
}

impl UserData for LuaMutex {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field("__type", "sync.Mutex")
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("lock", |_, m, ()| async move {
            let permit = m.sem.clone().acquire_owned().await.unwrap();
            m.permits.replace(Some(permit));
            Ok(m.value.clone())
        });

        methods.add_method("unlock", |_, m, ()| {
            m.permits.take();
            Ok(())
        });

        methods.add_meta_method(MetaMethod::ToString, |_, m, ()| {
            let address = m as *const _ as usize;
            Ok(format!("sync.Mutex 0x{address:x}"))
        });
    }
}
