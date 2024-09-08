use std::{
    cell::Cell,
    future::Future,
    task::{Poll, Waker},
};

use mlua::UserData;

pub(super) struct LuaWaitGroup {
    counter: Cell<usize>,
    waiter: Cell<Option<Waiter>>,
}

impl LuaWaitGroup {
    pub fn new() -> Self {
        Self {
            counter: Cell::new(0),
            waiter: Cell::new(None),
        }
    }

    fn wait(&self) -> WaitGroupFuture {
        WaitGroupFuture { wg: self }
    }

    fn add(&self, n: usize) {
        self.counter.replace(self.counter.get() + n);
    }

    fn done(&self) {
        if self.counter.get() == 0 {
            panic!("negative WaitGroup counter");
        }

        self.counter.replace(self.counter.get() - 1);

        if self.counter.get() == 0 {
            let waiter = self.waiter.take();
            if let Some(waiter) = waiter {
                waiter.wake();
            }
        }
    }
}

struct WaitGroupFuture<'a> {
    wg: &'a LuaWaitGroup,
}

impl<'a> Future for WaitGroupFuture<'a> {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        if self.wg.counter.get() == 0 {
            return Poll::Ready(());
        }

        let next_waiter = self.wg.waiter.take();
        self.wg.waiter.replace(Some(Waiter {
            next: Box::new(next_waiter),
            waker: cx.waker().clone(),
        }));

        Poll::Pending
    }
}

struct Waiter {
    waker: Waker,
    next: Box<Option<Waiter>>,
}

impl Waiter {
    fn wake(self) {
        if self.next.is_some() {
            self.next.unwrap().wake();
        }

        self.waker.wake_by_ref()
    }
}

impl UserData for LuaWaitGroup {
    fn add_fields<'lua, F: mlua::UserDataFields<'lua, Self>>(fields: &mut F) {
        fields.add_field("__type", "WaitGroup");

        fields.add_field_method_get("count", |_, wg| Ok(wg.counter.get()))
    }

    fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(mlua::MetaMethod::ToString, |_, wg, ()| {
            let address = wg as *const _ as usize;
            Ok(format!(
                "WaitGroup(counter={}) 0x{address:x}",
                wg.counter.get()
            ))
        });

        methods.add_method("add", |_, wg, n: usize| {
            wg.add(n);
            Ok(())
        });
        methods.add_method("done", |_, wg, ()| {
            wg.done();
            Ok(())
        });
        methods.add_async_method("wait", |_, wg, ()| async {
            wg.wait().await;
            Ok(())
        })
    }
}
