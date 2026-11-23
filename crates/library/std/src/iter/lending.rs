use colorless_macros::colorless;
use std::ops::{Deref, DerefMut};

#[colorless]
#[must_use = "iterators are lazy and do nothing unless consumed"]
pub trait ColoredLendingIterator {
    /// The type of the elements being iterated over.
    type Item<'a>
    where
        Self: 'a;

    /// Advances the iterator and returns the next value.
    ///
    /// Returns `None` when iteration has finished.
    fn next(&mut self) -> Option<Self::Item<'_>>;

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

#[colorless(no_wrap, C)]
impl<I: ColoredLendingIterator<C>> ColoredLendingIterator for &mut I {
    type Item<'a>
        = I::Item<'a>
    where
        Self: 'a;

    fn next(&mut self) -> Option<Self::Item<'_>> {
        self.deref_mut().next().resolve
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.deref().size_hint()
    }
}
