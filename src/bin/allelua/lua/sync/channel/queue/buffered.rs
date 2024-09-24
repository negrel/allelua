use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    future::{poll_fn, Future},
    task::{Poll, Waker},
};

use super::{Queue, QueueError};

enum QueueState {
    Idle,
    WaitingForWrite,
    WaitingForRead,
}

/// BufferedQueue define a [Queue] with a fixed size internal buffer. Pushing
/// and popping are blocking if buffer is respectively full or empty.
#[derive(Debug)]
pub struct BufferedQueue {
    queue: RefCell<VecDeque<mlua::Value>>,
    waitlist: RefCell<VecDeque<Waker>>,
    closed: Cell<bool>,
}

impl BufferedQueue {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0, "capacity is not greater than 0");
        Self {
            queue: RefCell::new(VecDeque::with_capacity(cap)),
            waitlist: RefCell::new(VecDeque::new()),
            closed: Cell::new(false),
        }
    }

    fn state(&self) -> QueueState {
        // Queue is empty and there is waker in waitlist.
        if self.queue.borrow().is_empty() && !self.waitlist.borrow().is_empty() {
            // Waitlist contains pop() waiters.
            QueueState::WaitingForWrite

            // Queue is full.
        } else if self.queue.borrow().len() == self.queue.borrow().capacity() {
            // Waitlist contains push() waiters.
            QueueState::WaitingForRead
        } else {
            // Waitlist is empty.
            QueueState::Idle
        }
    }

    fn wake_waiter(&self) -> bool {
        if let Some(waker) = self.waitlist.borrow_mut().pop_front() {
            waker.wake();
            true
        } else {
            false
        }
    }

    fn push_waiter(&self, w: Waker) {
        self.waitlist.borrow_mut().push_back(w);
    }
}

impl Queue for BufferedQueue {
    fn push(&self, value: mlua::Value) -> impl Future<Output = Result<(), QueueError>> {
        poll_fn(move |cx| {
            if self.closed.get() {
                return Poll::Ready(Err(QueueError::Closed));
            }

            match self.state() {
                QueueState::Idle => {
                    self.queue.borrow_mut().push_back(value.clone());
                    Poll::Ready(Ok(()))
                }
                QueueState::WaitingForWrite => {
                    self.queue.borrow_mut().push_back(value.clone());
                    self.wake_waiter();
                    Poll::Ready(Ok(()))
                }
                QueueState::WaitingForRead => {
                    self.push_waiter(cx.waker().to_owned());
                    Poll::Pending
                }
            }
        })
    }

