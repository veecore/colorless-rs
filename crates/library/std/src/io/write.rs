use colorless_macros::colorless;
use std::io::Result;
use std::ops::DerefMut;

/// A trait for writing bytes across colors (`Identity`, `Async`, etc.).
#[colorless]
pub trait ColoredWrite {
    /// Writes a buffer into this writer, returning how many bytes were written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    /// Flushes this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    fn flush(&mut self) -> Result<()>;

    /// Borrows a writer rather than consuming it.
    #[exclude]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

#[colorless(C)]
impl<W: ColoredWrite<C>> ColoredWrite for &mut W {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.deref_mut().write(buf).resolve
    }

    fn flush(&mut self) -> Result<()> {
        self.deref_mut().flush().resolve
    }

    #[exclude]
    fn by_ref(&mut self) -> &mut Self {
        self
    }
}

#[colorless]
impl ColoredWrite for &mut [u8] {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        std::io::Write::write(self, buf)
    }

    fn flush(&mut self) -> Result<()> {
        std::io::Write::flush(self)
    }
}

#[cfg(feature = "alloc")]
#[colorless]
impl ColoredWrite for alloc::vec::Vec<u8> {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        std::io::Write::write(self, buf)
    }

    fn flush(&mut self) -> Result<()> {
        std::io::Write::flush(self)
    }
}

pub use bridge::*;
mod bridge {
    use crate::io::ColoredWrite;
    #[allow(unused_imports)]
    use colorless_core::{Async, Identity};
    use colorless_macros::colorless;
    use std::io::Result;

    // -------------------------------- std_io --------------------------------

    /// Wrapper converting a standard [`std::io::Write`] into a [`ColoredWrite`].
    #[cfg(feature = "std")]
    #[derive(Debug)]
    pub struct FromStdWrite<W>(pub W);

    /// Converts a standard [`std::io::Write`] into a [`ColoredWrite`].
    #[cfg(feature = "std")]
    pub fn from_std_write<W: std::io::Write>(write: W) -> FromStdWrite<W> {
        FromStdWrite(write)
    }

    #[cfg(feature = "std")]
    #[colorless(color = Identity)]
    impl<W: std::io::Write> ColoredWrite for FromStdWrite<W> {
        #[inline]
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            self.0.write(buf)
        }

