use core::marker::PhantomData;

/// An execution context where the pipeline may be absent or may have failed.
///
/// The pipeline's native value is `Option<Result<T, E>>`:
/// - `Some(Ok(value))` — present and successful; continues with `value`.
/// - `Some(Err(e))` — present but failed; short-circuits.
/// - `None` — entirely absent (no attempt was made or no context existed).
///
/// Use `OptionalResult` when the outer envelope is optional and the inner
/// attempt may fail — for example, a configuration section that may not exist
/// at all (`None`) or may exist but be malformed (`Some(Err(…))`).
///
/// # `#[doc(alias = "OptionResult")]`
///
/// Layout is `Option<Result<T,E>>` (option outermost). The alias is searchable.
///
/// # Extracting the result
///
/// ```rust
/// use colorless_core::{Color, Colored, OptionalResult, OptionalResultExt};
///
/// fn section<C: Color>() -> impl Colored<C, Output = i32> { C::ready(3) }
///
/// let v: Option<Result<i32, ()>> = section::<OptionalResult<()>>().option_result_();
/// assert_eq!(v, Some(Ok(3)));
/// ```
#[doc(alias = "OptionResult")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OptionalResult<E>(pub(crate) PhantomData<E>);

pub use impls::OptionalResultExt;

mod impls {
    use crate::OptionalResult;
    
    macro_rules! exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                None => OptionalResultValue(None),
                Some(Ok($v)) => $ok,
                Some(Err(e)) => OptionalResultValue(Some(Err(e))),
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => OptionalResultValue(None),
                Some(Ok($v)) => OptionalResultValue(Some(Ok($ok))),
                Some(Err(e)) => OptionalResultValue(Some(Err(e))),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => return OptionalResultValue(None),
                Some(Ok(ControlFlow::Break(b))) => return OptionalResultValue(Some(Ok(b))),
                Some(Ok(ControlFlow::Continue($v))) => $ok,
                Some(Err(e)) => return OptionalResultValue(Some(Err(e))),
            }
        };
        (@init $expr:expr) => {
            OptionalResultValue(Some(Ok($expr)))
        };
    }

    define_sync! {
        exec,
        <E> OptionalResult OptionalResultValue
        Option<Result<T, E>>,
        Option<Result<Self::Output, E>>,
        option_result_ OptionalResultExt
    }
}