    fn pop(&self) -> impl Future<Output = Result<mlua::Value, QueueError>> {
        poll_fn(|cx| match self.state() {
            QueueState::Idle => match self.queue.borrow_mut().pop_front() {
                Some(value) => Poll::Ready(Ok(value)),
                None => {
                    if self.closed.get() {
                        return Poll::Ready(Err(QueueError::Closed));
                    }

                    self.push_waiter(cx.waker().to_owned());
                    Poll::Pending
                }
            },
            QueueState::WaitingForRead => {
                let value = self.queue.borrow_mut().pop_front().unwrap();
                self.wake_waiter();
                Poll::Ready(Ok(value))
            }
            QueueState::WaitingForWrite => {
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
        closed
    }

    fn is_closed(&self) -> bool {
        self.closed.get()
    }
}

impl Drop for BufferedQueue {
    fn drop(&mut self) {
        self.close();
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;
    use std::time::Duration;
    use tokio::time::Instant;

    use crate::lua::sync::channel::queue::{BufferedQueue, Queue, QueueError};

    const WAIT_DURATION: Duration = Duration::from_millis(10);

    #[tokio::test]
    async fn push_pop_within_bounds() {
        const CAP: usize = 10;
        let queue = Rc::new(BufferedQueue::new(CAP));

        for i in 0..CAP {
            let now = Instant::now();
            queue.push(mlua::Value::Integer(i as i64)).await.unwrap();
            // No wait.
            assert!(now.elapsed() < WAIT_DURATION);
        }

        for i in 0..CAP {
            let now = Instant::now();
            let j = queue.pop().await.unwrap();
            assert!(now.elapsed() < WAIT_DURATION);

            assert_eq!(mlua::Value::Integer(i as i64), j);
        }
    }

    #[tokio::test]
    async fn push_out_of_bounds() {
        const CAP: usize = 10;
        let queue = Rc::new(BufferedQueue::new(CAP));

        // Fill buffer.
        for i in 0..CAP {
            queue.push(mlua::Value::Integer(i as i64)).await.unwrap();
        }

        let set = tokio::task::LocalSet::new();

        // Push again so it goes to waitlist.
        let bufq = queue.clone();
        set.spawn_local(async move {
            let now = Instant::now();
            bufq.push(mlua::Value::Integer(-1)).await.unwrap();

            // push returned after pop from next task.
            assert!(now.elapsed() > WAIT_DURATION);
        });

        let bufq = queue.clone();
        set.spawn_local(async move {
            tokio::time::sleep(WAIT_DURATION).await;
            bufq.pop().await.unwrap();
        });

        set.await;
    }

    #[tokio::test]
    async fn pop_out_of_bounds() {
        const CAP: usize = 10;
        let queue = Rc::new(BufferedQueue::new(CAP));

        let set = tokio::task::LocalSet::new();

        // Pop will wait until a value is pushed.
        let bufq = queue.clone();
        set.spawn_local(async move {
            let now = Instant::now();
            bufq.pop().await.unwrap();

            // pop returned after push from next task.
            assert!(now.elapsed() > WAIT_DURATION);
        });

        let bufq = queue.clone();
        set.spawn_local(async move {
            tokio::time::sleep(WAIT_DURATION).await;
            bufq.push(mlua::Value::Nil).await.unwrap();
        });

        set.await;
    }

    #[tokio::test]
    async fn push_closed_error() {
        const CAP: usize = 10;
        let queue = Rc::new(BufferedQueue::new(CAP));

        queue.close();
        let result = queue.push(mlua::Value::Nil).await;
        assert_eq!(result, Err(QueueError::Closed));
    }

    #[tokio::test]
    async fn pop_closed_error() {
        const CAP: usize = 10;
        let queue = Rc::new(BufferedQueue::new(CAP));

        queue.close();
        let result = queue.pop().await;
        assert_eq!(result, Err(QueueError::Closed));
    }

    #[tokio::test]
    async fn push_out_of_bounds_closed_error() {
        const CAP: usize = 1;
        let queue = Rc::new(BufferedQueue::new(CAP));

        let set = tokio::task::LocalSet::new();

        // Pop a value then close buffer.
        let bufq = queue.clone();
        set.spawn_local(async move {
            // Pop works as buffer contains a value.
            bufq.pop().await.unwrap();

            bufq.close();
        });

        // Push two a value and close channel.
        let bufq = queue.clone();
        set.spawn_local(async move {
            assert!(!bufq.is_closed());
            bufq.push(mlua::Value::Nil).await.unwrap();

            // Second push fails.
            let result = bufq.push(mlua::Value::Nil).await;
            assert_eq!(result, Err(QueueError::Closed));
            assert!(bufq.is_closed());
        });

        set.await
    }

    #[tokio::test]
    async fn pop_out_of_bounds_closed_error() {
        const CAP: usize = 1;
        let queue = Rc::new(BufferedQueue::new(CAP));

        let set = tokio::task::LocalSet::new();

        // Push a value and close buffer.
        let bufq = queue.clone();
        set.spawn_local(async move {
            bufq.push(mlua::Value::Nil).await.unwrap();
            bufq.close();
        });

        // Pop two value.
        let bufq = queue.clone();
        set.spawn_local(async move {
            // Buffer is closed.
            assert!(bufq.is_closed());

            // Pop works as buffer contains a value.
            bufq.pop().await.unwrap();

            // Second pop fails.
            let result = bufq.pop().await;
            assert_eq!(result, Err(QueueError::Closed));
        });

        set.await
    }

    #[tokio::test]
    async fn bench_1_000_000_push_buffered_queue_1000_capacity() {
        const CAP: usize = 1000;
        const ITER: usize = 1_000_000;
        let queue = Rc::new(BufferedQueue::new(CAP));

        let set = tokio::task::LocalSet::new();

        // Push.
        let bufq = queue.clone();
        set.spawn_local(async move {
            for i in 0..ITER {
                bufq.push(mlua::Value::Integer(i as i64)).await.unwrap();
            }
        });

        // Pop.
        let bufq = queue.clone();
        set.spawn_local(async move {
            for _ in 0..ITER {
                bufq.pop().await.unwrap();
            }
        });

        let now = Instant::now();
        set.await;
        println!(
            "buffered queue {ITER} iteration done in {:?}",
            now.elapsed()
        );
    }
}