        #[inline]
        fn flush(&mut self) -> Result<()> {
            self.0.flush()
        }
    }

    // -------------------------------- futures_io --------------------------------

    /// Wrapper converting an async writer ([`futures_io::AsyncWrite`]) into a [`ColoredWrite`].
    #[cfg(feature = "futures-io")]
    #[derive(Debug)]
    pub struct FromFuturesAsyncWrite<W>(pub W);

    /// Converts an async writer ([`futures_io::AsyncWrite`]) into a [`ColoredWrite`].
    #[cfg(feature = "futures-io")]
    pub fn from_futures_async_write<W: futures_io::AsyncWrite>(
        write: W,
    ) -> FromFuturesAsyncWrite<W> {
        FromFuturesAsyncWrite(write)
    }

    #[cfg(feature = "futures-io")]
    #[colorless(color = Async)]
    impl<W: futures_io::AsyncWrite + Unpin> ColoredWrite for FromFuturesAsyncWrite<W> {
        #[inline]
        fn write(&mut self, buf: &[u8]) -> Result<usize> {
            core::future::poll_fn(|cx| core::pin::Pin::new(&mut self.0).poll_write(cx, buf))
        }

        #[inline]
        fn flush(&mut self) -> Result<()> {
            core::future::poll_fn(|cx| core::pin::Pin::new(&mut self.0).poll_flush(cx))
        }
    }

    // -------------------------------- tokio_io --------------------------------

    /// Wrapper converting a [`tokio::io::AsyncWrite`] into a [`ColoredWrite`].
    #[cfg(feature = "tokio")]
    #[derive(Debug)]
    pub struct FromTokioAsyncWrite<W>(pub W);

    /// Converts a [`tokio::io::AsyncWrite`] into a [`ColoredWrite`].
    #[cfg(feature = "tokio")]
    pub fn from_tokio_async_write<W: tokio::io::AsyncWrite>(write: W) -> FromTokioAsyncWrite<W> {
        FromTokioAsyncWrite(write)
    }

    #[cfg(feature = "tokio")]
    #[colorless(color = Async)]
    impl<W: tokio::io::AsyncWrite + Unpin> ColoredWrite for FromTokioAsyncWrite<W> {
        #[inline]
        async fn write(&mut self, buf: &[u8]) -> Result<usize> {
            tokio::io::AsyncWriteExt::write(&mut self.0, buf).await
        }

        #[inline]
        async fn flush(&mut self) -> Result<()> {
            tokio::io::AsyncWriteExt::flush(&mut self.0).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colorless_rt::{AsyncExt, IdentityExt, test_util::block_on_test};

    #[test]
    fn test_vec_write_sync() {
        let mut out = alloc::vec::Vec::new();
        let n = out.write(b"foo").sync_().unwrap();
        assert_eq!(n, 3);
        out.write(b"bar").sync_().unwrap();
        assert_eq!(&out[..], b"foobar");
    }

    #[test]
    fn test_vec_write_async() {
        let mut out = alloc::vec::Vec::new();
        block_on_test(out.write(b"async").async_()).unwrap();
        block_on_test(out.flush().async_()).unwrap();
        assert_eq!(&out[..], b"async");
    }

    #[test]
    fn test_write_colorless_generic() {
        #[colorless(C)]
        fn write_bytes<'a, W: ColoredWrite<C> + 'a>(mut w: W, data: &'a [u8]) -> Result<usize> {
            pin_local!(w);
            let n = w.write(data).resolve?;
            unpin_local!(w);
            Ok(n)
        }

        let mut out = alloc::vec::Vec::new();
        assert_eq!(write_bytes(&mut out, b"hello").sync_().unwrap(), 5);
        assert_eq!(&out[..], b"hello");

        #[cfg(feature = "futures-io")]
        {
            let mut out = alloc::vec::Vec::new();
            assert_eq!(
                block_on_test(write_bytes(from_futures_async_write(&mut out), b"fut").async_())
                    .unwrap(),
                3
            );
            assert_eq!(&out[..], b"fut");
            assert_eq!(
                block_on_test(write_bytes(from_futures_async_write(&mut out), b"-async").async_())
                    .unwrap(),
                6
            );
            assert_eq!(&out[..], b"fut-async");
        }

        #[cfg(feature = "tokio")]
        {
            let mut out = alloc::vec::Vec::new();
            assert_eq!(
                block_on_test(write_bytes(from_tokio_async_write(&mut out), b"tok").async_())
                    .unwrap(),
                3
            );
            assert_eq!(&out[..], b"tok");
            assert_eq!(
                block_on_test(write_bytes(from_tokio_async_write(&mut out), b"-async").async_())
                    .unwrap(),
                6
            );
            assert_eq!(&out[..], b"tok-async");
        }
    }

    #[cfg(feature = "futures-io")]
    #[test]
    fn test_from_futures_async_write() {
        let mut out = alloc::vec::Vec::new();
        {
            let mut w = from_futures_async_write(&mut out);
            assert_eq!(
                block_on_test(ColoredWrite::write(&mut w, b"fut-write").async_()).unwrap(),
                9
            );
            block_on_test(ColoredWrite::flush(&mut w).async_()).unwrap();
        }
        assert_eq!(&out[..], b"fut-write");

        {
            let mut w = from_futures_async_write(&mut out);
            assert_eq!(
                block_on_test(ColoredWrite::write(&mut w, b"-async").async_()).unwrap(),
                6
            );
            block_on_test(ColoredWrite::flush(&mut w).async_()).unwrap();
        }
        assert_eq!(&out[..], b"fut-write-async");
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn test_from_tokio_async_write() {
        let mut out = alloc::vec::Vec::new();
        {
            let mut w = from_tokio_async_write(&mut out);
            assert_eq!(
                block_on_test(ColoredWrite::write(&mut w, b"tokio-write").async_()).unwrap(),
                11
            );
            block_on_test(ColoredWrite::flush(&mut w).async_()).unwrap();
        }
        assert_eq!(&out[..], b"tokio-write");

        {
            let mut w = from_tokio_async_write(&mut out);
            assert_eq!(
                block_on_test(ColoredWrite::write(&mut w, b"-async").async_()).unwrap(),
                6
            );
            block_on_test(ColoredWrite::flush(&mut w).async_()).unwrap();
        }
        assert_eq!(&out[..], b"tokio-write-async");
    }
}
