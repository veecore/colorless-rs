use crate::io::unstable_rust::is_interrupted;
use crate::io::{ColoredRead, Result};
use crate::iter::ColoredIterator;
use colorless_macros::colorless;
use std::slice;

#[derive(Debug)]
pub struct Bytes<R> {
    inner: R,
}

#[colorless(no_wrap, C)]
impl<R: ColoredRead<C>> ColoredIterator for Bytes<R> {
    type Item = Result<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.read_byte().resolve
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

#[colorless(no_wrap, C)]
pub(super) fn slow_read_byte<R: ColoredRead<C> + ?Sized>(reader: &mut R) -> Option<Result<u8>> {
    let mut byte = 0;
    borrow!(reader);
    #[state(mut byte, mut reader)]
    loop {
        pin!(byte);
        return match rb!(reader).read(slice::from_mut(rb!(byte))).resolve {
            Ok(0) => None,
            Ok(..) => {
                unpin!(byte);
                Some(Ok(byte))
            }
            Err(ref e) if is_interrupted(e) => continue,
            Err(e) => Some(Err(e)),
        };
    }
}

pub(super) const fn bytes<R>(inner: R) -> Bytes<R> {
    Bytes { inner }
}
