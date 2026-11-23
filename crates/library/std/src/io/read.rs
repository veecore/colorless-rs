//! Core colored I/O traits: `ColoredRead`, `ColoredWrite`.

pub use crate::io::util::Bytes;
use colorless_macros::colorless;
use std::io::{IoSliceMut, Result};
use std::ops::{Deref, DerefMut};

/// A trait for reading bytes across colors (`Identity`, `Async`, etc.).
#[colorless(no_wrap)]
pub trait ColoredRead {
    /// Pulls some bytes from this source into the specified buffer, returning
    /// how many bytes were read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        let buf = bufs
            .iter_mut()
            .find(|b| !b.is_empty())
            .map_or(&mut [][..], |b| &mut **b);
        self.read(buf).resolve
    }

    fn read_byte(&mut self) -> Option<Result<u8>> {
        slow_read_byte(self).resolve
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Borrows a reader rather than consuming it.
    #[exclude]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }

    #[exclude]
    fn bytes(self) -> Bytes<Self>
    where
        Self: Sized,
    {
        bytes(self)
    }
}

#[colorless(no_wrap, C)]
impl<R: ColoredRead<C>> ColoredRead for &mut R {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.deref_mut().read(buf).resolve
    }

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.deref_mut().read_vectored(bufs).resolve
    }

    fn read_byte(&mut self) -> Option<Result<u8>> {
        self.deref_mut().read_byte().resolve
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.deref().size_hint()
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

    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        std::io::Read::read_vectored(self, bufs)
    }

    fn read_byte(&mut self) -> Option<Result<u8>> {
        if let Some((first, rest)) = self.split_first() {
            let first = *first;
            *self = rest;
            Some(Ok(first))
        } else {
            None
        }
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

use crate::io::util::{bytes, slow_read_byte};
pub use bridge::*;

mod bridge {
    use crate::io::ColoredRead;
    #[allow(unused_imports)]
    use colorless_core::{Async, Identity};
    use colorless_macros::colorless;
    use std::io::{IoSliceMut, Result};
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

        fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
            self.0.read_vectored(bufs)
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

        fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
            core::future::poll_fn(|cx| {
                core::pin::Pin::new(&mut self.0).poll_read_vectored(cx, bufs)
            })
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

        #[inline]
        async fn read_byte(&mut self) -> Option<Result<u8>> {
            match tokio::io::AsyncReadExt::read_u8(&mut self.0).await {
                Ok(b) => Some(Ok(b)),
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => None,
                Err(err) => Some(Err(err)),
            }
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
        fn read_five<'a, R: ColoredRead<C> + 'a>(mut r: R) -> Result<[u8; 5]> {
            pin!(r);
            let mut buf = [0u8; 5];
            pin!(buf);
            rb!(r).read(rb!(buf)).resolve?;
            unpin!(buf);
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

    #[test]
    fn read_byte() {
        let mut s = &[][..];
        assert!(ColoredRead::read_byte(&mut s).sync_().is_none());

        let mut s = &[1][..];
        ColoredRead::read_byte(&mut s).sync_().unwrap().unwrap();
    }
}
