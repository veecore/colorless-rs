use crate::iter::ColoredIterator;
use colorless_core::Color;

/// Conversion into a [`ColoredIterator`].
pub trait IntoColoredIterator<C: Color> {
    /// The type of the elements being iterated over.
    type Item;

    /// Which kind of iterator are we turning this into?
    type IntoIter: ColoredIterator<C, Item = Self::Item>;

    /// Creates an iterator from a value.
    fn into_iter(self) -> Self::IntoIter;
}

impl<C: Color, I: ColoredIterator<C>> IntoColoredIterator<C> for I {
    type Item = I::Item;
    type IntoIter = I;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self
    }
}
