use pin_project_lite::pin_project;
use std::io::{self, IoSlice, SeekFrom};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncBufRead, AsyncRead, AsyncSeek, AsyncWrite, ReadBuf};

use super::{BufReader, BufWriter};

pin_project! {
    /// Wraps a type that is [`AsyncWrite`] and [`AsyncRead`], and buffers its input and output.
    ///
    /// It can be excessively inefficient to work directly with something that implements [`AsyncWrite`]
    /// and [`AsyncRead`]. For example, every `write`, however small, has to traverse the syscall
    /// interface, and similarly, every read has to do the same. The [`BufWriter`] and [`BufReader`]
    /// types aid with these problems respectively, but do so in only one direction. `BufStream` wraps
    /// one in the other so that both directions are buffered. See their documentation for details.
    #[derive(Debug)]
    pub struct BufStream<RW> {
        #[pin]
        inner: BufReader<BufWriter<RW>>,
    }
}

impl<RW: AsyncRead + AsyncWrite> BufStream<RW> {
    /// Wraps a type in both [`BufWriter`] and [`BufReader`].
    ///
    /// See the documentation for those types and [`BufStream`] for details.
    pub fn new(stream: RW, reader_buf: Box<[u8]>, writer_buf: Box<[u8]>) -> BufStream<RW> {
        BufStream {
            inner: BufReader::new(BufWriter::new(stream, writer_buf), reader_buf),
        }
    }

    /// Gets a reference to the underlying I/O object.
    ///
    /// It is inadvisable to directly read from the underlying I/O object.
    pub fn get_ref(&self) -> &RW {
        self.inner.get_ref().get_ref()
    }

    /// Gets a mutable reference to the underlying I/O object.
    ///
    /// It is inadvisable to directly read from the underlying I/O object.
    pub fn get_mut(&mut self) -> &mut RW {
        self.inner.get_mut().get_mut()
    }

    /// Gets a pinned mutable reference to the underlying I/O object.
    ///
    /// It is inadvisable to directly read from the underlying I/O object.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut RW> {
        self.project().inner.get_pin_mut().get_pin_mut()
    }

    /// Consumes this `BufStream`, returning the underlying I/O object.
    ///
    /// Note that any leftover data in the internal buffer is lost.
    pub fn into_inner(self) -> RW {
        self.inner.into_inner().into_inner()
    }
}

impl<RW: AsyncRead + AsyncWrite> AsyncWrite for BufStream<RW> {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.project().inner.poll_write(cx, buf)
    }

    fn poll_write_vectored(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        bufs: &[IoSlice<'_>],
    ) -> Poll<io::Result<usize>> {
        self.project().inner.poll_write_vectored(cx, bufs)
    }

    fn is_write_vectored(&self) -> bool {
        self.inner.is_write_vectored()
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.project().inner.poll_shutdown(cx)
    }
}

impl<RW: AsyncRead + AsyncWrite> AsyncRead for BufStream<RW> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        self.project().inner.poll_read(cx, buf)
    }
}

/// Seek to an offset, in bytes, in the underlying stream.
///
/// The position used for seeking with `SeekFrom::Current(_)` is the
/// position the underlying stream would be at if the `BufStream` had no
/// internal buffer.
///
/// Seeking always discards the internal buffer, even if the seek position
/// would otherwise fall within it. This guarantees that calling
/// `.into_inner()` immediately after a seek yields the underlying reader
/// at the same position.
///
/// See [`AsyncSeek`] for more details.
///
/// Note: In the edge case where you're seeking with `SeekFrom::Current(n)`
/// where `n` minus the internal buffer length overflows an `i64`, two
/// seeks will be performed instead of one. If the second seek returns
/// `Err`, the underlying reader will be left at the same position it would
/// have if you called `seek` with `SeekFrom::Current(0)`.
impl<RW: AsyncRead + AsyncWrite + AsyncSeek> AsyncSeek for BufStream<RW> {
    fn start_seek(self: Pin<&mut Self>, position: SeekFrom) -> io::Result<()> {
        self.project().inner.start_seek(position)
    }

    fn poll_complete(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<u64>> {
        self.project().inner.poll_complete(cx)
    }
}

impl<RW: AsyncRead + AsyncWrite> AsyncBufRead for BufStream<RW> {
    fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
        self.project().inner.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        self.project().inner.consume(amt);
    }
}
