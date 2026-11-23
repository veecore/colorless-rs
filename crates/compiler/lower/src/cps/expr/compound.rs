//! CPS lowering for compound expressions that hoist sub-expressions via ANF
//! (Arrays, Tuples, Calls, MethodCalls, Unary, Casts, Field expressions, etc.).

use crate::ir::{
    CpsExprArray, CpsExprAwait, CpsExprCall, CpsExprCast, CpsExprGroup, CpsExprMethodCall,
    CpsExprRange, CpsExprRawAddr, CpsExprReference, CpsExprRepeat, CpsExprStruct, CpsExprTuple,
    CpsExprUnary, CpsExprYield,
};
use crate::ir::{CpsExprAssign, CpsExprIndex, CpsExprParen};
use crate::{CpsError, derive_cps_bind, derive_cps_bind_error};
use proc_macro2::TokenStream;
use syn::{
    ExprArray, ExprAssign, ExprAwait, ExprCall, ExprCast, ExprGroup, ExprIndex, ExprLet,
    ExprMethodCall, ExprParen, ExprRange, ExprRawAddr, ExprReference, ExprRepeat, ExprStruct,
    ExprTryBlock, ExprTuple, ExprUnary, ExprYield,
};

derive_cps_bind! {
    ExprArray {
        attrs,
        bracket_token,
        (pub elems),
    } -> {
        CpsExprArray
    }
}

derive_cps_bind! {
    ExprAssign {
        attrs,
        (pub right),
        (pub left),
        eq_token,
    } -> {
        CpsExprAssign
    }
}

derive_cps_bind! {
    ExprAwait {
        attrs,
        (pub base),
        dot_token,
        await_token,
    } -> {
        CpsExprAwait
    }
}

// The order is correct... func gets resolved first
derive_cps_bind! {
    ExprCall {
        attrs,
        (pub func),
        paren_token,
        (pub args),
    } -> {
        CpsExprCall
    }
}

derive_cps_bind! {
    ExprCast {
        attrs,
        (pub expr),
        as_token,
        ty,
    } -> {
        CpsExprCast
    }
}

// derive_cps_bind_error!(ExprGroup => CpsError::SynGroup);

derive_cps_bind! {
    ExprGroup {
        attrs,
        (pub expr),
        group_token,
    } -> {
        CpsExprGroup
    }
}

derive_cps_bind! {
    ExprIndex {
        attrs,
        (pub expr),
        bracket_token,
        (pub index),
    } -> {
        CpsExprIndex
    }
}

derive_cps_bind_error!(ExprLet => CpsError::LetExpr);

// TODO: We might catch mistaken use of resolve as method here
//  I just made a compound expression hoping to have resolve but
//  rust considered it method... although one should immediately
//  know, it might just mistakenly pass
derive_cps_bind! {
    ExprMethodCall {
        attrs,
        (pub receiver),
        dot_token,
        method,
        turbofish,
        paren_token,
        (pub args),
    } -> {
        CpsExprMethodCall
    }
}

derive_cps_bind! {
    ExprParen {
        attrs,
        paren_token,
        (pub expr),
    } -> {
        CpsExprParen
    }
}

derive_cps_bind! {
    ExprRange {
        attrs,
        (pub start),
        limits,
        (pub end),
    } -> {
        CpsExprRange
    }
}

derive_cps_bind! {
    ExprRawAddr {
        attrs,
        and_token,
        raw,
        mutability,
        (pub expr),
    } -> {
        CpsExprRawAddr
    }
}

derive_cps_bind! {
    ExprReference {
        attrs,
        and_token,
        mutability,
        (pub expr),
    } -> {
        CpsExprReference
    }
}

derive_cps_bind! {
    ExprRepeat {
        attrs,
        bracket_token,
        (pub expr),
        semi_token,
        (pub len),
    } -> {
        CpsExprRepeat
    }
}

derive_cps_bind! {
    ExprStruct {
        attrs,
        qself,
        path,
        brace_token,
        (pub fields),
        dot2_token,
        (pub rest),
    } -> {
        CpsExprStruct
    }
}

derive_cps_bind_error!(ExprTryBlock => CpsError::TryBlock);

derive_cps_bind! {
    ExprTuple {
        attrs,
        paren_token,
        (pub elems),
    } -> {
        CpsExprTuple
    }
}

derive_cps_bind! {
    ExprUnary { attrs, op, (pub expr), } -> {
        CpsExprUnary
    }
}

