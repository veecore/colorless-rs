use core::marker::PhantomData;

/// The typed-error-propagating execution context.
///
/// Under `Fallible<E>` every yield point is an implicit `Result` check.
/// Write steps in plain direct style — if any step produces `Err(e)`,
/// every subsequent step in the pipeline is **silently skipped** and the
/// whole computation returns `Err(e)` immediately.
///
/// ```rust
/// use colorless_core::{Color, Colored, Colorize, Fallible, FallibleExt};
///
/// #[derive(Debug, PartialEq)]
/// enum ParseError { Empty, NotANumber }
///
/// fn try_parse(s: &str) -> impl Colored<Fallible<ParseError>, Output = u32> {
///     <_ as Colorize<Fallible<ParseError>>>::colorize({
///         if s.is_empty() {
///             Err(ParseError::Empty)
///         } else {
///             s.parse::<u32>().map_err(|_| ParseError::NotANumber)
///         }
///     })
/// }
///
/// fn double_parse<'a, C: Color, D, F>(s: &'a str, parse: F) -> impl Colored<C, Output = u32>
/// where
///     F: FnOnce(&'a str) -> D,
///     D: Colored<C, Output = u32>
/// {
///     // No ? operator, no match — if parse returns Err the map never runs.
///     C::map(parse(s), |n| n * 2)
/// }
///
/// assert_eq!(double_parse("21", try_parse).result_(), Ok(42));
/// assert_eq!(double_parse("??", try_parse).result_(), Err(ParseError::NotANumber));
/// assert_eq!(double_parse("", try_parse).result_(),   Err(ParseError::Empty));
/// ```
///
/// # Short-circuiting is invisible at the call site
///
/// The pipeline author writes sequential steps with no error-handling code:
///
/// ```rust,ignore
/// let raw   = read_file(path).resolve;        // step A — may Err
/// let doc   = parse_document(raw).resolve;    // step B — may Err
/// let title = extract_title(doc).resolve;     // step C — may Err
/// render(title)
/// ```
///
/// When driven under `Fallible<E>`:
/// - If step A returns `Err(e)`, B and C never execute. Result: `Err(e)`.
/// - If step B returns `Err(e)`, C never executes. Result: `Err(e)`.
/// - If all steps return `Ok`, the pipeline completes. Result: `Ok(rendered)`.
///
/// The same function driven under [`Identity`](crate::Identity) treats every step as always
/// present.
///
/// # Short-circuiting in loops
///
/// An error in any loop iteration immediately exits the entire loop,
/// propagating `Err` without running remaining iterations:
///
/// ```rust
/// use colorless_core::{Color, Colored, Fallible, FallibleExt, OneOf2, Colorize};
/// use core::ops::ControlFlow;
///
/// #[derive(Debug, PartialEq)]
/// struct Bad(u32);
///
/// // Sums items, stopping immediately on any value > 100.
/// fn guarded_sum<C: Color>(items: Vec<impl Colored<C, Output = u32>>)
///     -> impl Colored<C, Output = u32>
/// {
///     C::fold((items, 0_u32), move |(mut items, acc)| {
///         match items.pop() {
///             None => <C as OneOf2>::arm_1(
///                 C::ready(ControlFlow::Break(acc))
///             ),
///             Some(step) => {
///                 // If step is Err, the fold stops here — no further
///                 // iterations, Err propagates directly to the caller.
///                 let next = C::map(step, move |n| {
///                     ControlFlow::Continue((items, acc + n))
///                 });
///                 <C as OneOf2>::arm_0(next)
///             }
///         }
///     })
/// }
///
/// fn colorize(t: Result<u32, Bad>) -> impl Colored<Fallible<Bad>, Output = u32> {
///    <_ as Colorize<Fallible<Bad>>>::colorize(t)
/// }
///
/// let good = vec![colorize(Ok(1)), colorize(Ok(2)), colorize(Ok(3))];
/// assert_eq!(guarded_sum(good).result_(), Ok(6));
///
/// let bad = vec![colorize(Ok(1)), colorize(Err(Bad(999))), colorize(Ok(3))];
/// assert_eq!(guarded_sum(bad).result_(), Err(Bad(999)));
/// ```
///
/// # Extracting the result
///
/// Call [`.result_()`](FallibleExt::result_) on any
/// `impl Colored<Fallible<E>, Output = T>` to obtain `Result<T, E>`:
///
/// ```rust
/// use colorless_core::{Color, Colored, Fallible, FallibleExt};
///
/// fn double<C: Color>(n: u32) -> impl Colored<C, Output = u32> { C::ready(n * 2) }
///
/// let r: Result<u32, ()> = double::<Fallible<()>>(5).result_();
/// assert_eq!(r, Ok(10));
/// ```
#[doc(alias = "Result")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Fallible<E>(pub(crate) PhantomData<E>);

pub use impls::FallibleExt;

mod impls {
    use crate::Fallible;

    macro_rules! exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                Err(err) => FallibleValue(Err(err)),
                Ok($v) => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                Err(err) => FallibleValue(Err(err)),
                Ok($v) => FallibleValue(Ok($ok)),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Ok(ControlFlow::Break(b)) => return FallibleValue(Ok(b)),
                Ok(ControlFlow::Continue($v)) => $ok,
                Err(e) => return FallibleValue(Err(e)),
            }
        };
        (@init $expr:expr) => {
            FallibleValue(Ok($expr))
        };
    }

    define_sync! {
        exec,
        <E> Fallible FallibleValue
        Result<T, E>,
        Result<Self::Output, E>,
        result_ FallibleExt
    }
}
