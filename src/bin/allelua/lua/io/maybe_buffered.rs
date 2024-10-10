use tokio::io::{AsyncRead, AsyncWrite, BufReader, BufStream, BufWriter};

/// [MaybeBuffered] is a trait implemented by [tokio::io::BufReader],
/// [tokio::io::BufWriter], [tokio::io::BufStream].
pub trait MaybeBuffered<T> {
    fn get_ref(&self) -> &T;
    fn get_mut(&mut self) -> &mut T;
    fn into_inner(self) -> T;
}

impl<T> MaybeBuffered<T> for T {
    fn get_ref(&self) -> &T {
        self
    }

    fn get_mut(&mut self) -> &mut T {
        self
    }

    fn into_inner(self) -> T {
        self
    }
}

impl<T: AsyncRead> MaybeBuffered<T> for BufReader<T> {
    fn get_ref(&self) -> &T {
        self.get_ref()
    }

    fn get_mut(&mut self) -> &mut T {
        self.get_mut()
    }

    fn into_inner(self) -> T {
        self.into_inner()
    }
}

impl<T: AsyncWrite> MaybeBuffered<T> for BufWriter<T> {
    fn get_ref(&self) -> &T {
        self.get_ref()
    }

    fn get_mut(&mut self) -> &mut T {
        self.get_mut()
    }

    fn into_inner(self) -> T {
        self.into_inner()
    }
}

impl<T: AsyncRead + AsyncWrite> MaybeBuffered<T> for BufStream<T> {
    fn get_ref(&self) -> &T {
        self.get_ref()
    }

    fn get_mut(&mut self) -> &mut T {
        self.get_mut()
    }

    fn into_inner(self) -> T {
        self.into_inner()
    }
}
