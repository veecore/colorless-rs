//! Bidirectional conversion traits between `syn` and CPS IR.

use crate::Result;

/// Conversion from `syn` AST nodes into CPS IR.
pub trait IntoCps<Target> {
    fn into_cps(self) -> Result<Target>;
}

/// Conversion from CPS IR back into standard `syn` AST nodes.
pub trait IntoSyn {
    type Output;
    fn into_syn(self) -> Self::Output;
}
