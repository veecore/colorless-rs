//! Experimental module for parsers

use crate::function::ColoredFnMut;
use colorless_macros::colorless;
use std::ops::ControlFlow;
use std::prelude::v1::Vec;

// we could convert iterators to this
// I'm thinking breaking the methods into capability traits might be good
// I think the current serde could use Iterator but then fails read
// We should be trying to reduce monomorphization

pub enum Reference<'source, 'borrow, T: ?Sized> {
    /// A reference into the original source.
    Source(&'source T),

    /// A reference whose lifetime is limited by the chunk.
    Scoped(&'borrow T),
}

pub trait Chunk<'source, 'call> {
    fn as_slice(&self) -> &[u8];

    fn into_reference(self) -> Reference<'source, 'call, [u8]>;

    fn len(&self) -> usize {
        self.as_slice().len()
    }
}

// actually, a parseread could decide to return several forms of chunk
#[colorless(Z, arrgh)]
pub trait ParseRead<'r> {
    // With type assoc like iterator... then we could have many adapters
    type Chunk<'_r, 'a>: Chunk<'_r, 'a>
    where
        'r: '_r,
        '_r: 'r,
        Self: 'a;

    fn peek<'a>(&'a mut self, size_hint: Option<usize>) -> Option<Self::Chunk<'r, 'a>>;

    // This is excluded because of my selfish interest in serde-json for now... but, really,
    // the content is now in memory
    #[exclude]
    fn discard_exact(&mut self, n: usize);

    // #[exclude]
    // fn position(&self) -> usize;

    fn read_with<T, F>(&mut self, mut w: F) -> T
    where
        for<'a> F: ColoredFnMut<Option<Self::Chunk<'r, 'a>>, Z, Output = ControlFlow<T, usize>>,
    {
        todo!()
    }

    fn read_matches<'a>(&'a mut self, mut expected: &[u8]) -> bool {
        todo!()
    }

    fn peek_byte(&mut self) -> Option<u8> {
        let b = self.peek(Some(1)).resolve;
        match b {
            Some(b) => b.as_slice().first().copied(),
            None => None,
        }
    }

    #[exclude]
    fn discard(&mut self) {
        self.discard_exact(1)
    }

    // http response could check the header content-length
    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

pub struct SliceChunk<'r>(&'r [u8]);

#[colorless(Z)]
impl<'r> ParseRead<'r> for &'r [u8] {
    type Chunk<'_r, 'a>
        = SliceChunk<'r>
    where
        'r: '_r,
        '_r: 'r,
        Self: 'a;

    fn peek<'a>(&'a mut self, size_hint: Option<usize>) -> Option<Self::Chunk<'r, 'a>> {
        let size = size_hint.map(|n| n).unwrap_or(self.len());
        self.get(..size).and_then(|chunk| Some(SliceChunk(chunk)))
    }

    #[exclude]
    fn discard_exact(&mut self, n: usize) {
        *self = &self[n..]
    }

    fn read_matches<'a>(&'a mut self, expected: &[u8]) -> bool {
        if self.starts_with(expected) {
            *self = &self[expected.len()..];
            true
        } else {
            let matched = prefix_mismatch_len(self, expected);
            *self = &self[matched..];
            false
        }
    }

    fn peek_byte(&mut self) -> Option<u8> {
        self.get(0).copied()
    }

    #[exclude]
    fn discard(&mut self) {
        *self = &self[1..]
    }

    #[exclude]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl<'r, 'a> Chunk<'r, 'a> for SliceChunk<'r> {
    #[inline]
    fn as_slice(&self) -> &[u8] {
        self.0
    }

    #[inline]
    fn into_reference(self) -> Reference<'r, 'a, [u8]> {
        Reference::Source(self.0)
    }
}

pub struct FromStdRead<R> {
    pub buf: Vec<u8>,
    pub r: R,
}

pub struct FromStdReadChunk<'a>(&'a [u8]);

impl<'source, 'call> Chunk<'source, 'call> for Reference<'source, 'call, [u8]> {
    fn as_slice(&self) -> &[u8] {
        match self {
            Reference::Source(s) => s,
            Reference::Scoped(s) => s,
        }
    }

    fn into_reference(self) -> Reference<'source, 'call, [u8]> {
        self
    }
}

fn prefix_mismatch_len(got: &[u8], expected: &[u8]) -> usize {
    got.iter()
        .zip(expected.iter())
        .take_while(|(a, b)| a == b)
        .count()
}
