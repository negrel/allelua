use std::future::Future;

mod buffered;
mod unbuffered;

pub use buffered::*;
pub use unbuffered::*;

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum QueueError {
    #[error("queue is closed")]
    Closed,
}

/// Queue define common methods of async FIFO like queue.
pub trait Queue {
    fn push(&self, _: mlua::Value) -> impl Future<Output = Result<(), QueueError>>;
    fn pop(&self) -> impl Future<Output = Result<mlua::Value, QueueError>>;
    fn close(&self) -> bool;
    fn is_closed(&self) -> bool;
}