derive_cps_bind! {
    ExprYield {
        attrs,
        yield_token,
        (pub expr),
    } -> {
        CpsExprYield
    }
}

derive_cps_bind_error!(TokenStream => CpsError::Verbatim);

mod macros {
    /// Implements [`CpsStep`](crate::cps::CpsStep) and [`CpsBind`](crate::cps::expr::CpsBind)
    /// for compound expressions by:
    /// 1. Destructuring the `syn` AST node.
    /// 2. Hoisting all sub-expressions marked with `(pub <field>)` into a [`Hoister`](crate::cps::anf::Hoister).
    /// 3. Reassembling the node using the generated temporary identifiers (`__tmp_e_N`).
    /// 4. Wrapping the compound statement with the accumulated bindings in left-to-right evaluation order.
    // We use `pub` inside parentheses `(pub field)` because it avoids macro syntax collisions
    // and is highlighted cleanly in IDEs.
    #[macro_export]
    macro_rules! derive_cps_bind {
        (
            $syn_ty:ident {
                $attrs:ident,
                $(
                    $( (pub $expand:ident) )?
                    $( $plain:ident )?,
                )*
            } -> {
                $cps_ty:ident
            }
        ) => {
            impl $crate::cps::CpsStep for $syn_ty {
                #[inline]
                fn then(
                    self,
                    continuation: $crate::cps::CpsContinuation,
                    ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    let $syn_ty {
                        $attrs,
                        $( $( $expand, )?
                        $( $plain, )? )*
                    } = self;
                    $crate::error::assert_no_attrs($attrs)?;

                    let mut hoister = $crate::cps::anf::Hoister::new();
                    $(
                        $(
                            let $expand
                                = $crate::cps::anf::HoistTarget::hoist_into($expand, &mut hoister, ctx)?;
                        )?
                        $(
                            let $plain = $plain;
                        )?
                    )*

                    let cps = $cps_ty {
                        $(
                            $( $expand )?
                            $( $plain )?,
                        )*
                    };

                    let continuation: $crate::cps::CpsContinuation = $crate::cps::expr::prepend_expr_stmt(cps, continuation);
                    hoister.finish(continuation, ctx)
                }
            }

            impl $crate::cps::expr::CpsBind for $syn_ty {
                #[inline]
                fn and_then(
                    self,
                    bind: syn::Ident,
                    continuation: $crate::cps::CpsContinuation,
                    ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    let $syn_ty {
                        $attrs,
                        $( $( $expand, )?
                        $( $plain, )? )*
                    } = self;
                    $crate::error::assert_no_attrs($attrs)?;

                    let mut hoister = $crate::cps::anf::Hoister::new();
                    $(
                        $(
                            let $expand
                                = $crate::cps::anf::HoistTarget::hoist_into($expand, &mut hoister, ctx)?;
                        )?
                        $(
                            let $plain = $plain;
                        )?
                    )*

                    let cps = $cps_ty {
                        $(
                            $( $expand )?
                            $( $plain )?,
                        )*
                    };

                    let continuation: $crate::cps::CpsContinuation
                        = $crate::cps::expr::bind_expr_and_prepend_stmt(bind, cps, continuation);
                    hoister.finish(continuation, ctx)
                }
            }
        };
    }

    #[macro_export]
    macro_rules! derive_cps_bind_error {
        ($ty:ty => $err:path) => {
            impl $crate::cps::CpsStep for $ty {
                fn then(
                    self,
                    _continuation: $crate::cps::CpsContinuation,
                    _ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    Err($err(self).into())
                }
            }

            impl $crate::cps::CpsBind for $ty {
                fn and_then(
                    self,
                    _bind: syn::Ident,
                    _continuation: $crate::cps::CpsContinuation,
                    _ctx: &mut $crate::cps::CpsCtx,
                ) -> $crate::Result<$crate::cps::CpsContinuation> {
                    Err($err(self).into())
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cps::CpsStep;

    #[test]
    fn verbatim_expression_rejected() {
        use proc_macro2::TokenStream;

        let tokens: TokenStream = quote::quote!(some random syntax);
        let err = tokens.step().unwrap_err();
        assert!(matches!(*err, CpsError::Verbatim(..)));
    }

    #[test]
    fn attribute_rejection_on_call_expr() {
        use syn::{ExprCall, parse_quote};

        let expr: ExprCall = parse_quote!(
            #[cold]
            foo(1)
        );
        let err = expr.step().unwrap_err();
        assert!(matches!(*err, CpsError::UnallowedAttributes(..)));
    }
}
