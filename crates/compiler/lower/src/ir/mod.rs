//! Intermediate Representation (IR) produced by CPS lowering.
//!
//! Represents control-flow structures without labels or direct jump statements,
//! encoding execution as sequential statements followed by an explicit terminator.
//!
//! Submodules:
//! - [`body`]: Block and body structures ([`CpsBlock`], [`CpsBody`]).
//! - [`stmt`]: Statement nodes ([`CpsStmt`], [`CpsStmtLocal`], etc.).
//! - [`terminator`]: Control-flow terminators ([`CpsTerminator`], branches, signals).
//! - [`expr`]: Inert expressions and operands.

pub mod body;
pub mod expr;
pub mod scoped;
pub mod stmt;
pub mod terminator;

pub use body::*;
pub use expr::*;
pub use stmt::*;
pub use terminator::*;

// TODO: HAVE CUSTOM PAT

use crate::Result;

/// Opaque identity of one structural continuation owner.
///
/// The numeric value supplies uniqueness only. Ordering and ancestry come
/// from the continuation path, never from comparing IDs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CpsScopeId(pub(crate) usize);

/// Conversion from `syn` AST nodes into CPS IR.
pub trait IntoCps {
    type Output;

    fn into_cps(self) -> Result<Self::Output>;
}

/// Conversion from CPS IR back into `syn` AST nodes.
pub trait IntoSyn<Target> {
    fn into_syn(self) -> Target;
}

mod impls {
    use crate::ir::IntoSyn;
    use syn::punctuated::{Pair, Punctuated};

    impl<Target, T: IntoSyn<Target>, P: Default> IntoSyn<Punctuated<Target, P>> for Punctuated<T, P> {
        #[inline]
        fn into_syn(self) -> Punctuated<Target, P> {
            self.into_pairs()
                .map(|elem| {
                    let (v, p) = elem.into_tuple();
                    let r = v.into_syn();
                    Pair::new(r, p)
                })
                .collect()
        }
    }

    impl<Target, T: IntoSyn<Target>> IntoSyn<Vec<Target>> for Vec<T> {
        #[inline]
        fn into_syn(self) -> Vec<Target> {
            self.into_iter().map(IntoSyn::into_syn).collect()
        }
    }

    impl<Target, T: IntoSyn<Target>> IntoSyn<Box<Target>> for Box<T> {
        #[inline]
        fn into_syn(self) -> Box<Target> {
            (*self).into_syn().into()
        }
    }

    impl<Target, T: IntoSyn<Target>> IntoSyn<Option<Target>> for Option<T> {
        #[inline]
        fn into_syn(self) -> Option<Target> {
            match self {
                None => None,
                Some(s) => s.into_syn().into(),
            }
        }
    }
}

mod macros {
    // for plain cps types
    #[macro_export]
    macro_rules! derive_into_syn {
        (
            $syn_ty:ident {
                $(
                    $(_ $missing:ident)?
                    $( (pub $expand:ident) )?
                    $( $plain:ident )?,
                )*
            } -> {$cps_ty:ident}
        ) => {
            impl $crate::ir::IntoSyn<$syn_ty> for $cps_ty {
                fn into_syn(self) -> $syn_ty {
                    let $cps_ty {
                        $(
                          $( $expand, )?
                          $( $plain, )?
                        )*
                        ..
                    } = self;

                    $syn_ty {
                        $(
                            $($missing: Default::default())?
                            $( $expand: $crate::ir::IntoSyn::into_syn($expand) )?
                            $( $plain )?
                        ),*
                    }
                }
            }

            impl From<$cps_ty> for $syn_ty {
                #[inline]
                fn from(value: $cps_ty) -> Self {
                    $crate::ir::IntoSyn::into_syn(value)
                }
            }
        };
        // no expand
        (
            $syn_ty:ident {
                _ $attrs:ident,
                $(
                    $plain:ident,
                )*
            } <-> {$cps_ty:ident}
        ) => {
            derive_into_syn! {
                $syn_ty {
                    _ $attrs,
                    $(
                        $plain,
                    )*
                } -> {$cps_ty}
            }

            impl $crate::ir::IntoCps for $syn_ty {
                type Output = $cps_ty;

                #[inline]
                fn into_cps(self) -> $crate::Result<Self::Output> {
                    let $syn_ty {
                        $attrs,
                        $(
                          $plain,
                        )*
                    } = self;

                    $crate::error::assert_no_attrs($attrs)?;
                    Ok($cps_ty {
                        $(
                          $plain,
                        )*
                    })
                }
            }
            impl TryFrom<$syn_ty> for $cps_ty {
                type Error = Box<$crate::error::CpsError>;

                #[inline]
                fn try_from(value: $syn_ty) -> Result<Self, Self::Error> {
                    $crate::ir::IntoCps::into_cps(value)
                }
            }
        }
    }

    /// A `pub` payload is transparent and delegates directly to its own `IntoSyn` output,
    /// as in `Inert(pub CpsInert)`.
    #[macro_export]
    macro_rules! ir_enum {
        (
            $(#[$enum_attr:meta])*
            pub enum $name:ident {
                $(
                    $(#[$variant_attr:meta])*
                    $variant:ident($($field:tt)+),
                )*
            }
        ) => {
            $(#[$enum_attr])*
            pub enum $name {
                $(
                    $(#[$variant_attr])*
                    $variant(ir_enum!(@field_type $($field)+))
                ),+
            }

            $(
                impl From<ir_enum!(@field_type $($field)+)> for $name {
                    #[inline]
                    fn from(expr: ir_enum!(@field_type $($field)+)) -> Self {
                        Self::$variant(expr)
                    }
                }
            )*

            #[allow(non_snake_case, unused_macros)]
            macro_rules! $name {
                ($expr:expr => $method:ident()) => {
                    match $expr {
                        $(
                            $name::$variant(v) => v.$method(),
                        )+
                    }
                };
                ($expr:expr => $method:ident($arg:ident)) => {
                    match $expr {
                        $(
                            $name::$variant(v) => v.$method($arg),
                        )+
                    }
                };
                ($expr:expr => $target_enum:ty => $method:ident()) => {
                    match $expr {
                        $(
                            $name::$variant(v) => $name!(
                                @map [$target_enum] [$variant] [$($field)+] [v $method]
                            ),
                        )+
                    }
                };
                (@map [$target_enum:ty] [$variant_:ident] [pub $field_:ty] [$f:ident $method:ident]) => {
                    $f.$method()
                };
                (@map [$target_enum:ty] [$variant_:ident] [$field_:ty] [$f:ident $method:ident]) => {
                    <$target_enum>::$variant_($f.$method())
                };
            }
        };
        (@field_type pub $field:ty) => { $field };
        (@field_type $field:ty) => { $field };
    }
}
