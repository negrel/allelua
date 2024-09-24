use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::{poll_fn, Future},
    task::{Poll, Waker},
};

use super::{Queue, QueueError};

/// UnbufferedQueue define a [Queue] that always block.
#[derive(Debug, Default)]
pub struct UnbufferedQueue {
    slot: RefCell<Option<(mlua::Value, Waker)>>,
    waitlist: RefCell<VecDeque<Waker>>,
    closed: Cell<bool>,
}

impl UnbufferedQueue {
    fn push_waiter(&self, w: Waker) {
        self.waitlist.borrow_mut().push_back(w);
    }

    fn wake_waiter(&self) -> bool {
        if let Some(waker) = self.waitlist.borrow_mut().pop_front() {
            waker.wake();
            true
        } else {
            false
        }
    }
}

impl Queue for UnbufferedQueue {
    fn push(&self, value: mlua::Value) -> impl Future<Output = Result<(), QueueError>> {
        let mut waiting_pop = false;
        poll_fn(move |cx| {
            if self.closed.get() {
                return Poll::Ready(Err(QueueError::Closed));
            }

            if waiting_pop {
                self.wake_waiter();
                return Poll::Ready(Ok(()));
            }

            // Place value and wait for pop.
            if self.slot.borrow().is_some() {
                // Push back to end of waitlist.
                self.push_waiter(cx.waker().to_owned());
                self.wake_waiter();
                Poll::Pending
            } else {
                self.slot
                    .replace(Some((value.clone(), cx.waker().to_owned())));
                self.wake_waiter();
                waiting_pop = true;
                Poll::Pending
            }
        })
    }

    fn pop(&self) -> impl Future<Output = Result<mlua::Value, QueueError>> {
        poll_fn(|cx| {
            if self.closed.get() {
                return Poll::Ready(Err(QueueError::Closed));
            }

            if let Some((value, waker)) = self.slot.take() {
                // Wake associated push task.
                waker.wake();
                Poll::Ready(Ok(value))
            } else {
                self.push_waiter(cx.waker().to_owned());
                Poll::Pending
            }
        })
    }

    fn close(&self) -> bool {
        let closed = self.closed.replace(true);
        while let Some(waker) = self.waitlist.borrow_mut().pop_front() {
            waker.wake();
        }
        if let Some((_, waker)) = self.slot.take() {
            waker.wake();
        }
        closed
    }

    fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

impl Drop for UnbufferedQueue {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    use crate::lua::sync::channel::queue::{Queue, QueueError, UnbufferedQueue};

    const WAIT_DURATION: Duration = Duration::from_millis(10);

    #[tokio::test]
    async fn push_pop_blocks() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            let now = Instant::now();
            ubufq.push(mlua::Value::Nil).await.unwrap();
            assert!(now.elapsed() > WAIT_DURATION);
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            tokio::time::sleep(WAIT_DURATION).await;
            ubufq.pop().await.unwrap();
        });

        set.await;
    }

    #[tokio::test]
    async fn pop_push_blocks() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            let now = Instant::now();
            let value = ubufq.pop().await.unwrap();
            assert!(now.elapsed() > WAIT_DURATION);
            assert_eq!(value, mlua::Value::Nil);
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            tokio::time::sleep(WAIT_DURATION).await;
            ubufq.push(mlua::Value::Nil).await.unwrap();
        });

        set.await;
    }

    #[tokio::test]
    async fn push_push_pop_pop() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            let value = ubufq.pop().await.unwrap();
            assert_eq!(value, mlua::Value::Integer(1));

            let value = ubufq.pop().await.unwrap();
            assert_eq!(value, mlua::Value::Integer(2));
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.push(mlua::Value::Integer(1)).await.unwrap();
            ubufq.push(mlua::Value::Integer(2)).await.unwrap();
        });
    }

    #[tokio::test]
    async fn pop_pop_push_push() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            let value = ubufq.pop().await.unwrap();
            assert_eq!(value, mlua::Value::Integer(1));

            let value = ubufq.pop().await.unwrap();
            assert_eq!(value, mlua::Value::Integer(2));
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.push(mlua::Value::Integer(1)).await.unwrap();
            ubufq.push(mlua::Value::Integer(2)).await.unwrap();
        });

        set.await;
    }

    #[tokio::test]
    async fn pop_pop_push_close_push() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.pop().await.unwrap();
            let result = ubufq.pop().await;
            assert_eq!(result, Err(QueueError::Closed))
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.push(mlua::Value::Nil).await.unwrap();

            ubufq.close();

            let result = ubufq.push(mlua::Value::Nil).await;
            assert_eq!(result, Err(QueueError::Closed))
        });

        set.await;
    }

    #[tokio::test]
    async fn push_push_pop_close_pop() {
        let queue = Rc::new(UnbufferedQueue::default());
        let set = tokio::task::LocalSet::new();

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.push(mlua::Value::Nil).await.unwrap();

            let result = ubufq.push(mlua::Value::Nil).await;
            assert_eq!(result, Err(QueueError::Closed))
        });

        let ubufq = queue.clone();
        set.spawn_local(async move {
            ubufq.pop().await.unwrap();

            // This is required before close as pop wake associated push task but
            // tokio doesn't run it directly.
            tokio::task::yield_now().await;

            ubufq.close();

            let result = ubufq.pop().await;
            assert_eq!(result, Err(QueueError::Closed))
        });

        set.await;
    }

    #[tokio::test]
    async fn bench_1_000_000_push_unbuffered_queue() {
        const ITER: usize = 1_000_000;
        let queue = Rc::new(UnbufferedQueue::default());

        let set = tokio::task::LocalSet::new();

        // Push.
        let ubufq = queue.clone();
        set.spawn_local(async move {
            for i in 0..ITER {
                ubufq.push(mlua::Value::Integer(i as i64)).await.unwrap();
            }
        });

        // Pop.
        let ubufq = queue.clone();
        set.spawn_local(async move {
            for _ in 0..ITER {
                ubufq.pop().await.unwrap();
            }
        });

        let now = Instant::now();
        set.await;
        println!(
            "unbuffered queue {ITER} iteration done in {:?}",
            now.elapsed()
        );
    }
}
