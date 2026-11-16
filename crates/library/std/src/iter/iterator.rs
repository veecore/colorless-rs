//! Core colored iterator traits: `ColoredIterator`, `IntoColoredIterator`.

use colorless_macros::colorless;
use core::ops::{Deref, DerefMut};

/// An iterator that evaluates across colors (`Identity`, `Async`, etc.).
///
/// Strictly mirrors `std::iter::Iterator` parity without lending:
/// each step produces an owned item `Self::Item`.
#[colorless]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub trait ColoredIterator {
    /// The type of the elements being iterated over.
    type Item;

    /// Advances the iterator and returns the next value.
    ///
    /// Returns `None` when iteration has finished.
    fn next(&mut self) -> Option<Self::Item>;

    /// Returns the bounds on the remaining length of the iterator.
    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }

    /// Borrows an iterator, rather than consuming it.
    #[exclude]
    fn by_ref(&mut self) -> &mut Self
    where
        Self: Sized,
    {
        self
    }
}

#[colorless(C)]
impl<I: ColoredIterator<C>> ColoredIterator for &mut I {
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.deref_mut().next().resolve
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.deref().size_hint()
    }
}

pub use bridge::*;

#[allow(unused_imports)]
mod bridge {
    use crate::iter::ColoredIterator;
    use colorless_core::FallibleAsync;
    use colorless_core::{Async, Identity};
    use colorless_macros::colorless;
    use core::task::Poll;

    // -------------------------------- std_iter --------------------------------

    /// Wrapper converting a standard [`core::iter::Iterator`] into a [`ColoredIterator`].
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[derive(Debug, Copy, Clone)]
    pub struct FromIter<I>(pub I);

    /// Converts any standard [`core::iter::Iterator`] into a [`ColoredIterator`].
    #[inline(always)]
    pub fn from_iter<I: ::core::iter::IntoIterator>(iter: I) -> FromIter<I::IntoIter> {
        FromIter(iter.into_iter())
    }

    #[colorless(color = Identity)]
    impl<I: ::core::iter::Iterator> ColoredIterator for FromIter<I> {
        type Item = I::Item;

        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }

        #[exclude]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }

    // -------------------------------- std_async_iter nightly --------------------------------

    /// Wrapper converting a standard [`core::async_iter::AsyncIterator`] into a [`ColoredIterator`].
    #[cfg(feature = "nightly")]
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[derive(Debug, Copy, Clone)]
    pub struct FromAsyncIter<I>(pub I);

    /// Converts a standard [`core::async_iter::AsyncIterator`] into a [`ColoredIterator`].
    #[cfg(feature = "nightly")]
    pub fn from_async_iter<I: core::async_iter::IntoAsyncIterator>(
        iter: I,
    ) -> FromAsyncIter<I::IntoAsyncIter> {
        FromAsyncIter(iter.into_async_iter())
    }

    #[cfg(feature = "nightly")]
    #[colorless(color = Async)]
    impl<I: core::async_iter::AsyncIterator + Unpin> ColoredIterator for FromAsyncIter<I> {
        type Item = I::Item;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            core::future::poll_fn(|cx| core::pin::Pin::new(&mut self.0).poll_next(cx))
        }

        #[exclude]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }

    // -------------------------------- futures_core stream --------------------------------

    /// Wrapper converting a [`futures_core::stream::Stream`] into a [`ColoredIterator`].
    #[cfg(feature = "futures-io")]
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[derive(Debug, Copy, Clone)]
    pub struct FromStream<S>(pub S);

    /// Converts a [`futures_core::stream::Stream`] into a [`ColoredIterator`].
    #[cfg(feature = "futures-io")]
    pub fn from_stream<S: futures_core::stream::Stream>(stream: S) -> FromStream<S> {
        FromStream(stream)
    }

    #[cfg(feature = "futures-io")]
    #[colorless(color = Async)]
    impl<S: futures_core::stream::Stream + Unpin> ColoredIterator for FromStream<S> {
        type Item = S::Item;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            core::future::poll_fn(|cx| core::pin::Pin::new(&mut self.0).poll_next(cx))
        }

        #[exclude]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }

    /// Wrapper converting a [`futures_core::stream::TryStream`] into a [`ColoredIterator`].
    #[cfg(feature = "futures-io")]
    #[must_use = "iterators are lazy and do nothing unless consumed"]
    #[derive(Debug, Copy, Clone)]
    pub struct FromTryStream<S>(pub S);

