use core::marker::PhantomData;

/// An execution context that short-circuits on both `Err` and `None`.
///
/// The pipeline's native value is `Result<Option<T>, E>`:
/// - `Ok(Some(value))` — both present and successful; continues with `value`.
/// - `Ok(None)` — no value available; short-circuits and returns `Ok(None)`.
/// - `Err(e)` — error; short-circuits and returns `Err(e)`.
///
/// Think of `FallibleOption` as "try to find a value, and also propagate
/// I/O errors": a database lookup that may legitimately return 'not found'
/// (`Ok(None)`) or fail with a network error (`Err(e)`).
///
/// # `#[doc(alias = "ResultOption")]`
///
/// The layout is `Result<Option<T>, E>` (error outermost) — not
/// `Option<Result<T,E>>`. The alias `ResultOption` is searchable as a reminder.
///
/// # Extracting the result
///
/// ```rust
/// use colorless_core::{Color, Colored, FallibleOption, FallibleOptionExt};
///
/// fn lookup<C: Color>() -> impl Colored<C, Output = i32> { C::ready(7) }
///
/// let v: Result<Option<i32>, ()> = lookup::<FallibleOption<()>>().result_option_();
/// assert_eq!(v, Ok(Some(7)));
/// ```
#[doc(alias = "ResultOption")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FallibleOption<E>(pub(crate) PhantomData<E>);

pub use impls::FallibleOptionExt;

mod impls {
    use crate::FallibleOption;

    macro_rules! exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => FallibleOptionValue(Ok(None)),
                Ok(Some($v)) => $ok,
                Err(e) => FallibleOptionValue(Err(e)),
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => FallibleOptionValue(Ok(None)),
                Ok(Some($v)) => FallibleOptionValue(Ok(Some($ok))),
                Err(e) => FallibleOptionValue(Err(e)),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(None) => return FallibleOptionValue(Ok(None)),
                Ok(Some(ControlFlow::Break(b))) => return FallibleOptionValue(Ok(Some(b))),
                Ok(Some(ControlFlow::Continue($v))) => $ok,
                Err(e) => return FallibleOptionValue(Err(e)),
            }
        };
        (@init $expr:expr) => {
            FallibleOptionValue(Ok(Some($expr)))
        };
    }

    define_sync! {
        exec,
        <E> FallibleOption FallibleOptionValue
        Result<Option<T>, E>,
        Result<Option<Self::Output>, E>,
        result_option_ FallibleOptionExt
    }
}
