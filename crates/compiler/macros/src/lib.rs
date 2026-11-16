//! # `colorless-macros`
//!
//! Procedural macros for Colorless: `#[colorless]` and `color!`.

mod config;
mod expr;
mod function;
mod impl_;
mod inert;
mod struct_;
mod trait_;
mod util;

use proc_macro::TokenStream;
use syn::{Item, parse_macro_input};

/// Transforms a function, trait, impl block, or struct to be generic over an
/// execution color.
///
/// A *color* is an execution strategy for a Colorless computation — it
/// controls whether `.resolve` runs synchronously, asynchronously, with
/// short-circuiting for `Option`, with propagation for `Result`, and so on.
/// By annotating an item with `#[colorless]`, you opt that item into the
/// colorless CPS transform, making it generic over whichever color the caller
/// supplies at instantiation time.
///
/// # Prerequisite — `relaxed-drop` feature
///
/// Before the macro will compile, you must acknowledge its current
/// destruction-timing limitation by enabling the `relaxed-drop` Cargo feature:
///
/// ```toml
/// # Cargo.toml
/// [dependencies]
/// colorless = { version = "...", features = ["relaxed-drop"] }
/// ```
///
/// Without this feature the macro emits a hard compile error explaining the
/// requirement. See [Destruction semantics](#destruction-semantics) for details.
///
/// # Applying to a function
///
/// ## Basic usage
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// fn double(x: u32) -> u32 {
///     x * 2
/// }
/// ```
///
/// The macro rewrites `double` into:
///
/// ```rust,ignore
/// fn double<__C: colorless::Color>(x: u32) -> impl colorless::Colored<__C, Output = u32> {
/// # stringify!(
///     ...
/// # )
/// }
/// ```
///
/// Callers can `.resolve` the return value under any color:
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// fn double(x: u32) -> u32 {
///     x * 2
/// }
///
/// // synchronous / Identity color
/// let n = double(21).resolve;
/// assert_eq!(n, 42);
/// ```
///
/// ## `.resolve` desugaring inside the body
///
/// Every `.resolve` expression inside a `#[colorless]` function is a CPS
/// yield-point. Under the `Identity` color each one runs synchronously and
/// returns its inner value immediately. Under the `Async` color each one
/// becomes an `await` point. Under `Optional` or `Fallible`, a `.resolve` that
/// produces `None` / `Err` short-circuits the rest of the function.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// fn fetch_and_double(x: impl IntoColored<__C, Output = u32>) -> u32 {
///     let v = x.resolve;      // yield-point: color-generic
///     v * 2
/// }
/// ```
///
/// ## The `#[colorless]` attribute accepts no arguments by default
///
/// Plain `#[colorless]` injects a fresh `__C: Color` type parameter. If `__C`
/// is already taken by an existing generic, the macro automatically picks
/// `__C1`, `__C2`, … to avoid shadowing.
///
/// # Argument forms
///
/// The attribute accepts zero or one argument.
///
/// ## `#[colorless]` — auto generic (default)
///
/// Injects a hidden color generic. The function becomes generic over all
/// colors.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// fn id(x: u32) -> u32 { x }
/// ```
///
/// ## `#[colorless(C)]` — named generic
///
/// Gives the injected color generic a user-visible name. Use this when you
/// want to reference the color in bounds, associated types, or return
/// position.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless(C)]
/// fn id<R: ColoredRead<C>>(x: R) -> u32 { x }
///                   // ^ already in scope; the macro uses it, not a fresh one
/// ```
///
/// Or without pre-declaring it — the macro injects it:
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless(MyColor)]
/// fn id(x: u32) -> u32 { x }
/// // expands to: fn id<MyColor: colorless::Color>(x: u32) -> impl Colored<MyColor, …>
/// ```
///
/// ## `#[colorless(color = Async)]` — monomorphise to a specific color
///
/// Pins the function to a single concrete color instead of making it generic.
/// This is useful inside `impl` blocks that implement a color-specific trait,
/// or when writing adapter code for a known execution context.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless(color = Async)]
/// async fn fetch(url: &str) -> String {
///     // body is lowered specifically for Async
///     todo!()
/// }
/// ```
///
/// > **Note:** `color = T` accepts any type, not just the built-in colors.
/// > You can pass a path to your own `Color` alias.
///
/// # Applying to a trait
///
/// When applied to a trait, `#[colorless]` injects the color generic into the
/// trait itself **and** rewrites every method signature inside it. Methods that
/// already have a default body are CPS-lowered; abstract methods (no body) only
/// have their signatures rewritten.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// pub trait Transformer {
///     fn transform(&self, x: u32) -> u32;
/// }
/// ```
///
/// Expands to roughly:
///
/// ```rust,ignore
/// pub trait Transformer<__C: colorless::Color> {
///     fn transform(&self, x: u32) -> impl colorless::Colored<__C, Output = u32>;
/// }
/// ```
///
/// ## Named color generic on a trait
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless(C)]
/// pub trait Transformer {
///     fn transform(&self, x: u32) -> u32;
/// }
/// // => pub trait Transformer<C: colorless::Color> { … }
/// ```
///
/// ## The `#[exclude]` inner attribute
///
/// Inside a `#[colorless]` trait or impl block, mark individual methods with
/// `#[exclude]` to opt them out of the transform. They are left exactly as
/// written.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// pub trait Mixed {
///     /// This method is colorless-generic.
///     fn colored_method(&self, x: u32) -> u32;
///
///     /// This method is excluded from the transform — stays plain Rust.
///     #[exclude]
///     fn plain_method(&self) -> &str;
/// }
/// ```
///
/// # Applying to an impl block
///
/// ## Generic impl (no `color =` argument)
///
/// Adds a color generic to the impl and rewrites all methods, mirroring what
/// `#[colorless]` does on the trait definition. The color argument is appended
/// to the trait path automatically.
///
/// ```rust,ignore
/// use colorless::*;
///
/// struct MyTransformer;
///
/// #[colorless]
/// pub trait Transformer {
///     fn transform(&self, x: u32) -> u32;
/// }
///
/// #[colorless]
/// impl Transformer for MyTransformer {
///     fn transform(&self, x: u32) -> u32 {
///         x * 2
///     }
/// }
/// ```
///
/// ## Specific-color impl (`color = SomeColor`)
///
/// When you supply a concrete color, the macro does **not** introduce a new
/// generic. Instead each method body is wrapped in a `colorize(|| …)` or
/// `colorize(async move { … })` call and the return type is rewritten to
/// `impl Colored<SomeColor, Output = …>`. This is the form used by the
/// built-in color `impl` blocks inside `colorless-core`.
///
/// ```rust,ignore
/// use colorless::*;
///
/// struct MyTransformer;
///
/// #[colorless]
/// pub trait Transformer {
///     fn transform(&self, x: u32) -> u32;
/// }
///
/// #[colorless(color = colorless::Identity)]
/// impl Transformer for MyTransformer {
///     fn transform(&self, x: u32) -> u32 {
///         x * 2    // runs synchronously
///     }
/// }
/// ```
///
/// # Applying to a struct
///
/// When applied to a struct, `#[colorless]` makes the struct carry a phantom
/// color parameter. This is useful for builder types that need to remember
/// which color they will be resolved under.
///
/// A hidden `PhantomData<__C>` field is injected:
/// - **Named structs**: as `__colorless_marker: PhantomData<__C>`
/// - **Tuple structs**: as an extra trailing `PhantomData<__C>` element
/// - **Unit structs**: converted to a one-element tuple struct `(PhantomData<__C>,)`
///
/// The default value of `__C` is `colorless::Identity`, so the struct remains
/// constructible without specifying a color turbofish.
///
/// ```rust,ignore
/// use colorless::*;
///
/// #[colorless]
/// pub struct FetchBuilder {
///     url: String,
/// }
/// // expands to:
/// // pub struct FetchBuilder<__C: colorless::Color = colorless::Identity> {
/// //     url: String,
/// //     __colorless_marker: core::marker::PhantomData<__C>,
/// // }
/// ```
///
/// # Restrictions
///
/// | Restriction | Reason |
/// |-------------|--------|
/// | Cannot be `async fn` | The function is already execution-generic; use `.resolve` instead of `.await` |
/// | Cannot be `const fn` | `const` traits are not yet stable in Rust |
/// | Only one argument allowed | `#[colorless(A, B)]` is a compile error |
/// | Cannot be applied to modules, enums, or type aliases | Only `fn`, `trait`, `impl`, and `struct` |
///
/// # Destruction semantics
///
/// **Current limitation:** the macro only supports *relaxed implicit-destruction
/// mode*. Under this mode, a local value that crosses a `.resolve` boundary may
/// be destroyed earlier or later than it would be in straight Rust, and
/// implicit sibling destruction order may differ from source order.
///
/// This is safe (no undefined behavior), but it means that code relying on
/// *when* destructors run — such as `MutexGuard`, file handles held across
/// yield points, or custom `Drop` impls with timing-sensitive side effects —
/// may behave unexpectedly.
///
/// A future *strict mode* will preserve source-order destruction. Until then,
/// avoid wrapping RAII-sensitive resources in `#[colorless]` functions.
///
/// # How the inert copy works
///
/// The macro emits **two** copies of the function body:
///
/// 1. **Inert copy** — a syntactically valid plain-Rust version of the body
///    where every `.resolve` is replaced by a call to
///    `__compiler::resolve_unreachable(…).resolve`. This copy is passed to
///    `never_execute_inert`, so rustc type-checks and borrow-checks it without
///    ever running it. This catches borrow errors, lifetime mismatches, and
///    `let-else` divergence before CPS lowering gets involved.
///
/// 2. **Lowered copy** — the actual CPS-transformed body, produced by
///    `colorless-codegen`.
///
/// At runtime only the lowered copy executes. The inert copy is dead code
/// that is optimized away by the compiler.
///
/// # Cross-references
///
/// - [`colorless::Color`] — the trait all colors implement
/// - [`colorless::Colored`] — the trait all colored computations implement;
///   the return type of every `#[colorless]` function
/// - [`colorless::IntoColored`] — builder-pattern entry point; arguments to
///   `#[colorless]` functions typically accept `impl IntoColored<C>`
/// - [`colorless::Resolvable`] — the transparent wrapper emitted when the
///   `autocomplete` feature is enabled, exposing the `.resolve` field for IDE
///   autocompletion
/// - [`color!`] — inline block form of the same transform
///
///  [`colorless::Color`]: https://docs.rs/colorless-core/latest/colorless_core/trait.Color.html
///  [`colorless::Colored`]: https://docs.rs/colorless-core/latest/colorless_core/trait.Colored.html
///  [`colorless::IntoColored`]: https://docs.rs/colorless-core/latest/colorless_core/trait.IntoColored.html
///  [`colorless::Resolvable`]: https://docs.rs/colorless-core/latest/colorless_core/struct.Resolvable.html
#[proc_macro_attribute]
pub fn colorless(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config = parse_macro_input!(attr as config::Config);
    let item = parse_macro_input!(item as Item);

    let expanded = match item {
        Item::Fn(f) => function::expand(config, f),
        Item::Trait(t) => trait_::expand(config, t),
        Item::Impl(i) => impl_::expand(config, i),
        Item::Struct(s) => struct_::expand(config, s),
        other => Err(syn::Error::new_spanned(
            other,
            "#[colorless] can only be applied to functions, trait definitions, structs, and impl blocks",
        )),
    };

    match expanded {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Evaluates an inline block expression as a colored computation.
///
/// `color!` is the block-expression counterpart of [`#[colorless]`](colorless).
/// Where `#[colorless]` annotates an entire item, `color!` wraps a single
/// expression block, making it possible to introduce a colorless computation
/// inline — without naming a function.
///
/// # Basic usage
///
/// ```rust,ignore
/// use colorless::*;
///
/// // The block is color-inferred from context.
/// let result = color! {
///     let x = some_colored_value.resolve;
///     x + 1
/// }.resolve;
/// ```
///
/// # Specifying a color
///
/// Pass an inner `#![colorless(color = …)]` attribute as the first item inside
/// the block to pin it to a specific color:
///
/// ```rust,ignore
/// use colorless::*;
///
/// let result = color! {
///     #![colorless(color = Async)]
///
///     let x = async_computation.resolve;
///     x + 1
/// }.resolve;
/// ```
///
/// # Capturing the environment with `move`
///
/// To capture outer variables by move, write `move` immediately after `color!`:
///
/// ```rust,ignore
/// color! {move
///     #![colorless(color = Async)]
///
///     let val = captured_var.resolve;
///     val
/// }
/// ```
///
/// Without `move`, variables are captured by reference (the default closure
/// capture mode).
///
// move still not working as intended
/// # Restrictions
///
/// - `color! { … }` does **not** accept a named generic color (`color!(C) { … }`)
///   because there is no declaration site for `C` to come from. Use
///   `#[colorless(C)]` on a function instead.
/// - Only one `#![colorless(…)]` inner attribute is permitted per `color!` block.
/// - The return type is inferred from the block; there is no explicit `-> T`
///   syntax inside `color!`.
///
/// # Relationship to `#[colorless]`
///
/// `color! { … }` is equivalent to immediately invoking an anonymous
/// `#[colorless]` function with no arguments:
///
/// ```rust,ignore
/// // These two are equivalent:
/// let a = color! { some_val.resolve }.resolve;
///
/// #[colorless]
/// fn __anon() -> TheType { some_val.resolve }
/// let a = __anon().resolve;
/// ```
///
/// Use `color!` for ad-hoc inline blocks; use `#[colorless]` when the
/// computation needs a name, documentation, or multiple call sites.
///
/// # Cross-references
///
/// - [`colorless`](macro@colorless) — the item-level attribute form
/// - [`colorless::Colored`] — the trait implemented by the value this macro produces
/// - [`colorless::IntoColored`] — the protocol for passing colored values into functions
///
///  [`colorless::Colored`]: https://docs.rs/colorless-core/latest/colorless_core/trait.Colored.html
///  [`colorless::IntoColored`]: https://docs.rs/colorless-core/latest/colorless_core/trait.IntoColored.html
#[proc_macro]
pub fn color(input: TokenStream) -> TokenStream {
    match expr::expand(parse_macro_input!(input)) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
