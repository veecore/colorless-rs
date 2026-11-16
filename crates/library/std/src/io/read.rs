//! Core colored I/O traits: `ColoredRead`, `ColoredWrite`.

use colorless_macros::colorless;
use std::io::Result;
use std::ops::DerefMut;

/// A trait for reading bytes across colors (`Identity`, `Async`, etc.).
#[colorless]
pub trait ColoredRead {
    /// Pulls some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Borrows a reader rather than consuming it.
    #[exclude]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

#[colorless(C)]
impl<R: ColoredRead<C>> ColoredRead for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.deref_mut().read(buf).resolve
    }

    #[exclude]
    fn by_ref(&mut self) -> &mut Self {
        self
    }
}

#[colorless]
impl ColoredRead for &[u8] {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        std::io::Read::read(self, buf)
    }
}

pub use bridge::*;

mod bridge {
    use crate::io::ColoredRead;
    #[allow(unused_imports)]
    use colorless_core::{Async, Identity};
    use colorless_macros::colorless;
    use std::io::Result;

    // -------------------------------- std_io --------------------------------

    /// Wrapper converting a standard [`std::io::Read`] into a [`ColoredRead`].
    #[derive(Debug)]
    pub struct FromStdRead<R>(pub R);

    /// Converts a standard [`std::io::Read`] into a [`ColoredRead`].
    #[cfg(feature = "std")]
    pub fn from_std_read<R: std::io::Read>(read: R) -> FromStdRead<R> {
        FromStdRead(read)
    }

    #[cfg(feature = "std")]
    #[colorless(color = Identity)]
    impl<R: std::io::Read> ColoredRead for FromStdRead<R> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            self.0.read(buf)
        }
    }

    // -------------------------------- futures_io --------------------------------

    /// Wrapper converting an async reader ([`futures_io::AsyncRead`]) into a [`ColoredRead`].
    #[cfg(feature = "futures-io")]
    #[derive(Debug)]
    pub struct FromFuturesAsyncRead<R>(pub R);

    /// Converts an async reader ([`futures_io::AsyncRead`]) into a [`ColoredRead`].
    #[cfg(feature = "futures-io")]
    pub fn from_futures_async_read<R: futures_io::AsyncRead>(read: R) -> FromFuturesAsyncRead<R> {
        FromFuturesAsyncRead(read)
    }

    #[cfg(feature = "futures-io")]
    #[colorless(color = Async)]
    impl<R: futures_io::AsyncRead + Unpin> ColoredRead for FromFuturesAsyncRead<R> {
        #[inline]
        fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            core::future::poll_fn(|cx| core::pin::Pin::new(&mut self.0).poll_read(cx, buf))
        }
    }

    // -------------------------------- tokio_io --------------------------------

    /// Wrapper converting a [`tokio::io::AsyncRead`] into a [`ColoredRead`].
    #[cfg(feature = "tokio")]
    #[derive(Debug)]
    pub struct FromTokioAsyncRead<R>(pub R);

    /// Converts a [`tokio::io::AsyncRead`] into a [`ColoredRead`].
    #[cfg(feature = "tokio")]
    pub fn from_tokio_async_read<R: tokio::io::AsyncRead>(read: R) -> FromTokioAsyncRead<R> {
        FromTokioAsyncRead(read)
    }

    #[cfg(feature = "tokio")]
    #[colorless(color = Async)]
    impl<R: tokio::io::AsyncRead + Unpin> ColoredRead for FromTokioAsyncRead<R> {
        #[inline]
        async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
            tokio::io::AsyncReadExt::read(&mut self.0, buf).await
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use colorless_rt::{AsyncExt, IdentityExt, test_util::block_on_test};

    #[test]
    fn test_slice_read_sync() {
        let mut data = &b"hello world"[..];
        let mut buf = [0u8; 5];
        let n = data.read(&mut buf).sync_().unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf, b"hello");
        assert_eq!(data, b" world");
    }

    #[test]
    fn test_slice_read_async() {
        let mut data = &b"colorless"[..];
        let mut buf = [0u8; 9];
        let n = block_on_test(data.read(&mut buf).async_()).unwrap();
        assert_eq!(n, 9);
        assert_eq!(&buf, b"colorless");
    }

    #[test]
    fn test_read_colorless_generic() {
        #[colorless(C)]
        fn read_five<R: ColoredRead<C>>(mut r: R) -> Result<[u8; 5]> {
            pin_local!(r);
            let mut buf = [0u8; 5];
            pin_local!(buf);
            r.read(buf).resolve?;
            unpin_local!(buf);
            Ok(buf)
        }

        let data = &b"abcdef"[..];
        assert_eq!(read_five(data).sync_().unwrap(), *b"abcde");
        assert_eq!(block_on_test(read_five(data).async_()).unwrap(), *b"abcde");

        #[cfg(feature = "futures-io")]
        {
            let cursor = &b"futures-read"[..];
            assert_eq!(
                block_on_test(read_five(from_futures_async_read(cursor)).async_()).unwrap(),
                *b"futur"
            );
        }

        #[cfg(feature = "tokio")]
        {
            let cursor = &b"tokio-read"[..];
            assert_eq!(
                block_on_test(read_five(from_tokio_async_read(cursor)).async_()).unwrap(),
                *b"tokio"
            );
        }
    }

    #[cfg(feature = "futures-io")]
    #[test]
    fn test_from_futures_async_read() {
        let cursor = &b"futures-read"[..];
        let mut r = from_futures_async_read(cursor);
        let mut buf = [0u8; 7];
        assert_eq!(
            block_on_test(ColoredRead::read(&mut r, &mut buf).async_()).unwrap(),
            7
        );
        assert_eq!(&buf, b"futures");
        assert_eq!(
            block_on_test(ColoredRead::read(&mut r, &mut buf).async_()).unwrap(),
            5
        );
        assert_eq!(&buf[..5], b"-read");
    }

    #[cfg(feature = "tokio")]
    #[test]
    fn test_from_tokio_async_read() {
        let cursor = &b"tokio-read"[..];
        let mut r = from_tokio_async_read(cursor);
        let mut buf = [0u8; 5];
        assert_eq!(
            block_on_test(ColoredRead::read(&mut r, &mut buf).async_()).unwrap(),
            5
        );
        assert_eq!(&buf, b"tokio");
        assert_eq!(
            block_on_test(ColoredRead::read(&mut r, &mut buf).async_()).unwrap(),
            5
        );
        assert_eq!(&buf, b"-read");
    }
}
