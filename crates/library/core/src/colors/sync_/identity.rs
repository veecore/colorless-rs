/// The synchronous, zero-cost execution context.
///
/// Evaluating a function with `C = Identity` compiles down to plain synchronous Rust:
/// - Yield points run immediately without suspension.
/// - Loops execute via standard machine loops.
#[doc(alias = "Sync")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Identity;

pub use impls::IdentityExt;

mod impls {
    use crate::Identity;

    macro_rules! exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                $v => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                $v => ReadyValue($ok),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                ControlFlow::Break(b) => return ReadyValue(b),
                ControlFlow::Continue($v) => $ok,
            }
        };
        (@init $expr:expr) => {
            ReadyValue($expr)
        };
    }

    define_sync! {
        exec,
        <> Identity ReadyValue
        T,
        Self::Output,
        sync_ IdentityExt
    }
}
