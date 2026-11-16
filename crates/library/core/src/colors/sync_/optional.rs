/// The optionally-short-circuiting execution context.
///
/// Under `Optional` every yield point is an implicit `Option` check.
/// Write steps in plain direct style — if any step produces `None`,
/// every subsequent step in the pipeline is **silently skipped** and the
/// whole computation returns `None` immediately.
///
/// ```rust
/// use colorless_core::{Color, Colored, Colorize, Optional, OptionalExt};
///
/// fn try_parse(s: &str) -> impl Colored<Optional, Output = u32> {
///     // If this returns None, the caller gets None.
///     // The function body never does anything else.
///     <_ as Colorize<Optional>>::colorize(s.parse::<u32>().ok().filter(|&n| n > 0))
/// }
///
/// fn format_with<'a, C: Color, D, F>(s: &'a str, with: F) -> impl Colored<C, Output = String>
/// where
///     F: FnOnce(&'a str) -> D,
///     D: Colored<C, Output = u32>
/// {
///     // Both steps written without any Option-checking code.
///     C::map(with(s), |n| format!("found: {n}"))
/// }
///
/// assert_eq!(format_with("42", try_parse).option_(), Some("found: 42".into()));
/// assert_eq!(format_with("0", try_parse).option_(), None);  // silently skipped
/// assert_eq!(format_with("??", try_parse).option_(), None); // silently skipped
/// ```
///
/// # Short-circuiting is invisible at the call site
///
/// This is the essential property of `Optional`. The pipeline author writes:
///
/// ```rust,ignore
/// let id      = parse_id(input).resolve;    // step A
/// let profile = fetch_profile(id).resolve;  // step B
/// let avatar  = download_avatar(profile).resolve; // step C
/// build_user(id, profile, avatar)
/// ```
///
/// At the call site there are no `?` operators, no `if let Some(...)` guards,
/// and no branching. Yet when driven under `Optional`:
/// - If step A is `None`, B and C never execute. Result: `None`.
/// - If step B is `None`, C never executes. Result: `None`.
/// - If all steps are `Some`, the pipeline completes. Result: `Some(user)`.
///
/// The same function driven under [`Identity`](crate::Identity) treats every step as always
/// present.
///
/// # Short-circuiting in loops
///
/// The short-circuit propagates through [`Loop::fold`](crate::Loop::fold) too. Each iteration
/// that returns `None` immediately exits the loop — no break condition, no
/// guard:
///
/// ```rust
/// use colorless_core::{Color, Colored, Optional, OptionalExt, OneOf2, Colorize};
/// use core::ops::ControlFlow;
/// use std::collections::VecDeque;
///
/// // Processes items until one is missing.
/// fn first_missing_index<C: Color>(items: &mut VecDeque<impl Colored<C, Output = u32>>) -> impl Colored<C, Output = usize> {
///     let original_len = items.len();
///     C::fold((0_usize, items), move |(i, mut items)| {
///         if i >= original_len {
///             return <C as OneOf2>::arm_1(C::ready(ControlFlow::Break(i)));
///         }
///
///         // If items[i] is None this iteration returns None,
///         // propagating None out of the entire fold immediately.
///         let step = C::map(items.pop_front().unwrap(), move |_opt| {
///             ControlFlow::Continue((i + 1, items))
///         });
///         <C as OneOf2>::arm_0(step)
///     })
/// }
///
/// fn colorize(t: Option<u32>) -> impl Colored<Optional, Output = u32> {
///    <_ as Colorize<Optional>>::colorize(t)
/// }
///
/// let mut data = VecDeque::from([colorize(Some(1)), colorize(Some(2)), colorize(None), colorize(Some(4))]);
/// // Under Optional: stops at index 2 (the None), returns None for the fold.
/// assert_eq!(first_missing_index(&mut data).option_(), None);
/// assert_eq!(data.len(), 1);
/// ```
///
/// # Extracting the result
///
/// Call [`.option_()`](OptionalExt::option_) on any `impl Colored<Optional, Output = T>`
/// to obtain `Option<T>`:
///
/// ```rust
/// use colorless_core::{Color, Colored, Optional, OptionalExt};
///
/// fn double<C: Color>(n: u32) -> impl Colored<C, Output = u32> { C::ready(n * 2) }
///
/// assert_eq!(double(5).option_(), Some(10));
/// ```
#[doc(alias = "Option")]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Optional;

pub use impls::OptionalExt;

mod impls {
    use crate::Optional;

    macro_rules! exec {
        ($c:expr => $v:ident $ok:expr) => {
            match $c {
                None => OptionalValue(None),
                Some($v) => $ok,
            }
        };
        (@map $c:expr => $v:ident $ok:expr) => {
            match $c {
                None => OptionalValue(None),
                Some($v) => OptionalValue(Some($ok)),
            }
        };
        (@loop $c:expr => $v:ident $ok:expr) => {
            match $c {
                Some(ControlFlow::Break(b)) => return OptionalValue(Some(b)),
                Some(ControlFlow::Continue($v)) => $ok,
                None => return OptionalValue(None),
            }
        };
        (@init $expr:expr) => {
            OptionalValue(Some($expr))
        };
    }

    define_sync! {
        exec,
        <> Optional OptionalValue
        Option<T>,
        Option<Self::Output>,
        option_ OptionalExt
    }
}