    /// Converts a [`futures_core::stream::TryStream`] into a [`ColoredIterator`].
    #[cfg(feature = "futures-io")]
    pub fn from_try_stream<S: futures_core::stream::TryStream>(stream: S) -> FromTryStream<S> {
        FromTryStream(stream)
    }

    #[cfg(feature = "futures-io")]
    #[colorless(color = FallibleAsync<S::Error>)]
    impl<S: futures_core::stream::TryStream + Unpin> ColoredIterator for FromTryStream<S> {
        type Item = S::Ok;

        #[inline]
        fn next(&mut self) -> Option<Self::Item> {
            // a special transpose color to allow single O(1) match would be cool
            core::future::poll_fn(
                |cx| match core::pin::Pin::new(&mut self.0).try_poll_next(cx) {
                    Poll::Ready(Some(Ok(x))) => Poll::Ready(Ok(Some(x))),
                    Poll::Ready(Some(Err(e))) => Poll::Ready(Err(e)),
                    Poll::Ready(None) => Poll::Ready(Ok(None)),
                    Poll::Pending => Poll::Pending,
                },
            )
        }

        #[exclude]
        fn size_hint(&self) -> (usize, Option<usize>) {
            self.0.size_hint()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use colorless_rt::AsyncExt;
    use colorless_rt::IdentityExt;
    #[allow(unused_imports)]
    use colorless_rt::test_util::block_on_test;

    #[test]
    fn test_from_iter_sync() {
        let v = [1, 2, 3];
        let mut it = from_iter(v.into_iter());
        assert_eq!(it.next().sync_(), Some(1));
        assert_eq!(it.next().sync_(), Some(2));
        assert_eq!(it.next().sync_(), Some(3));
        assert_eq!(it.next().sync_(), None);
    }

    #[test]
    fn test_by_ref() {
        let v = [1, 2];
        let mut it = from_iter(v.into_iter());
        let r = ColoredIterator::by_ref(&mut it);
        assert_eq!(r.next().sync_(), Some(1));
        assert_eq!(it.next().sync_(), Some(2));
    }

    #[test]
    fn test_by_ref_colorless() {
        #[colorless(C)]
        fn run<'a, I: ColoredIterator<C, Item = i32> + 'a>(mut it: I) -> Option<i32> {
            pin_local!(it);
            let r = it.by_ref();
            r.next().resolve
        }

        let v = [1, 2];
        let it = from_iter(v.into_iter());
        assert_eq!(run(it).sync_(), Some(1));
    }

    #[cfg(feature = "futures-io")]
    #[test]
    fn test_from_stream() {
        struct SimpleStream(i32);
        impl futures_core::stream::Stream for SimpleStream {
            type Item = i32;
            fn poll_next(
                mut self: core::pin::Pin<&mut Self>,
                _cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<Option<Self::Item>> {
                if self.0 < 3 {
                    let val = self.0;
                    self.0 += 1;
                    core::task::Poll::Ready(Some(val))
                } else {
                    core::task::Poll::Ready(None)
                }
            }
        }

        let mut it = from_stream(SimpleStream(0));
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(0)
        );
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(1)
        );
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(2)
        );
        assert_eq!(block_on_test(ColoredIterator::next(&mut it).async_()), None);
    }

    #[cfg(feature = "nightly")]
    #[test]
    fn test_from_async_iter() {
        struct SimpleAsyncIter(i32);
        impl core::async_iter::AsyncIterator for SimpleAsyncIter {
            type Item = i32;
            fn poll_next(
                mut self: core::pin::Pin<&mut Self>,
                _cx: &mut core::task::Context<'_>,
            ) -> core::task::Poll<Option<Self::Item>> {
                if self.0 < 3 {
                    let val = self.0;
                    self.0 += 1;
                    core::task::Poll::Ready(Some(val))
                } else {
                    core::task::Poll::Ready(None)
                }
            }
        }

        let mut it = from_async_iter(SimpleAsyncIter(0));
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(0)
        );
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(1)
        );
        assert_eq!(
            block_on_test(ColoredIterator::next(&mut it).async_()),
            Some(2)
        );
        assert_eq!(block_on_test(ColoredIterator::next(&mut it).async_()), None);
    }
}
