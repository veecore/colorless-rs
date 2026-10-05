//! Zero-macro typed AST builders and helper factories for CPS IR.

use super::CpsBlock;

#[inline]
#[must_use]
pub fn empty_block() -> CpsBlock {
    CpsBlock::default()
}
